//! Builds the [`SystemModel`] from parsed files: which code forms components,
//! and every call, injection, event, HTTP request and render between them.
//! Pure: names resolve through the [`Linker`].

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::domain::linker::{Linker, Resolve};
use crate::domain::model::{Edge, EdgeKind, MemberKind, SymbolKind, SymbolNode};
use crate::domain::parsed::{CallSite, DecoratorSite, DecoratorTarget, ParsedFile};

use super::classify::{self, NON_COMPONENT_DECORATORS};
use super::externals::{self, HTTP_VERBS};
use super::model::{
    EntryKind, EntryPoint, Interaction, InteractionKind, Layer, MemberCall, NodeKind, Role, Site,
    SystemModel, SystemNode,
};

const MAX_SITES: usize = 8;
const EMIT_METHODS: &[&str] = &["emit", "emitAsync", "publish", "dispatch"];
const LISTEN_METHODS: &[&str] = &["on", "once", "addListener", "subscribe"];
const ROUTE_DECORATORS: &[&str] = &[
    "Get", "Post", "Put", "Patch", "Delete", "All", "Options", "Head",
];
const EVENT_DECORATORS: &[&str] = &["OnEvent", "EventPattern", "MessagePattern"];

/// What an injected field points at.
#[derive(Debug, Clone)]
enum Target {
    Node {
        node: String,
        symbol: String,
    },
    External {
        name: &'static str,
        entity: Option<String>,
    },
    Http,
    Queue(String),
}

#[derive(Debug)]
struct Route {
    verb: String,
    segments: Vec<String>,
    label: String,
    node: String,
    symbol: String,
    member: Option<String>,
}

/// Labels with counts, sample sites and the total, for one node pair and kind.
type Aggregate = (BTreeMap<String, u32>, Vec<Site>, u32);

#[derive(Debug)]
struct HttpCall {
    from_symbol: String,
    from_member: Option<String>,
    verb: String,
    url: String,
    site: Site,
}

pub fn build<R: Resolve>(
    files: &HashMap<String, ParsedFile>,
    edges: &[Edge],
    linker: &Linker<R>,
) -> SystemModel {
    let mut b = Builder {
        files,
        linker,
        nodes: BTreeMap::new(),
        tentative: HashSet::new(),
        owner: HashMap::new(),
        features: HashMap::new(),
        module_dirs: vec![],
        implementers: HashMap::new(),
        calls: vec![],
        entries: vec![],
        routes: vec![],
        http: vec![],
        mounts: HashMap::new(),
    };
    for e in edges.iter().filter(|e| e.kind == EdgeKind::Implements) {
        b.implementers
            .entry(e.to.clone())
            .or_default()
            .push(e.from.clone());
    }
    let mut paths: Vec<&String> = files.keys().collect();
    paths.sort();
    for path in &paths {
        b.module_features(&files[*path]);
    }
    for path in &paths {
        b.components(&files[*path]);
    }
    for path in &paths {
        b.mounts(&files[*path]);
    }
    for path in &paths {
        b.interactions(&files[*path]);
    }
    b.match_http();
    b.finish()
}

struct Builder<'a, R: Resolve> {
    files: &'a HashMap<String, ParsedFile>,
    linker: &'a Linker<'a, R>,
    nodes: BTreeMap<String, SystemNode>,
    /// Nodes kept only if something connects to them.
    tentative: HashSet<String>,
    /// Symbol id to the node it belongs to.
    owner: HashMap<String, String>,
    /// Symbol id to its Nest module's feature.
    features: HashMap<String, String>,
    /// Folders owned by a Nest module, with its feature.
    module_dirs: Vec<(String, String)>,
    implementers: HashMap<String, Vec<String>>,
    calls: Vec<MemberCall>,
    entries: Vec<EntryPoint>,
    routes: Vec<Route>,
    http: Vec<HttpCall>,
    /// File to the path prefix it's mounted under (`app.use('/api', router)`).
    mounts: HashMap<String, String>,
}

fn decorators_of<'f>(file: &'f ParsedFile, symbol: &str) -> Vec<&'f DecoratorSite> {
    file.facts
        .decorators
        .iter()
        .filter(|d| d.symbol_id == symbol)
        .collect()
}

fn class_decorator<'f>(sites: &[&'f DecoratorSite], name: &str) -> Option<&'f DecoratorSite> {
    sites
        .iter()
        .find(|d| d.target == DecoratorTarget::Class && d.decorator.name == name)
        .copied()
}

/// `Repository<Claim> | undefined` → `Repository`.
fn base_type(text: &str) -> &str {
    let text = text.trim().trim_start_matches("readonly ").trim();
    let first = text
        .split('|')
        .map(str::trim)
        .find(|t| *t != "undefined" && *t != "null");
    let first = first.unwrap_or(text);
    first.split(['<', '[', ' ']).next().unwrap_or(first).trim()
}

fn file_node_id(path: &str) -> String {
    format!("file:{path}")
}

fn file_stem(path: &str) -> &str {
    let base = path.rsplit('/').next().unwrap_or(path);
    match base.split_once('.') {
        Some((stem, _)) if stem == "index" || stem == "route" || stem == "page" => {
            path.rsplit('/').nth(1).unwrap_or(stem)
        }
        Some((stem, _)) => stem,
        None => base,
    }
}

/// Joins route parts into `/a/:b`.
fn join_path(parts: &[&str]) -> String {
    let joined: Vec<&str> = parts
        .iter()
        .flat_map(|p| p.split('/'))
        .filter(|s| !s.is_empty())
        .collect();
    format!("/{}", joined.join("/"))
}

