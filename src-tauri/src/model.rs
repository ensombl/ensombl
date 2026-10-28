//! Core architecture model shared with the frontend. `cargo test` exports
//! these types to `src/bindings/` via ts-rs.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SymbolKind {
    Class,
    Interface,
    Enum,
    Function,
    TypeAlias,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MemberKind {
    Method,
    Property,
    Constructor,
    Getter,
    Setter,
    EnumMember,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Range {
    /// 1-based, inclusive.
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Member {
    pub name: String,
    pub kind: MemberKind,
    /// Source-level signature, e.g. `save(user: User): Promise<void>`.
    pub signature: String,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_async: bool,
    pub is_abstract: bool,
    /// Declared type for properties, return type for methods/getters.
    pub type_text: Option<String>,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SymbolNode {
    /// Stable id: `<repo-relative file>#<name>`.
    pub id: String,
    pub kind: SymbolKind,
    pub name: String,
    /// Repo-relative path using `/` separators.
    pub file: String,
    pub range: Range,
    /// Workspace package (or top-level folder) the file belongs to.
    pub package: String,
    pub exported: bool,
    pub is_abstract: bool,
    pub type_params: Option<String>,
    /// Signature for function symbols; header line for others.
    pub signature: String,
    pub members: Vec<Member>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EdgeKind {
    Extends,
    Implements,
    Association,
    Dependency,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArchModel {
    pub root: String,
    pub symbols: Vec<SymbolNode>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}
