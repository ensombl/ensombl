//! The system view: components and how they communicate, as opposed to the
//! type-level [`ArchModel`](crate::domain::model::ArchModel).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What a component does in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Role {
    Controller,
    Gateway,
    Job,
    Router,
    Middleware,
    Page,
    UiComponent,
    Hook,
    Service,
    Repository,
    Client,
    Other,
    /// An event or queue channel.
    Channel,
    /// A system outside the repo (database, S3, an HTTP API, ...).
    External,
}

/// Architectural layer, outermost first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Layer {
    Interface,
    Application,
    Domain,
    Data,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NodeKind {
    Component,
    Channel,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SystemNode {
    /// Stable id: the symbol id, `file:<path>`, `channel:<name>` or `ext:<name>`.
    pub id: String,
    pub kind: NodeKind,
    pub name: String,
    pub role: Role,
    pub layer: Layer,
    /// Workspace package.
    pub package: String,
    /// Feature or module within the package.
    pub feature: String,
    /// Code that makes up this component.
    pub symbol_ids: Vec<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
    /// Written by the user (from `.crm/architecture.json`).
    pub description: Option<String>,
    /// Hidden by the user; kept so it can be shown again.
    #[serde(default)]
    pub hidden: bool,
    /// Ids of the detected components merged into this one.
    #[serde(default)]
    pub merged: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum InteractionKind {
    /// Calls methods on it.
    Call,
    /// Has it injected but no calls were found.
    Inject,
    /// Publishes to a channel.
    Emits,
    /// A channel delivers to a handler.
    Handles,
    Http,
    Renders,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Site {
    pub file: String,
    pub line: u32,
}

/// Everything one node does to another, aggregated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Interaction {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: InteractionKind,
    /// Methods, routes or channels involved, most used first.
    pub members: Vec<String>,
    /// Where it happens (capped).
    pub sites: Vec<Site>,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EntryKind {
    Http,
    Event,
    Queue,
    Cron,
    Ws,
    Page,
}

/// Where work starts: a route, an event handler, a job, a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EntryPoint {
    pub id: String,
    pub kind: EntryKind,
    /// `GET /claims/:id`, `claim.created`, `/dashboard`, ...
    pub label: String,
    pub node: String,
    pub symbol_id: String,
    pub member: Option<String>,
    pub site: Site,
}

/// One call from a member, kept for tracing flows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberCall {
    pub from_symbol: String,
    pub from_member: Option<String>,
    pub to_node: String,
    /// Where the callee's code is, when it's in the repo.
    pub to_symbol: Option<String>,
    pub to_member: Option<String>,
    pub label: String,
    pub kind: InteractionKind,
    pub site: Site,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SystemModel {
    pub nodes: Vec<SystemNode>,
    pub interactions: Vec<Interaction>,
    pub entry_points: Vec<EntryPoint>,
    #[serde(skip)]
    #[ts(skip)]
    pub calls: Vec<MemberCall>,
}
