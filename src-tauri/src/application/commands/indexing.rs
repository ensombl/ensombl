//! Commands that build and maintain the architecture index of a worktree.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::cqrs::{Command, CommandHandler};
use crate::application::index_registry::{IndexRegistry, WorktreeIndex};
use crate::application::ports::{
    FileWatcher, IndexEvents, ModuleResolver, ModuleResolverFactory, ParseCache, SessionRepository,
    SourceParser, SourceScanner,
};
use crate::application::{Error, Result};
use crate::domain::linker::Linker;
use crate::domain::model::{ArchModel, SymbolNode};
use crate::domain::parsed::ParsedFile;
use crate::domain::workspace::WorktreeId;

/// Indexes an open worktree tab and starts watching it. A no-op when it's already indexed, unless `force`.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IndexWorktree {
    pub worktree_id: WorktreeId,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IndexSummary {
    pub files: u32,
    pub symbols: u32,
    pub edges: u32,
    /// Files that had to be parsed (not served from cache).
    pub parsed: u32,
    pub millis: u32,
}

/// Re-parses changed paths (from the watcher) and relinks the model.
#[derive(Debug)]
pub struct ReindexPaths {
    pub worktree_id: WorktreeId,
    pub paths: Vec<String>,
}

/// Stops watching and frees a worktree's index (its tab was closed).
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DropIndex {
    pub worktree_id: WorktreeId,
}

impl Command for IndexWorktree {
    type Output = IndexSummary;
}
impl Command for ReindexPaths {
    type Output = ();
}
impl Command for DropIndex {
    type Output = ();
}

pub struct IndexingPorts {
    pub sessions: Arc<dyn SessionRepository>,
    pub scanner: Arc<dyn SourceScanner>,
    pub parser: Arc<dyn SourceParser>,
    pub resolvers: Arc<dyn ModuleResolverFactory>,
    pub cache: Arc<dyn ParseCache>,
    pub watcher: Arc<dyn FileWatcher>,
    pub events: Arc<dyn IndexEvents>,
}

pub struct IndexingCommands {
    ports: IndexingPorts,
    registry: Arc<IndexRegistry>,
}

fn hash(source: &str) -> String {
    blake3::hash(source.as_bytes()).to_hex()[..16].to_string()
}

/// Rebuilds the linked model from parsed files.
fn build_model(
    root: &Path,
    files: &HashMap<String, ParsedFile>,
    resolver: &dyn ModuleResolver,
) -> ArchModel {
    let resolve = |from: &str, spec: &str| resolver.resolve(from, spec);
    let edges = Linker::new(files, &resolve).link();
    let mut symbols: Vec<SymbolNode> = files
        .values()
        .flat_map(|f| f.symbols.iter().cloned())
        .collect();
    symbols.sort_by(|a, b| a.id.cmp(&b.id));
    ArchModel {
        root: root.to_string_lossy().into_owned(),
        symbols,
        edges,
    }
}

impl IndexingCommands {
    pub fn new(ports: IndexingPorts, registry: Arc<IndexRegistry>) -> Self {
        Self { ports, registry }
    }

    fn worktree_root(&self, worktree_id: &str) -> Result<std::path::PathBuf> {
        let session = self.ports.sessions.get()?;
        let tab = session
            .tabs
            .iter()
            .find(|t| t.id == worktree_id)
            .ok_or_else(|| Error::Invalid(format!("worktree {worktree_id} isn't open")))?;
        Ok(tab.path.clone().into())
    }

    /// Parses one file, using the cache when the content hash matches.
    fn parse_file(
        &self,
        worktree: &str,
        root: &Path,
        path: &str,
        package: &str,
    ) -> Option<(Option<String>, ParsedFile)> {
        let source = self.ports.scanner.read(root, path).ok()?;
        let key = format!("{}:{}", self.ports.parser.version(), hash(&source));
        if let Some(hit) = self.ports.cache.get(worktree, path, &key) {
            return Some((None, hit));
        }
        let parsed = self.ports.parser.parse(path, package, &source).ok()?;
        Some((Some(key), parsed))
    }

    fn summary(index: &WorktreeIndex, parsed: usize, started: Instant) -> IndexSummary {
        IndexSummary {
            files: index.files.len() as u32,
            symbols: index.model.symbols.len() as u32,
            edges: index.model.edges.len() as u32,
            parsed: parsed as u32,
            millis: started.elapsed().as_millis() as u32,
        }
    }
}

