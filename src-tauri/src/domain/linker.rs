//! Cross-file linking: turns per-file [`ParsedFile`]s into graph edges by
//! resolving type names through local declarations, imports and barrel
//! re-exports. Pure: module resolution is supplied by the caller.

use std::collections::{HashMap, HashSet};

use super::model::{Edge, EdgeKind};
use super::parsed::{ImportedName, ParsedFile, ReExport};

/// Maps `(importing file, specifier)` to the repo-relative target file.
pub trait Resolve {
    fn resolve(&self, from: &str, specifier: &str) -> Option<String>;
}

impl<F: Fn(&str, &str) -> Option<String>> Resolve for F {
    fn resolve(&self, from: &str, specifier: &str) -> Option<String> {
        self(from, specifier)
    }
}

const MAX_REEXPORT_DEPTH: usize = 8;

pub struct Linker<'a, R: Resolve> {
    files: &'a HashMap<String, ParsedFile>,
    resolver: &'a R,
}

impl<'a, R: Resolve> Linker<'a, R> {
    pub fn new(files: &'a HashMap<String, ParsedFile>, resolver: &'a R) -> Self {
        Self { files, resolver }
    }

    /// All edges between known symbols, deduplicated. Stronger relationships win:
    /// a type that is both extended and used only yields the `extends` edge.
    pub fn link(&self) -> Vec<Edge> {
        let mut seen = HashSet::new();
        let mut edges = Vec::new();
        let mut paths: Vec<_> = self.files.keys().collect();
        paths.sort();

        for path in paths {
            let file = &self.files[path];
            for refs in &file.refs {
                let groups = [
                    (EdgeKind::Extends, &refs.extends),
                    (EdgeKind::Implements, &refs.implements),
                    (EdgeKind::Association, &refs.associations),
                    (EdgeKind::Dependency, &refs.dependencies),
                ];
                let mut linked = HashSet::new();
                for (kind, names) in groups {
                    for name in names {
                        let Some(target) = self.resolve_name(file, name) else {
                            continue;
                        };
                        if target == refs.symbol_id || !linked.insert(target.clone()) {
                            continue;
                        }
                        let edge = Edge {
                            from: refs.symbol_id.clone(),
                            to: target,
                            kind,
                        };
                        if seen.insert(edge.clone()) {
                            edges.push(edge);
                        }
                    }
                }
            }
        }
        edges
    }

    /// Resolves a type name as written in `file` to a symbol id.
    pub fn resolve_name(&self, file: &ParsedFile, name: &str) -> Option<String> {
        if let Some((ns, member)) = name.split_once('.') {
            let import = file
                .imports
                .iter()
                .find(|i| i.local == ns && i.imported == ImportedName::Namespace)?;
            let target = self.resolver.resolve(&file.path, &import.specifier)?;
            return self.find_export(&target, member, 0);
        }

        if let Some(symbol) = file.symbols.iter().find(|s| s.name == name) {
            return Some(symbol.id.clone());
        }

        let import = file.imports.iter().find(|i| i.local == name)?;
        let target = self.resolver.resolve(&file.path, &import.specifier)?;
        match &import.imported {
            ImportedName::Named(imported) => self.find_export(&target, imported, 0),
            ImportedName::Default => self.find_default(&target),
            ImportedName::Namespace => None,
        }
    }

    fn find_default(&self, path: &str) -> Option<String> {
        let file = self.files.get(path)?;
        let local = file.default_export.as_deref()?;
        self.resolve_name(file, local)
    }