fn segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.starts_with(':') || s.starts_with('{') || s.starts_with('[') || s.contains('*') {
                "*".to_string()
            } else {
                s.to_string()
            }
        })
        .collect()
}

/// `https://api.x.com/v1/a?b` → (`api.x.com`, [`v1`, `a`]); a leading `${base}` hole is dropped.
fn split_url(url: &str) -> (Option<String>, Vec<String>) {
    let mut rest = url;
    let mut host = None;
    if let Some(i) = rest.find("://") {
        let after = &rest[i + 3..];
        let (h, p) = after.split_once('/').unwrap_or((after, ""));
        host = Some(h.to_string());
        rest = p;
    }
    let path = rest.split(['?', '#']).next().unwrap_or("");
    let mut segs = segments(path);
    if url.starts_with('*') && segs.first().is_some_and(|s| s == "*") {
        segs.remove(0);
    }
    (host.filter(|h| !h.contains('*')), segs)
}

fn matches(route: &[String], call: &[String]) -> bool {
    route.len() == call.len()
        && route
            .iter()
            .zip(call)
            .all(|(r, c)| r == c || r == "*" || c == "*")
}

/// Next.js `app/(group)/claims/[id]/route.ts` → `/claims/:id`.
fn next_route(file: &str, root: &str) -> Option<String> {
    let dirs: Vec<&str> = file.split('/').collect();
    let at = dirs.iter().position(|d| *d == root)?;
    let parts: Vec<String> = dirs[at + 1..dirs.len() - 1]
        .iter()
        .filter(|d| !(d.starts_with('(') && d.ends_with(')')) && !d.starts_with('@'))
        .map(|d| {
            d.strip_prefix('[')
                .and_then(|d| d.strip_suffix(']'))
                .map(|p| format!(":{}", p.trim_start_matches("...")))
                .unwrap_or_else(|| d.to_string())
        })
        .collect();
    Some(format!("/{}", parts.join("/")))
}

impl<'a, R: Resolve> Builder<'a, R> {
    fn resolve(&self, file: &ParsedFile, name: &str) -> Option<String> {
        self.linker.resolve_name(file, name)
    }

    /// The component behind a symbol; an interface with one implementation stands for it.
    fn node_of(&self, symbol: &str) -> Option<(String, String)> {
        if let Some(node) = self.owner.get(symbol) {
            return Some((node.clone(), symbol.to_string()));
        }
        match self.implementers.get(symbol).map(Vec::as_slice) {
            Some([only]) => self.owner.get(only).map(|n| (n.clone(), only.clone())),
            _ => None,
        }
    }

    fn specifier(&self, file: &ParsedFile, local: &str) -> Option<String> {
        file.imports
            .iter()
            .find(|i| i.local == local)
            .map(|i| i.specifier.clone())
    }

    fn feature_for(&self, symbol: Option<&str>, file: &str, package: &str) -> String {
        symbol
            .and_then(|s| self.features.get(s).cloned())
            .or_else(|| {
                // A module owns the folder it's declared in.
                self.module_dirs
                    .iter()
                    .filter(|(dir, _)| file.starts_with(dir.as_str()))
                    .max_by_key(|(dir, _)| dir.len())
                    .map(|(_, f)| f.clone())
            })
            .or_else(|| classify::feature_of(file))
            .unwrap_or_else(|| package.to_string())
    }

    fn add_node(&mut self, node: SystemNode, tentative: bool) {
        if tentative {
            self.tentative.insert(node.id.clone());
        }
        self.nodes.entry(node.id.clone()).or_insert(node);
    }

    fn special_node(&mut self, id: String, name: String, kind: NodeKind) -> String {
        let (role, layer, feature) = match kind {
            NodeKind::Channel => (Role::Channel, Layer::Application, "events"),
            _ => (Role::External, Layer::External, "external"),
        };
        self.nodes.entry(id.clone()).or_insert_with(|| SystemNode {
            id: id.clone(),
            kind,
            name,
            role,
            layer,
            package: String::new(),
            feature: feature.into(),
            symbol_ids: vec![],
            file: None,
            line: None,
            description: None,
        });
        id
    }

    fn external(&mut self, name: &str) -> String {
        self.special_node(format!("ext:{name}"), name.into(), NodeKind::External)
    }

    fn channel(&mut self, name: &str) -> String {
        self.special_node(format!("channel:{name}"), name.into(), NodeKind::Channel)
    }

    /// The node for module-level code and loose functions of a file.
    fn file_node(&mut self, file: &ParsedFile) -> String {
        let id = file_node_id(&file.path);
        if !self.nodes.contains_key(&id) {
            let package = file
                .symbols
                .first()
                .map(|s| s.package.clone())
                .unwrap_or_default();
            let role = classify::role_from_path(&file.path).unwrap_or(Role::Other);
            let feature = self.feature_for(None, &file.path, &package);
            self.add_node(
                SystemNode {
                    id: id.clone(),
                    kind: NodeKind::Component,
                    name: file_stem(&file.path).to_string(),
                    role,
                    layer: classify::layer_of(role),
                    package,
                    feature,
                    symbol_ids: vec![],
                    file: Some(file.path.clone()),
                    line: Some(1),
                    description: None,
                },
                true,
            );
        }
        id
    }

    fn module_features(&mut self, file: &ParsedFile) {
        for site in &file.facts.decorators {
            if site.decorator.name != "Module" || site.target != DecoratorTarget::Class {
                continue;
            }
            let Some(module) = file.symbols.iter().find(|s| s.id == site.symbol_id) else {
                continue;
            };
            let feature = classify::feature_from_module(&module.name);
            if let Some((dir, _)) = file.path.rsplit_once('/') {
                self.module_dirs.push((format!("{dir}/"), feature.clone()));
            }
            for (key, values) in &site.decorator.fields {
                if key != "providers" && key != "controllers" {
                    continue;
                }
                for value in values {
                    if let Some(id) = self.resolve(file, value) {
                        self.features.insert(id, feature.clone());
                    }
                }
            }
        }
    }

