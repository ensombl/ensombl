//! What a parser extracts from a single source file, before cross-file
//! linking. Pure data, serializable so it can be cached.

use serde::{Deserialize, Serialize};

use super::model::SymbolNode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportedName {
    Named(String),
    Default,
    Namespace,
}

/// `import { imported as local } from "specifier"` and friends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Import {
    pub local: String,
    pub imported: ImportedName,
    pub specifier: String,
}

/// `export { local as exported } [from "specifier"]` or `export * from "specifier"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReExport {
    /// `export * from "specifier"`
    All { specifier: String },
    /// `export { local as exported } from "specifier"`; `specifier` is `None` for local re-exports.
    Named {
        local: String,
        exported: String,
        specifier: Option<String>,
    },
}

/// Type names a symbol refers to, by relationship. Names are as written
/// in source (`Foo` or `ns.Foo`), resolved later by the linker.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolRefs {
    pub symbol_id: String,
    pub extends: Vec<String>,
    pub implements: Vec<String>,
    /// Property and parameter-property types: "has a".
    pub associations: Vec<String>,
    /// Types used in signatures: "uses a".
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedFile {
    /// Repo-relative path with `/` separators.
    pub path: String,
    pub symbols: Vec<SymbolNode>,
    pub imports: Vec<Import>,
    pub reexports: Vec<ReExport>,
    /// Local name of the `export default` declaration, if any.
    pub default_export: Option<String>,
    pub refs: Vec<SymbolRefs>,
}