impl CommandHandler<IndexWorktree> for IndexingCommands {
    fn handle(&self, cmd: IndexWorktree) -> Result<IndexSummary> {
        let started = Instant::now();
        let id = cmd.worktree_id.as_str();
        if !cmd.force
            && let Some(summary) = self
                .registry
                .update(id, |index| Self::summary(index, 0, started))
        {
            return Ok(summary);
        }

        let root = self.worktree_root(id)?;
        let sources = self.ports.scanner.scan(&root)?;
        let total = sources.len();
        let done = AtomicUsize::new(0);
        self.ports.events.progress(id, 0, total);

        let results: Vec<_> = sources
            .par_iter()
            .filter_map(|source| {
                let result = self.parse_file(id, &root, &source.path, &source.package);
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n.is_multiple_of(250) {
                    self.ports.events.progress(id, n, total);
                }
                result
            })
            .collect();

        let mut fresh = Vec::new();
        let mut files = HashMap::with_capacity(results.len());
        for (key, parsed) in results {
            if let Some(key) = key {
                fresh.push((key, parsed.clone()));
            }
            files.insert(parsed.path.clone(), parsed);
        }
        self.ports.cache.put_many(id, &fresh);

        let resolver = self.ports.resolvers.for_root(&root);
        let model = build_model(&root, &files, resolver.as_ref());
        let index = WorktreeIndex {
            root: root.clone(),
            files,
            resolver,
            model,
        };
        let summary = Self::summary(&index, fresh.len(), started);
        self.registry.insert(id, index);

        self.ports.events.progress(id, total, total);
        self.ports.events.updated(id, &[], &[]);
        self.ports.watcher.watch(id, &root)?;
        Ok(summary)
    }
}

impl CommandHandler<ReindexPaths> for IndexingCommands {
    fn handle(&self, cmd: ReindexPaths) -> Result<()> {
        let id = cmd.worktree_id.as_str();
        let Some(root) = self.registry.root(id) else {
            return Ok(());
        };

        // Parse outside the registry lock; only the swap happens under it.
        let mut fresh = Vec::new();
        let mut parsed: Vec<(String, Option<ParsedFile>)> = Vec::new();
        for path in &cmd.paths {
            let exists = root.join(path).is_file() && self.ports.scanner.is_source(path);
            let file = exists
                .then(|| {
                    let package = self.ports.scanner.package_of(&root, path);
                    self.parse_file(id, &root, path, &package)
                })
                .flatten()
                .map(|(key, file)| {
                    if let Some(key) = key {
                        fresh.push((key, file.clone()));
                    }
                    file
                });
            parsed.push((path.clone(), file));
        }
        self.ports.cache.put_many(id, &fresh);
        let gone: Vec<String> = parsed
            .iter()
            .filter(|(_, f)| f.is_none())
            .map(|(p, _)| p.clone())
            .collect();
        self.ports.cache.forget(id, &gone);

        let diff = self.registry.update(id, |index| {
            let mut before: HashMap<String, SymbolNode> = HashMap::new();
            for (path, _) in &parsed {
                if let Some(old) = index.files.get(path) {
                    before.extend(old.symbols.iter().map(|s| (s.id.clone(), s.clone())));
                }
            }
            let mut changed = Vec::new();
            let mut seen = HashSet::new();
            for (path, file) in parsed {
                match file {
                    Some(file) => {
                        for s in &file.symbols {
                            seen.insert(s.id.clone());
                            if before.get(&s.id) != Some(s) {
                                changed.push(s.id.clone());
                            }
                        }
                        index.files.insert(path, file);
                    }
                    None => {
                        index.files.remove(&path);
                    }
                }
            }
            let removed: Vec<String> = before.into_keys().filter(|id| !seen.contains(id)).collect();
            index.model = build_model(&index.root, &index.files, index.resolver.as_ref());
            (changed, removed)
        });

        if let Some((changed, removed)) = diff
            && (!changed.is_empty() || !removed.is_empty())
        {
            self.ports.events.updated(id, &changed, &removed);
        }
        Ok(())
    }
}

