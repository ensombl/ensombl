//! [`SourceScanner`] that walks a worktree respecting `.gitignore` and maps
//! each file to its workspace package (nearest `package.json` name).

use std::path::Path;

use ignore::WalkBuilder;

use crate::application::Result;
use crate::application::ports::{FileLister, SourceFile, SourceScanner};

const EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".crm",
    "dist",
    "build",
    "out",
    "coverage",
    ".next",
    ".turbo",
    ".cache",
    "target",
    "vendor",
];
/// Bigger files are almost always generated or minified.
const MAX_FILE_BYTES: u64 = 512 * 1024;

pub struct FsScanner;

pub fn to_rel(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    Some(
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

fn is_source_path(path: &str) -> bool {
    if path.ends_with(".d.ts") || path.ends_with(".min.js") {
        return false;
    }
    if path.split('/').any(|seg| SKIP_DIRS.contains(&seg)) {
        return false;
    }
    path.rsplit_once('.')
        .is_some_and(|(_, ext)| EXTENSIONS.contains(&ext))
}

/// Name from the nearest `package.json` at or above `dir` (within `root`).
fn package_for(root: &Path, rel_dir: &str) -> String {
    let mut dir = root.join(rel_dir);
    loop {
        if let Some(name) = read_package_name(&dir) {
            return name;
        }
        if dir == root || !dir.pop() || !dir.starts_with(root) {
            break;
        }
    }
    root.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "root".into())
}

fn read_package_name(dir: &Path) -> Option<String> {
    let bytes = std::fs::read(dir.join("package.json")).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    json.get("name")?.as_str().map(str::to_string)
}

impl FileLister for FsScanner {
    fn list(&self, root: &Path, limit: usize) -> Result<Vec<String>> {
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .require_git(false)
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != ".git" && !SKIP_DIRS.contains(&name.as_ref())
            })
            .build();
        let mut files: Vec<String> = walker
            .flatten()
            .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
            .filter_map(|e| to_rel(root, e.path()))
            .take(limit)
            .collect();
        files.sort();
        Ok(files)
    }
}

impl SourceScanner for FsScanner {
    fn scan(&self, root: &Path) -> Result<Vec<SourceFile>> {
        let walker = WalkBuilder::new(root)
            .hidden(true)
            .git_ignore(true)
            .git_exclude(true)
            .require_git(false)
            .filter_entry(|e| !SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
            .build();

        let mut packages = std::collections::HashMap::<String, String>::new();
        let mut files = Vec::new();
        for entry in walker.flatten() {
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let Some(rel) = to_rel(root, entry.path()) else {
                continue;
            };
            if !is_source_path(&rel)
                || entry
                    .metadata()
                    .map(|m| m.len() > MAX_FILE_BYTES)
                    .unwrap_or(false)
            {
                continue;
            }
            let dir = rel
                .rsplit_once('/')
                .map(|(d, _)| d.to_string())
                .unwrap_or_default();
            let package = packages
                .entry(dir.clone())
                .or_insert_with(|| package_for(root, &dir))
                .clone();
            files.push(SourceFile { path: rel, package });
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    fn package_of(&self, root: &Path, path: &str) -> String {
        let dir = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        package_for(root, dir)
    }

    fn is_source(&self, path: &str) -> bool {
        is_source_path(path)
    }

    fn read(&self, root: &Path, path: &str) -> Result<String> {
        Ok(std::fs::read_to_string(root.join(path))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, contents: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, contents).unwrap();
    }

    #[test]
    fn lists_every_file_that_isnt_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, ".gitignore", "dist/\n");
        write(root, "README.md", "");
        write(root, "src/a.ts", "");
        write(root, "dist/out.js", "");
        write(root, "node_modules/x/index.js", "");
        let files = FsScanner.list(root, 100).unwrap();
        assert_eq!(files, [".gitignore", "README.md", "src/a.ts"]);
        assert_eq!(FsScanner.list(root, 1).unwrap().len(), 1);
    }

    #[test]
    fn scans_sources_with_packages_and_ignores() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", r#"{"name":"mono"}"#);
        write(root, ".gitignore", "generated/\n");
        write(root, "packages/api/package.json", r#"{"name":"@acme/api"}"#);
        write(root, "packages/api/src/server.ts", "");
        write(root, "packages/api/src/types.d.ts", "");
        write(root, "src/app.tsx", "");
        write(root, "src/readme.md", "");
        write(root, "node_modules/x/index.js", "");
        write(root, "generated/out.ts", "");

        let files = FsScanner.scan(root).unwrap();
        assert_eq!(
            files,
            vec![
                SourceFile {
                    path: "packages/api/src/server.ts".into(),
                    package: "@acme/api".into()
                },
                SourceFile {
                    path: "src/app.tsx".into(),
                    package: "mono".into()
                },
            ]
        );
        assert_eq!(
            FsScanner.package_of(root, "packages/api/src/new.ts"),
            "@acme/api"
        );
    }

    #[test]
    fn source_path_rules() {
        assert!(is_source_path("a/b.ts"));
        assert!(!is_source_path("a/b.d.ts"));
        assert!(!is_source_path("node_modules/b.ts"));
        assert!(!is_source_path("a/b.css"));
    }
}