    fn components(&mut self, file: &ParsedFile) {
        let renders: HashSet<&str> = file
            .facts
            .jsx
            .iter()
            .filter_map(|j| j.symbol_id.as_deref())
            .collect();
        for symbol in &file.symbols {
            let decorators = decorators_of(file, &symbol.id);
            let names: Vec<&str> = decorators
                .iter()
                .filter(|d| d.target == DecoratorTarget::Class)
                .map(|d| d.decorator.name.as_str())
                .collect();
            let path_role = classify::role_from_path(&symbol.file);
            let (role, tentative) = match symbol.kind {
                SymbolKind::Class => {
                    if names.iter().any(|n| NON_COMPONENT_DECORATORS.contains(n)) {
                        continue;
                    }
                    let by_decorator = classify::role_from_decorators(names.iter().copied());
                    let by_name = classify::role_from_name(&symbol.name);
                    if by_decorator.is_none() && classify::is_data_shape(&symbol.name) {
                        continue;
                    }
                    // A name beats a generic `@Injectable()`.
                    let role = match (by_decorator, by_name) {
                        (Some(Role::Service), Some(n)) => Some(n),
                        (Some(d), _) => Some(d),
                        (None, n) => n.or(path_role),
                    };
                    (role.unwrap_or(Role::Other), role.is_none())
                }
                SymbolKind::Function => {
                    // Capitalized functions in JSX files are components even if they only render HTML.
                    let jsx_file = symbol.file.ends_with(".tsx") || symbol.file.ends_with(".jsx");
                    let fn_role = classify::function_role(
                        &symbol.name,
                        renders.contains(symbol.id.as_str()) || jsx_file,
                    );
                    let role = match (fn_role, path_role) {
                        (Some(Role::UiComponent), Some(Role::Page)) => Some(Role::Page),
                        (Some(r), _) => Some(r),
                        // Route handlers exported from Next.js route files.
                        (None, Some(Role::Controller))
                            if HTTP_VERBS.contains(&symbol.name.to_lowercase().as_str()) =>
                        {
                            Some(Role::Controller)
                        }
                        _ => None,
                    };
                    match role {
                        Some(r) => (r, false),
                        None => {
                            // Loose functions join their file's node.
                            let node = self.file_node(file);
                            self.owner.insert(symbol.id.clone(), node.clone());
                            if let Some(n) = self.nodes.get_mut(&node) {
                                n.symbol_ids.push(symbol.id.clone());
                            }
                            continue;
                        }
                    }
                }
                _ => continue,
            };
            let feature = self.feature_for(Some(&symbol.id), &symbol.file, &symbol.package);
            self.owner.insert(symbol.id.clone(), symbol.id.clone());
            self.add_node(
                SystemNode {
                    id: symbol.id.clone(),
                    kind: NodeKind::Component,
                    name: symbol.name.clone(),
                    role,
                    layer: classify::layer_for(role, &symbol.file),
                    package: symbol.package.clone(),
                    feature,
                    symbol_ids: vec![symbol.id.clone()],
                    file: Some(symbol.file.clone()),
                    line: Some(symbol.range.start_line),
                    description: None,
                },
                tentative,
            );
            self.entry_points(file, symbol, &decorators, role);
        }
    }

    fn entry(&mut self, kind: EntryKind, label: String, symbol: &SymbolNode, member: Option<&str>) {
        let line = member
            .and_then(|m| symbol.members.iter().find(|x| x.name == m))
            .map_or(symbol.range.start_line, |m| m.line);
        self.entries.push(EntryPoint {
            id: format!("{}:{}:{label}", symbol.id, member.unwrap_or("")),
            kind,
            label,
            node: symbol.id.clone(),
            symbol_id: symbol.id.clone(),
            member: member.map(Into::into),
            site: Site {
                file: symbol.file.clone(),
                line,
            },
        });
    }

    fn route(&mut self, verb: &str, path: String, symbol: &SymbolNode, member: Option<&str>) {
        let label = format!("{} {path}", verb.to_uppercase());
        self.routes.push(Route {
            verb: verb.to_uppercase(),
            segments: segments(&path),
            label: label.clone(),
            node: symbol.id.clone(),
            symbol: symbol.id.clone(),
            member: member.map(Into::into),
        });
        self.entry(EntryKind::Http, label, symbol, member);
    }