impl CommandHandler<DropIndex> for IndexingCommands {
    fn handle(&self, cmd: DropIndex) -> Result<()> {
        self.ports.watcher.unwatch(&cmd.worktree_id);
        self.registry.remove(&cmd.worktree_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::domain::workspace::{Session, Worktree};
    use crate::infrastructure::fs_scanner::FsScanner;
    use crate::infrastructure::sqlite_cache::SqliteCache;
    use crate::infrastructure::ts_parser::TreeSitterParser;
    use crate::infrastructure::ts_resolver::TsResolverFactory;

    struct Sessions(Session);
    impl SessionRepository for Sessions {
        fn get(&self) -> Result<Session> {
            Ok(self.0.clone())
        }
        fn update(&self, _: &mut dyn FnMut(&mut Session) -> Result<()>) -> Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct NoWatch;
    impl FileWatcher for NoWatch {
        fn watch(&self, _: &str, _: &Path) -> Result<()> {
            Ok(())
        }
        fn unwatch(&self, _: &str) {}
    }

    #[derive(Default)]
    struct Events(Mutex<Vec<(Vec<String>, Vec<String>)>>);
    impl IndexEvents for Events {
        fn progress(&self, _: &str, _: usize, _: usize) {}
        fn updated(&self, _: &str, changed: &[String], removed: &[String]) {
            self.0
                .lock()
                .unwrap()
                .push((changed.to_vec(), removed.to_vec()));
        }
    }

    fn write(root: &Path, rel: &str, contents: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, contents).unwrap();
    }

    fn setup(
        root: &Path,
    ) -> (
        IndexingCommands,
        Arc<IndexRegistry>,
        Arc<Events>,
        Arc<SqliteCache>,
    ) {
        let mut session = Session::default();
        session.add_project("p".into(), "proj".into(), root.to_string_lossy().into());
        session
            .open_tab(Worktree {
                id: "wt".into(),
                project_id: "p".into(),
                path: root.to_string_lossy().into(),
                branch: Some("main".into()),
                is_main: true,
            })
            .unwrap();
        let registry = Arc::new(IndexRegistry::default());
        let events = Arc::new(Events::default());
        let cache = Arc::new(SqliteCache::in_memory().unwrap());
        let ports = IndexingPorts {
            sessions: Arc::new(Sessions(session)),
            scanner: Arc::new(FsScanner),
            parser: Arc::new(TreeSitterParser),
            resolvers: Arc::new(TsResolverFactory),
            cache: cache.clone(),
            watcher: Arc::new(NoWatch),
            events: events.clone(),
        };
        (
            IndexingCommands::new(ports, registry.clone()),
            registry,
            events,
            cache,
        )
    }

    #[test]
    fn indexes_a_worktree_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "src/repo.ts",
            "export interface Repo { find(id: string): User }\nexport class User {}",
        );
        write(
            root,
            "src/service.ts",
            "import { Repo, User } from './repo';\nexport class Service { constructor(private repo: Repo) {} get(): User { return null as any } }",
        );
        let (cmds, registry, _, _) = setup(root);

        let summary = cmds
            .handle(IndexWorktree {
                worktree_id: "wt".into(),
                force: false,
            })
            .unwrap();
        assert_eq!((summary.files, summary.symbols, summary.parsed), (2, 3, 2));

        let model = registry.model("wt").unwrap();
        let edge = |from: &str, to: &str| {
            model
                .edges
                .iter()
                .find(|e| e.from.ends_with(from) && e.to.ends_with(to))
                .map(|e| e.kind)
        };
        use crate::domain::model::EdgeKind;
        assert_eq!(edge("#Service", "#Repo"), Some(EdgeKind::Association));
        assert_eq!(edge("#Service", "#User"), Some(EdgeKind::Dependency));
        assert_eq!(edge("#Repo", "#User"), Some(EdgeKind::Dependency));
    }

    #[test]
    fn reindexing_uses_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.ts", "export class A {}");
        let (cmds, _, _, _) = setup(dir.path());
        cmds.handle(IndexWorktree {
            worktree_id: "wt".into(),
            force: false,
        })
        .unwrap();
        let again = cmds
            .handle(IndexWorktree {
                worktree_id: "wt".into(),
                force: true,
            })
            .unwrap();
        assert_eq!(again.parsed, 0);
    }

    #[test]
    fn reindex_paths_reports_changed_and_removed_symbols() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "a.ts", "export class A {}\nexport class Gone {}");
        write(root, "b.ts", "export class B {}");
        let (cmds, registry, events, _) = setup(root);
        cmds.handle(IndexWorktree {
            worktree_id: "wt".into(),
            force: false,
        })
        .unwrap();

        write(
            root,
            "a.ts",
            "export class A { added(): void {} }\nexport class New {}",
        );
        std::fs::remove_file(root.join("b.ts")).unwrap();
        cmds.handle(ReindexPaths {
            worktree_id: "wt".into(),
            paths: vec!["a.ts".into(), "b.ts".into()],
        })
        .unwrap();

        let (mut changed, mut removed) = events.0.lock().unwrap().last().cloned().unwrap();
        changed.sort();
        removed.sort();
        assert_eq!(changed, ["a.ts#A", "a.ts#New"]);
        assert_eq!(removed, ["a.ts#Gone", "b.ts#B"]);
        let names: Vec<_> = registry
            .model("wt")
            .unwrap()
            .symbols
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, ["A", "New"]);
    }

    #[test]
    fn indexing_a_closed_worktree_fails() {
        let dir = tempfile::tempdir().unwrap();
        let (cmds, _, _, _) = setup(dir.path());
        assert!(
            cmds.handle(IndexWorktree {
                worktree_id: "nope".into(),
                force: false
            })
            .is_err()
        );
    }
}

