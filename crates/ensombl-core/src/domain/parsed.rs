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

/// `@Name(...)` with the arguments architecture detection cares about.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decorator {
    pub name: String,
    /// String literal arguments, in order (`@Get(':id')`).
    pub strings: Vec<String>,
    /// Bare identifier arguments (`@InjectRepository(Claim)`).
    pub idents: Vec<String>,
    /// Object literal argument: each key with the identifiers or strings under it
    /// (`@Module({ providers: [A, B] })`).
    pub fields: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecoratorTarget {
    Class,
    Member(String),
    Param { member: String, param: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecoratorSite {
    pub symbol_id: String,
    pub target: DecoratorTarget,
    pub decorator: Decorator,
}

/// A call as written: `this.repo.save(x)` has object `["this", "repo"]` and method `save`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSite {
    /// Enclosing top-level symbol; `None` at module level.
    pub symbol_id: Option<String>,
    /// Enclosing class member.
    pub member: Option<String>,
    /// Path of the callee's object; empty for a bare call like `fetch(...)`.
    pub object: Vec<String>,
    pub method: String,
    /// String (and template) literal arguments; template holes become `*`.
    pub strings: Vec<String>,
    /// Bare identifier arguments (handlers, routers, classes).
    pub idents: Vec<String>,
    pub line: u32,
}

/// `<ClaimForm />` inside a component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsxUse {
    pub symbol_id: Option<String>,
    pub name: String,
    pub line: u32,
}

/// Behavioral facts used to work out who talks to whom.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts {
    pub decorators: Vec<DecoratorSite>,
    pub calls: Vec<CallSite>,
    pub jsx: Vec<JsxUse>,
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
    #[serde(default)]
    pub facts: Facts,
}