    fn entry_points(
        &mut self,
        file: &ParsedFile,
        symbol: &SymbolNode,
        decorators: &[&DecoratorSite],
        role: Role,
    ) {
        let prefix = class_decorator(decorators, "Controller")
            .and_then(|d| d.decorator.strings.first().cloned())
            .unwrap_or_default();
        let queue = class_decorator(decorators, "Processor")
            .and_then(|d| d.decorator.strings.first().cloned());
        let mut handled_queue = false;
        for site in decorators {
            let DecoratorTarget::Member(member) = &site.target else {
                continue;
            };
            let d = &site.decorator;
            let first = d.strings.first().cloned();
            let name = d.name.as_str();
            if ROUTE_DECORATORS.contains(&name) {
                let path = join_path(&[&prefix, first.as_deref().unwrap_or("")]);
                self.route(name, path, symbol, Some(member));
            } else if name == "Query" || name == "Mutation" || name == "Subscription" {
                let label = format!(
                    "{} {}",
                    name.to_lowercase(),
                    first.unwrap_or(member.clone())
                );
                self.entry(EntryKind::Http, label, symbol, Some(member));
            } else if EVENT_DECORATORS.contains(&name) || name == "SubscribeMessage" {
                let Some(channel) = first else { continue };
                let kind = if name == "SubscribeMessage" {
                    EntryKind::Ws
                } else {
                    EntryKind::Event
                };
                self.entry(kind, channel.clone(), symbol, Some(member));
                self.handles(&channel, symbol, Some(member));
            } else if name == "Process" {
                let job = first.unwrap_or_else(|| member.clone());
                let channel = format!("queue:{}", queue.clone().unwrap_or(job.clone()));
                self.entry(EntryKind::Queue, job, symbol, Some(member));
                self.handles(&channel, symbol, Some(member));
                handled_queue = true;
            } else if matches!(name, "Cron" | "Interval" | "Timeout") {
                let label = format!("{} {}", name.to_lowercase(), first.unwrap_or_default())
                    .trim()
                    .to_string();
                self.entry(EntryKind::Cron, label, symbol, Some(member));
            }
        }
        // BullMQ workers implement `process` instead of decorating handlers.
        if let Some(q) = queue
            && !handled_queue
        {
            self.entry(EntryKind::Queue, q.clone(), symbol, Some("process"));
            self.handles(&format!("queue:{q}"), symbol, Some("process"));
        }

        if symbol.kind != SymbolKind::Function {
            return;
        }
        let verb = symbol.name.to_lowercase();
        if role == Role::Controller && HTTP_VERBS.contains(&verb.as_str()) {
            if let Some(path) = next_route(&file.path, "app") {
                self.route(&verb, path, symbol, None);
            }
        } else if file.default_export.as_deref() == Some(symbol.name.as_str()) {
            if role == Role::Controller && file.path.contains("pages/api/") {
                let path = next_route(&file.path.replace(".ts", "/x.ts"), "pages");
                let path = path.unwrap_or_default();
                self.route("any", path, symbol, None);
            } else if role == Role::Page {
                let root = if file.path.contains("/app/") || file.path.starts_with("app/") {
                    "app"
                } else {
                    "pages"
                };
                let path = if root == "app" {
                    next_route(&file.path, "app")
                } else {
                    let stem = file_stem(&file.path);
                    next_route(&file.path, "pages").map(|p| {
                        if stem == "index" {
                            p
                        } else {
                            join_path(&[&p, stem])
                        }
                    })
                };
                if let Some(path) = path {
                    self.entry(EntryKind::Page, path, symbol, None);
                }
            }
        }
    }

    fn handles(&mut self, channel: &str, symbol: &SymbolNode, member: Option<&str>) {
        let channel_id = self.channel(channel);
        self.calls.push(MemberCall {
            from_symbol: channel_id.clone(),
            from_member: None,
            to_node: symbol.id.clone(),
            to_symbol: Some(symbol.id.clone()),
            to_member: member.map(Into::into),
            label: member.unwrap_or(&symbol.name).to_string(),
            kind: InteractionKind::Handles,
            site: Site {
                file: symbol.file.clone(),
                line: symbol.range.start_line,
            },
        });
    }

    /// `app.use('/api', router)` mounts another file's routes under a prefix.
    fn mounts(&mut self, file: &ParsedFile) {
        for call in &file.facts.calls {
            if call.method != "use" || call.object.len() != 1 {
                continue;
            }
            let (Some(prefix), Some(router)) = (call.strings.first(), call.idents.last()) else {
                continue;
            };
            if let Some(target) = self
                .specifier(file, router)
                .and_then(|spec| self.linker_resolve_file(&file.path, &spec))
            {
                self.mounts.insert(target, prefix.clone());
            }
        }
    }

    fn linker_resolve_file(&self, from: &str, specifier: &str) -> Option<String> {
        self.linker.resolve_file(from, specifier)
    }

    /// Field name to what it points at, for a class.
    fn fields(&self, file: &ParsedFile, symbol: &SymbolNode) -> HashMap<String, Target> {
        let mut out = HashMap::new();
        for member in symbol
            .members
            .iter()
            .filter(|m| m.kind == MemberKind::Property)
        {
            let Some(text) = &member.type_text else {
                continue;
            };
            let ty = base_type(text);
            let target = if let Some((node, sym)) =
                self.resolve(file, ty).and_then(|id| self.node_of(&id))
            {
                Some(Target::Node { node, symbol: sym })
            } else {
                self.specifier(file, ty).and_then(|spec| {
                    if externals::is_http_client(&spec) {
                        Some(Target::Http)
                    } else {
                        externals::system_for(&spec)
                            .map(|name| Target::External { name, entity: None })
                    }
                })
            };
            if let Some(t) = target {
                out.insert(member.name.clone(), t);
            }
        }
        for site in decorators_of(file, &symbol.id) {
            let DecoratorTarget::Param { param, .. } = &site.target else {
                continue;
            };
            let d = &site.decorator;
            match d.name.as_str() {
                "InjectRepository" | "InjectModel" | "InjectEntityManager" | "InjectDataSource" => {
                    out.insert(
                        param.clone(),
                        Target::External {
                            name: "Database",
                            entity: d.idents.first().or(d.strings.first()).cloned(),
                        },
                    );
                }
                "InjectQueue" => {
                    if let Some(q) = d.strings.first() {
                        out.insert(param.clone(), Target::Queue(q.clone()));
                    }
                }
                "InjectRedis" => {
                    out.insert(
                        param.clone(),
                        Target::External {
                            name: "Redis",
                            entity: None,
                        },
                    );
                }
                _ => {}
            }
        }
        out
    }