    fn find_export(&self, path: &str, name: &str, depth: usize) -> Option<String> {
        if depth > MAX_REEXPORT_DEPTH {
            return None;
        }
        let file = self.files.get(path)?;

        if let Some(symbol) = file.symbols.iter().find(|s| s.exported && s.name == name) {
            return Some(symbol.id.clone());
        }
        if name == "default" {
            return self.find_default(path);
        }

        for reexport in &file.reexports {
            match reexport {
                ReExport::Named {
                    local,
                    exported,
                    specifier,
                } if exported == name => {
                    return match specifier {
                        Some(spec) => {
                            let target = self.resolver.resolve(&file.path, spec)?;
                            self.find_export(&target, local, depth + 1)
                        }
                        None => self.resolve_name(file, local),
                    };
                }
                _ => {}
            }
        }
        file.reexports.iter().find_map(|reexport| match reexport {
            ReExport::All { specifier } => {
                let target = self.resolver.resolve(&file.path, specifier)?;
                self.find_export(&target, name, depth + 1)
            }
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::{Range, SymbolKind, SymbolNode};
    use crate::domain::parsed::{Import, SymbolRefs};

    fn symbol(file: &str, name: &str, exported: bool) -> SymbolNode {
        SymbolNode {
            id: format!("{file}#{name}"),
            kind: SymbolKind::Class,
            name: name.into(),
            file: file.into(),
            range: Range {
                start_line: 1,
                end_line: 1,
            },
            package: "pkg".into(),
            exported,
            is_abstract: false,
            type_params: None,
            signature: format!("class {name}"),
            members: vec![],
        }
    }

    fn import(local: &str, imported: ImportedName, specifier: &str) -> Import {
        Import {
            local: local.into(),
            imported,
            specifier: specifier.into(),
        }
    }

    /// Resolves `./x` relative to the root, ignoring the importing file.
    fn resolver(_: &str, spec: &str) -> Option<String> {
        spec.strip_prefix("./").map(|s| format!("{s}.ts"))
    }

    fn files(list: Vec<ParsedFile>) -> HashMap<String, ParsedFile> {
        list.into_iter().map(|f| (f.path.clone(), f)).collect()
    }

    #[test]
    fn links_through_imports_and_barrels() {
        let files = files(vec![
            ParsedFile {
                path: "base.ts".into(),
                symbols: vec![
                    symbol("base.ts", "Base", true),
                    symbol("base.ts", "Repo", true),
                ],
                ..Default::default()
            },
            ParsedFile {
                path: "index.ts".into(),
                reexports: vec![ReExport::All {
                    specifier: "./base".into(),
                }],
                ..Default::default()
            },
            ParsedFile {
                path: "user.ts".into(),
                symbols: vec![symbol("user.ts", "User", true)],
                imports: vec![
                    import("Base", ImportedName::Named("Base".into()), "./index"),
                    import("R", ImportedName::Named("Repo".into()), "./index"),
                ],
                refs: vec![SymbolRefs {
                    symbol_id: "user.ts#User".into(),
                    extends: vec!["Base".into()],
                    associations: vec!["R".into()],
                    dependencies: vec!["R".into(), "Unknown".into()],
                    ..Default::default()
                }],
                ..Default::default()
            },
        ]);
        let edges = Linker::new(&files, &resolver).link();
        assert_eq!(
            edges,
            vec![
                Edge {
                    from: "user.ts#User".into(),
                    to: "base.ts#Base".into(),
                    kind: EdgeKind::Extends
                },
                Edge {
                    from: "user.ts#User".into(),
                    to: "base.ts#Repo".into(),
                    kind: EdgeKind::Association
                },
            ]
        );
    }

    #[test]
    fn resolves_default_and_namespace_imports_and_named_reexports() {
        let files = files(vec![
            ParsedFile {
                path: "a.ts".into(),
                symbols: vec![symbol("a.ts", "A", false), symbol("a.ts", "B", true)],
                default_export: Some("A".into()),
                ..Default::default()
            },
            ParsedFile {
                path: "barrel.ts".into(),
                reexports: vec![ReExport::Named {
                    local: "default".into(),
                    exported: "Alpha".into(),
                    specifier: Some("./a".into()),
                }],
                ..Default::default()
            },
            ParsedFile {
                path: "c.ts".into(),
                imports: vec![
                    import("Def", ImportedName::Default, "./a"),
                    import("ns", ImportedName::Namespace, "./a"),
                    import("Alpha", ImportedName::Named("Alpha".into()), "./barrel"),
                ],
                ..Default::default()
            },
        ]);
        let linker = Linker::new(&files, &resolver);
        let c = &files["c.ts"];
        assert_eq!(linker.resolve_name(c, "Def").as_deref(), Some("a.ts#A"));
        assert_eq!(linker.resolve_name(c, "ns.B").as_deref(), Some("a.ts#B"));
        assert_eq!(linker.resolve_name(c, "Alpha").as_deref(), Some("a.ts#A"));
        assert_eq!(
            linker.resolve_name(c, "ns.A"),
            None,
            "A is not exported by name"
        );
    }

    #[test]
    fn reexport_cycles_terminate() {
        let files = files(vec![
            ParsedFile {
                path: "x.ts".into(),
                reexports: vec![ReExport::All {
                    specifier: "./y".into(),
                }],
                ..Default::default()
            },
            ParsedFile {
                path: "y.ts".into(),
                reexports: vec![ReExport::All {
                    specifier: "./x".into(),
                }],
                ..Default::default()
            },
            ParsedFile {
                path: "z.ts".into(),
                imports: vec![import("Q", ImportedName::Named("Q".into()), "./x")],
                ..Default::default()
            },
        ]);
        let linker = Linker::new(&files, &resolver);
        assert_eq!(linker.resolve_name(&files["z.ts"], "Q"), None);
    }
}