/// `CRM_SMOKE_REPO=/path/to/repo cargo test --release smoke -- --ignored --nocapture`
#[cfg(test)]
mod smoke {
    use super::tests_support::setup_for;
    use super::*;

    #[test]
    #[ignore]
    fn smoke_index_real_repo() {
        let repo = std::env::var("CRM_SMOKE_REPO").expect("set CRM_SMOKE_REPO");
        let root = std::path::PathBuf::from(repo).canonicalize().unwrap();
        let (cmds, registry) = setup_for(&root);
        let summary = cmds
            .handle(IndexWorktree {
                worktree_id: "wt".into(),
                force: false,
            })
            .unwrap();
        println!("{summary:?}");
        let model = registry.model("wt").unwrap();
        let mut kinds = HashMap::new();
        for e in &model.edges {
            *kinds.entry(format!("{:?}", e.kind)).or_insert(0) += 1;
        }
        println!("edges by kind: {kinds:?}");
        let mut packages: HashMap<&str, usize> = HashMap::new();
        for s in &model.symbols {
            *packages.entry(&s.package).or_default() += 1;
        }
        let mut packages: Vec<_> = packages.into_iter().collect();
        packages.sort_by_key(|p| std::cmp::Reverse(p.1));
        println!("top packages: {:?}", &packages[..packages.len().min(8)]);
        let patterns = crate::domain::patterns::detect(&model);
        let mut by_kind: HashMap<String, usize> = HashMap::new();
        for p in &patterns {
            *by_kind.entry(format!("{:?}", p.kind)).or_default() += 1;
        }
        println!("patterns ({}): {by_kind:?}", patterns.len());
        for p in patterns.iter().take(15) {
            println!(
                "  {:.2} {} [{}]",
                p.confidence,
                p.name,
                p.evidence.join("; ")
            );
        }
    }
}

#[cfg(test)]
mod tests_support {
    use super::*;
    use crate::domain::workspace::{Session, Worktree};
    use crate::infrastructure::fs_scanner::FsScanner;
    use crate::infrastructure::sqlite_cache::SqliteCache;
    use crate::infrastructure::ts_parser::TreeSitterParser;
    use crate::infrastructure::ts_resolver::TsResolverFactory;

    struct Sessions(Session);
    impl SessionRepository for Sessions {
        fn get(&self) -> Result<Session> {
            Ok(self.0.clone())
        }
        fn update(&self, _: &mut dyn FnMut(&mut Session) -> Result<()>) -> Result<()> {
            Ok(())
        }
    }
    struct Quiet;
    impl FileWatcher for Quiet {
        fn watch(&self, _: &str, _: &Path) -> Result<()> {
            Ok(())
        }
        fn unwatch(&self, _: &str) {}
    }
    impl IndexEvents for Quiet {
        fn progress(&self, _: &str, _: usize, _: usize) {}
        fn updated(&self, _: &str, _: &[String], _: &[String]) {}
    }

    pub fn setup_for(root: &Path) -> (IndexingCommands, Arc<IndexRegistry>) {
        let mut session = Session::default();
        session.add_project("p".into(), "proj".into(), root.to_string_lossy().into());
        session
            .open_tab(Worktree {
                id: "wt".into(),
                project_id: "p".into(),
                path: root.to_string_lossy().into(),
                branch: None,
                is_main: true,
            })
            .unwrap();
        let registry = Arc::new(IndexRegistry::default());
        let ports = IndexingPorts {
            sessions: Arc::new(Sessions(session)),
            scanner: Arc::new(FsScanner),
            parser: Arc::new(TreeSitterParser),
            resolvers: Arc::new(TsResolverFactory),
            cache: Arc::new(SqliteCache::in_memory().unwrap()),
            watcher: Arc::new(Quiet),
            events: Arc::new(Quiet),
        };
        (IndexingCommands::new(ports, registry.clone()), registry)
    }
}