    fn record(
        &mut self,
        from: (&str, Option<&str>),
        to: (String, Option<String>, Option<String>),
        label: String,
        kind: InteractionKind,
        site: Site,
    ) {
        let (to_node, to_symbol, to_member) = to;
        self.calls.push(MemberCall {
            from_symbol: from.0.into(),
            from_member: from.1.map(Into::into),
            to_node,
            to_symbol,
            to_member,
            label,
            kind,
            site,
        });
    }

    fn interactions(&mut self, file: &ParsedFile) {
        let fields: HashMap<String, HashMap<String, Target>> = file
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Class && self.owner.contains_key(&s.id))
            .map(|s| (s.id.clone(), self.fields(file, s)))
            .collect();

        // Injection is an interaction even before any call is found.
        for (class, targets) in &fields {
            for (field, target) in targets {
                let (to, label) = match target {
                    Target::Node { node, symbol } => {
                        ((node.clone(), Some(symbol.clone()), None), field.clone())
                    }
                    Target::External { name, entity } => (
                        (self.external(name), None, None),
                        entity.clone().unwrap_or(field.clone()),
                    ),
                    _ => continue,
                };
                let line = file
                    .symbols
                    .iter()
                    .find(|s| &s.id == class)
                    .and_then(|s| s.members.iter().find(|m| &m.name == field))
                    .map_or(1, |m| m.line);
                let site = Site {
                    file: file.path.clone(),
                    line,
                };
                self.record((class, None), to, label, InteractionKind::Inject, site);
            }
        }

        for call in &file.facts.calls {
            self.call(file, call, &fields);
        }

        for jsx in &file.facts.jsx {
            let Some(from) = &jsx.symbol_id else { continue };
            let Some((node, symbol)) = self
                .resolve(file, jsx.name.split('.').next().unwrap_or(&jsx.name))
                .and_then(|id| self.node_of(&id))
            else {
                continue;
            };
            let site = Site {
                file: file.path.clone(),
                line: jsx.line,
            };
            let label = jsx.name.clone();
            self.record(
                (from, None),
                (node, Some(symbol), None),
                label,
                InteractionKind::Renders,
                site,
            );
        }
    }

    fn call(
        &mut self,
        file: &ParsedFile,
        call: &CallSite,
        fields: &HashMap<String, HashMap<String, Target>>,
    ) {
        let from_symbol = match &call.symbol_id {
            Some(id) => id.clone(),
            None => self.file_node(file),
        };
        let from_member = call.member.as_deref();
        let site = Site {
            file: file.path.clone(),
            line: call.line,
        };
        let from = (from_symbol.as_str(), from_member);
        let first_string = call.strings.first().cloned();

        // Events: `emitter.emit('claim.created')`, `bus.on('claim.created', handler)`.
        if let Some(channel) = &first_string {
            if EMIT_METHODS.contains(&call.method.as_str()) {
                let id = self.channel(channel);
                let to = (id.clone(), Some(id), None);
                self.record(from, to, channel.clone(), InteractionKind::Emits, site);
                return;
            }
            if LISTEN_METHODS.contains(&call.method.as_str()) && !call.object.is_empty() {
                let handler = call
                    .idents
                    .last()
                    .and_then(|h| self.resolve(file, h))
                    .unwrap_or(from_symbol.clone());
                let to_node = self
                    .owner
                    .get(&handler)
                    .cloned()
                    .unwrap_or(from_symbol.clone());
                let id = self.channel(channel);
                self.calls.push(MemberCall {
                    from_symbol: id,
                    from_member: None,
                    to_node,
                    to_symbol: Some(handler),
                    to_member: None,
                    label: channel.clone(),
                    kind: InteractionKind::Handles,
                    site,
                });
                return;
            }
        }

        match call.object.first().map(String::as_str) {
            // `this.validate()`: a step inside the same component.
            Some("this") if call.object.len() == 1 => {
                if let Some(owner) = self.owner.get(&from_symbol).cloned() {
                    let to = (owner, Some(from_symbol.clone()), Some(call.method.clone()));
                    self.record(from, to, call.method.clone(), InteractionKind::Call, site);
                }
            }
            Some("this") => {
                let field = &call.object[1];
                let rest = &call.object[2..];
                let Some(target) = fields.get(&from_symbol).and_then(|f| f.get(field)).cloned()
                else {
                    return;
                };
                let label = rest
                    .iter()
                    .map(String::as_str)
                    .chain([call.method.as_str()])
                    .collect::<Vec<_>>()
                    .join(".");
                match target {
                    Target::Node { node, symbol } => {
                        let member = rest.is_empty().then(|| call.method.clone());
                        let to = (node, Some(symbol), member);
                        self.record(from, to, label, InteractionKind::Call, site);
                    }
                    Target::External { name, entity } => {
                        let label = match entity {
                            Some(e) => format!("{e}.{label}"),
                            None => label,
                        };
                        let to = (self.external(name), None, None);
                        self.record(from, to, label, InteractionKind::Call, site);
                    }
                    Target::Http => self.http_call(from, &call.method, first_string, site),
                    Target::Queue(q) => {
                        let id = self.channel(&format!("queue:{q}"));
                        let to = (id.clone(), Some(id), None);
                        let label = first_string.unwrap_or(label);
                        self.record(from, to, label, InteractionKind::Emits, site);
                    }
                }
            }
            Some(root) => {
                let rest = &call.object[1..];
                // Express: `router.get('/claims', handler)`.
                let is_route = (HTTP_VERBS.contains(&call.method.as_str()) || call.method == "all")
                    && rest.is_empty()
                    && first_string.as_deref().is_some_and(|s| s.starts_with('/'));
                let spec = self.specifier(file, root);
                if is_route && !spec.as_deref().is_some_and(externals::is_http_client) {
                    self.express_route(file, call, &from_symbol);
                    return;
                }
                if let Some(spec) = &spec {
                    if externals::is_http_client(spec) {
                        return self.http_call(from, &call.method, first_string, site);
                    }
                    if let Some(name) = externals::system_for(spec) {
                        let label = rest
                            .iter()
                            .map(String::as_str)
                            .chain([call.method.as_str()])
                            .collect::<Vec<_>>()
                            .join(".");
                        let to = (self.external(name), None, None);
                        return self.record(from, to, label, InteractionKind::Call, site);
                    }
                }
                if let Some((node, symbol)) =
                    self.resolve(file, root).and_then(|id| self.node_of(&id))
                {
                    let member = rest.is_empty().then(|| call.method.clone());
                    let to = (node, Some(symbol), member);
                    self.record(from, to, call.method.clone(), InteractionKind::Call, site);
                }
            }
            None => {
                if call.method == "fetch" {
                    return self.http_call(from, "get", first_string, site);
                }
                if let Some((node, symbol)) = self
                    .resolve(file, &call.method)
                    .and_then(|id| self.node_of(&id).map(|(n, _)| (n, id)))
                {
                    let to = (node, Some(symbol), None);
                    self.record(from, to, call.method.clone(), InteractionKind::Call, site);
                }
            }
        }
    }

    fn http_call(
        &mut self,
        from: (&str, Option<&str>),
        method: &str,
        url: Option<String>,
        site: Site,
    ) {
        let verb = if HTTP_VERBS.contains(&method) {
            method.to_uppercase()
        } else {
            "REQUEST".to_string()
        };
        self.http.push(HttpCall {
            from_symbol: from.0.into(),
            from_member: from.1.map(Into::into),
            verb,
            url: url.unwrap_or_default(),
            site,
        });
    }

    fn express_route(&mut self, file: &ParsedFile, call: &CallSite, from_symbol: &str) {
        let path = call.strings[0].clone();
        let prefix = self.mounts.get(&file.path).cloned().unwrap_or_default();
        let full = join_path(&[&prefix, &path]);
        // The handler's own code, when it's a named function.
        let handler = call
            .idents
            .last()
            .and_then(|h| self.resolve(file, h))
            .and_then(|id| {
                file.symbols
                    .iter()
                    .find(|s| s.id == id)
                    .cloned()
                    .or_else(|| {
                        self.files
                            .values()
                            .flat_map(|f| f.symbols.iter())
                            .find(|s| s.id == id)
                            .cloned()
                    })
            });
        let node = self.file_node(file);
        if let Some(n) = self.nodes.get_mut(&node)
            && n.role == Role::Other
        {
            n.role = Role::Router;
            n.layer = Layer::Interface;
        }
        self.tentative.remove(&node);
        let label = format!("{} {full}", call.method.to_uppercase());
        let (symbol, member) = match &handler {
            Some(h) => (h.id.clone(), None),
            None => (from_symbol.to_string(), None),
        };
        self.routes.push(Route {
            verb: call.method.to_uppercase(),
            segments: segments(&full),
            label: label.clone(),
            node: node.clone(),
            symbol: symbol.clone(),
            member,
        });
        self.entries.push(EntryPoint {
            id: format!("{node}:{label}"),
            kind: EntryKind::Http,
            label,
            node,
            symbol_id: symbol,
            member: None,
            site: Site {
                file: file.path.clone(),
                line: call.line,
            },
        });
    }

    /// HTTP calls go to a matching route in the repo, or to an outside API.
    fn match_http(&mut self) {
        let calls = std::mem::take(&mut self.http);
        for call in calls {
            let (host, segs) = split_url(&call.url);
            let route = (host.is_none() && !segs.is_empty())
                .then(|| {
                    let candidates: Vec<&Route> = self
                        .routes
                        .iter()
                        .filter(|r| {
                            matches(&r.segments, &segs)
                                // Unknown global prefixes like `/api` or `/v1`.
                                || (segs.len() > r.segments.len()
                                    && segs.len() - r.segments.len() <= 2
                                    && !r.segments.is_empty()
                                    && matches(&r.segments, &segs[segs.len() - r.segments.len()..]))
                        })
                        .collect();
                    candidates
                        .iter()
                        .find(|r| r.verb == call.verb || r.verb == "ANY" || r.verb == "ALL")
                        .or(candidates.first())
                        .copied()
                })
                .flatten();
            let from = (call.from_symbol.as_str(), call.from_member.as_deref());
            match route {
                Some(r) => {
                    let to = (r.node.clone(), Some(r.symbol.clone()), r.member.clone());
                    let label = r.label.clone();
                    self.record(from, to, label, InteractionKind::Http, call.site);
                }
                None => {
                    let name = host.unwrap_or_else(|| "HTTP API".to_string());
                    let to = (self.external(&name), None, None);
                    let path = if call.url.is_empty() {
                        "…".into()
                    } else {
                        call.url.clone()
                    };
                    let label = format!("{} {path}", call.verb);
                    self.record(from, to, label, InteractionKind::Http, call.site);
                }
            }
        }
    }

    fn finish(mut self) -> SystemModel {
        // Aggregate member calls into node-to-node interactions.
        let mut grouped: BTreeMap<(String, String, InteractionKind), Aggregate> = BTreeMap::new();
        for c in &self.calls {
            let from_node = self
                .owner
                .get(&c.from_symbol)
                .cloned()
                .unwrap_or_else(|| c.from_symbol.clone());
            if from_node == c.to_node
                || !self.nodes.contains_key(&from_node)
                || !self.nodes.contains_key(&c.to_node)
            {
                continue;
            }
            let entry = grouped
                .entry((from_node, c.to_node.clone(), c.kind))
                .or_default();
            *entry.0.entry(c.label.clone()).or_default() += 1;
            if entry.1.len() < MAX_SITES {
                entry.1.push(c.site.clone());
            }
            entry.2 += 1;
        }
        let called: HashSet<(String, String)> = grouped
            .keys()
            .filter(|k| k.2 != InteractionKind::Inject)
            .map(|k| (k.0.clone(), k.1.clone()))
            .collect();
        let mut interactions: Vec<Interaction> = grouped
            .into_iter()
            // Injection that's also called is just the call.
            .filter(|((from, to, kind), _)| {
                *kind != InteractionKind::Inject || !called.contains(&(from.clone(), to.clone()))
            })
            .map(|((from, to, kind), (members, sites, count))| {
                let mut members: Vec<(String, u32)> = members.into_iter().collect();
                members.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                Interaction {
                    id: format!("{from}->{to}:{kind:?}"),
                    from,
                    to,
                    kind,
                    members: members.into_iter().map(|m| m.0).collect(),
                    sites,
                    count,
                }
            })
            .collect();

        let connected: HashSet<&str> = interactions
            .iter()
            .flat_map(|i| [i.from.as_str(), i.to.as_str()])
            .chain(self.entries.iter().map(|e| e.node.as_str()))
            .collect();
        let keep: HashSet<String> = self
            .nodes
            .keys()
            .filter(|id| !self.tentative.contains(*id) || connected.contains(id.as_str()))
            .cloned()
            .collect();
        interactions.retain(|i| keep.contains(&i.from) && keep.contains(&i.to));
        // A loose-function file that makes HTTP calls is a client.
        let http_callers: HashSet<String> = interactions
            .iter()
            .filter(|i| i.kind == InteractionKind::Http)
            .map(|i| i.from.clone())
            .collect();
        let mut nodes: Vec<SystemNode> = std::mem::take(&mut self.nodes)
            .into_values()
            .filter(|n| keep.contains(&n.id))
            .collect();
        for n in &mut nodes {
            if n.role == Role::Other && n.id.starts_with("file:") && http_callers.contains(&n.id) {
                n.role = Role::Client;
                n.layer = Layer::Data;
            }
        }
        self.entries.sort_by(|a, b| a.label.cmp(&b.label));
        self.entries.dedup_by(|a, b| a.id == b.id);
        SystemModel {
            nodes,
            interactions,
            entry_points: self.entries,
            calls: self.calls,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::application::ports::SourceParser;
    use crate::infrastructure::ts_parser::TreeSitterParser;

    /// Parses fixture files and builds the system with a simple relative resolver.
    pub fn system(sources: &[(&str, &str)]) -> SystemModel {
        let files: HashMap<String, ParsedFile> = sources
            .iter()
            .map(|(path, src)| {
                (
                    path.to_string(),
                    TreeSitterParser.parse(path, "app", src).unwrap(),
                )
            })
            .collect();
        let paths: Vec<String> = files.keys().cloned().collect();
        let resolve = move |from: &str, spec: &str| -> Option<String> {
            if !spec.starts_with('.') {
                return None;
            }
            let mut parts: Vec<&str> = from.split('/').collect();
            parts.pop();
            for seg in spec.split('/') {
                match seg {
                    "." => {}
                    ".." => {
                        parts.pop();
                    }
                    s => parts.push(s),
                }
            }
            let base = parts.join("/");
            ["ts", "tsx"]
                .iter()
                .map(|ext| format!("{base}.{ext}"))
                .find(|p| paths.contains(p))
        };
        let linker = Linker::new(&files, &resolve);
        let edges = linker.link();
        build(&files, &edges, &linker)
    }

    pub fn interaction<'m>(m: &'m SystemModel, from: &str, to: &str) -> &'m Interaction {
        m.interactions
            .iter()
            .find(|i| i.from.ends_with(from) && i.to.ends_with(to))
            .unwrap_or_else(|| panic!("no {from} -> {to} in {:#?}", m.interactions))
    }

    pub fn node<'m>(m: &'m SystemModel, id: &str) -> &'m SystemNode {
        m.nodes
            .iter()
            .find(|n| n.id.ends_with(id))
            .unwrap_or_else(|| panic!("no node {id} in {:#?}", m.nodes))
    }

    pub const NEST: &[(&str, &str)] = &[
        (
            "src/claims/claims.controller.ts",
            r#"
import { ClaimsService } from './claims.service';
@Controller('claims')
export class ClaimsController {
  constructor(private readonly claims: ClaimsService) {}
  @Get(':id')
  find(@Param('id') id: string) { return this.claims.find(id); }
  @Post()
  create(body: CreateClaimDto) { return this.claims.submit(body); }
}
"#,
        ),
        (
            "src/claims/claims.service.ts",
            r#"
import { ClaimsRepository } from './claims.repository';
import { HttpService } from '@nestjs/axios';
@Injectable()
export class ClaimsService {
  constructor(private repo: ClaimsRepository, private events: EventEmitter2, private http: HttpService) {}
  find(id: string) { return this.repo.findById(id); }
  submit(dto: CreateClaimDto) {
    this.validate(dto);
    this.repo.save(dto);
    this.events.emit('claim.submitted', dto);
    return this.http.post('https://api.carrier.com/v1/claims', dto);
  }
  private validate(dto: CreateClaimDto) {}
}
export class CreateClaimDto { id: string }
"#,
        ),
        (
            "src/claims/claims.repository.ts",
            r#"
import { Repository } from 'typeorm';
@Injectable()
export class ClaimsRepository {
  constructor(@InjectRepository(Claim) private orm: Repository<Claim>) {}
  findById(id: string) { return this.orm.findOne({ where: { id } }); }
  save(c: unknown) { return this.orm.save(c); }
}
"#,
        ),
        (
            "src/notifications/notifier.listener.ts",
            r#"
@Injectable()
export class NotifierListener {
  @OnEvent('claim.submitted')
  notify(payload: unknown) {}
  @Cron('0 * * * *')
  sweep() {}
}
"#,
        ),
        (
            "src/claims/claims.module.ts",
            r#"
import { ClaimsController } from './claims.controller';
import { ClaimsService } from './claims.service';
@Module({ controllers: [ClaimsController], providers: [ClaimsService] })
export class ClaimLinesModule {}
"#,
        ),
    ];

    #[test]
    fn nest_components_roles_and_features() {
        let m = system(NEST);
        assert_eq!(node(&m, "#ClaimsController").role, Role::Controller);
        assert_eq!(node(&m, "#ClaimsService").layer, Layer::Application);
        assert_eq!(node(&m, "#ClaimsRepository").role, Role::Repository);
        assert_eq!(node(&m, "#ClaimsService").feature, "claim-lines");
        assert_eq!(node(&m, "#ClaimsRepository").feature, "claim-lines");
        assert_eq!(node(&m, "#NotifierListener").feature, "notifications");
        assert!(!m.nodes.iter().any(|n| n.name == "CreateClaimDto"));
        assert!(!m.nodes.iter().any(|n| n.name == "ClaimLinesModule"));
    }

    #[test]
    fn nest_calls_events_externals_and_entry_points() {
        let m = system(NEST);
        let call = interaction(&m, "#ClaimsController", "#ClaimsService");
        assert_eq!(call.kind, InteractionKind::Call);
        assert_eq!(call.members, ["find", "submit"]);
        assert_eq!(
            interaction(&m, "#ClaimsService", "#ClaimsRepository").members,
            ["findById", "save"]
        );
        let db = interaction(&m, "#ClaimsRepository", "ext:Database");
        assert_eq!(db.members, ["Claim.findOne", "Claim.save"]);
        assert_eq!(
            interaction(&m, "#ClaimsService", "channel:claim.submitted").kind,
            InteractionKind::Emits
        );
        assert_eq!(
            interaction(&m, "channel:claim.submitted", "#NotifierListener").members,
            ["notify"]
        );
        let http = interaction(&m, "#ClaimsService", "ext:api.carrier.com");
        assert_eq!(http.members, ["POST https://api.carrier.com/v1/claims"]);

        let labels: Vec<&str> = m.entry_points.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "GET /claims/:id",
                "POST /claims",
                "claim.submitted",
                "cron 0 * * * *"
            ]
        );
    }

    #[test]
    fn express_routes_and_frontend_calls() {
        let m = system(&[
            (
                "server/app.ts",
                "import router from './claims.routes';\nconst app = express();\napp.use('/api', router);",
            ),
            (
                "server/claims.routes.ts",
                "import { listClaims } from './claims.handlers';\nconst router = express.Router();\nrouter.get('/claims', listClaims);\nexport default router;",
            ),
            (
                "server/claims.handlers.ts",
                "import { db } from './db';\nexport async function listClaims(req, res) { res.json(await prisma.claim.findMany()); }",
            ),
            (
                "web/hooks/useClaims.ts",
                "export function useClaims() { return fetch(`${base}/api/claims`); }",
            ),
            (
                "web/pages/claims.tsx",
                "import { useClaims } from '../hooks/useClaims';\nimport { ClaimTable } from '../components/ClaimTable';\nexport default function ClaimsPage() { const c = useClaims(); return <ClaimTable rows={c} /> }",
            ),
            (
                "web/components/ClaimTable.tsx",
                "export function ClaimTable({ rows }) { return <table /> }",
            ),
        ]);
        assert_eq!(node(&m, "file:server/claims.routes.ts").role, Role::Router);
        assert!(m.entry_points.iter().any(|e| e.label == "GET /api/claims"));
        assert!(
            m.entry_points
                .iter()
                .any(|e| e.label == "/claims" && e.kind == EntryKind::Page)
        );
        assert_eq!(node(&m, "#useClaims").role, Role::Hook);
        assert_eq!(node(&m, "#ClaimsPage").role, Role::Page);
        assert_eq!(
            interaction(&m, "#useClaims", "file:server/claims.routes.ts").members,
            ["GET /api/claims"]
        );
        assert_eq!(
            interaction(&m, "#ClaimsPage", "#ClaimTable").kind,
            InteractionKind::Renders
        );
        assert_eq!(
            interaction(&m, "#ClaimsPage", "#useClaims").kind,
            InteractionKind::Call
        );
    }

    #[test]
    fn urls_split_and_match_routes() {
        assert_eq!(
            split_url("*/api/claims/*?x=1"),
            (None, vec!["api".into(), "claims".into(), "*".into()])
        );
        assert_eq!(split_url("https://h.io/a").0.as_deref(), Some("h.io"));
        assert!(matches(&segments("/claims/:id"), &segments("/claims/42")));
        assert_eq!(
            base_type("readonly Repository<Claim> | undefined"),
            "Repository"
        );
        assert_eq!(
            next_route("web/app/(shop)/claims/[id]/route.ts", "app").as_deref(),
            Some("/claims/:id")
        );
    }
}
