//! Design-pattern detection over the architecture model. Heuristic and pure:
//! each detector looks at names, members and relations, and reports a
//! confidence plus the role every participant plays.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::model::{ArchModel, EdgeKind, Member, MemberKind, SymbolKind, SymbolNode, Visibility};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PatternKind {
    Singleton,
    Factory,
    Builder,
    Observer,
    Strategy,
    Decorator,
    Adapter,
    Repository,
    Facade,
}

impl PatternKind {
    fn slug(self) -> &'static str {
        match self {
            PatternKind::Singleton => "singleton",
            PatternKind::Factory => "factory",
            PatternKind::Builder => "builder",
            PatternKind::Observer => "observer",
            PatternKind::Strategy => "strategy",
            PatternKind::Decorator => "decorator",
            PatternKind::Adapter => "adapter",
            PatternKind::Repository => "repository",
            PatternKind::Facade => "facade",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Participant {
    pub symbol_id: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PatternInstance {
    /// Stable across reindexes: `<kind>:<anchor symbol id>`.
    pub id: String,
    pub kind: PatternKind,
    /// Short human name, e.g. `Strategy: PaymentMethod`.
    pub name: String,
    /// 0..1.
    pub confidence: f32,
    /// Why it was detected, for the UI.
    pub evidence: Vec<String>,
    pub participants: Vec<Participant>,
}

/// The user's verdict on a detected instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PatternDecision {
    Confirmed,
    Dismissed,
}

/// Instances below this confidence aren't reported.
pub const MIN_CONFIDENCE: f32 = 0.6;

struct Index<'a> {
    by_id: HashMap<&'a str, &'a SymbolNode>,
    out: HashMap<&'a str, Vec<(&'a str, EdgeKind)>>,
    implementers: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> Index<'a> {
    fn new(model: &'a ArchModel) -> Self {
        let by_id = model.symbols.iter().map(|s| (s.id.as_str(), s)).collect();
        let mut out: HashMap<&str, Vec<(&str, EdgeKind)>> = HashMap::new();
        let mut implementers: HashMap<&str, Vec<&str>> = HashMap::new();
        for e in &model.edges {
            out.entry(e.from.as_str())
                .or_default()
                .push((e.to.as_str(), e.kind));
            if matches!(e.kind, EdgeKind::Implements | EdgeKind::Extends) {
                implementers
                    .entry(e.to.as_str())
                    .or_default()
                    .push(e.from.as_str());
            }
        }
        Self {
            by_id,
            out,
            implementers,
        }
    }

    fn related(&self, id: &str, kinds: &[EdgeKind]) -> Vec<&'a str> {
        self.out
            .get(id)
            .map(|v| {
                v.iter()
                    .filter(|(_, k)| kinds.contains(k))
                    .map(|(t, _)| *t)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn is_abstraction(&self, id: &str) -> bool {
        self.by_id
            .get(id)
            .is_some_and(|s| s.kind == SymbolKind::Interface || s.is_abstract)
    }
}

fn methods(s: &SymbolNode) -> impl Iterator<Item = &Member> {
    s.members.iter().filter(|m| m.kind == MemberKind::Method)
}

fn has_method(s: &SymbolNode, names: &[&str]) -> bool {
    methods(s).any(|m| names.iter().any(|n| m.name.eq_ignore_ascii_case(n)))
}

fn starts_with_any(name: &str, prefixes: &[&str]) -> bool {
    let lower = name.to_ascii_lowercase();
    prefixes.iter().any(|p| lower.starts_with(p))
}

fn mentions(text: Option<&str>, name: &str) -> bool {
    text.is_some_and(|t| {
        t.split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|tok| tok == name)
    })
}

fn instance(
    kind: PatternKind,
    anchor: &SymbolNode,
    confidence: f32,
    evidence: Vec<String>,
    participants: Vec<Participant>,
) -> PatternInstance {
    let label = match kind {
        PatternKind::Singleton => "Singleton",
        PatternKind::Factory => "Factory",
        PatternKind::Builder => "Builder",
        PatternKind::Observer => "Observer",
        PatternKind::Strategy => "Strategy",
        PatternKind::Decorator => "Decorator",
        PatternKind::Adapter => "Adapter",
        PatternKind::Repository => "Repository",
        PatternKind::Facade => "Facade",
    };
    PatternInstance {
        id: format!("{}:{}", kind.slug(), anchor.id),
        kind,
        name: format!("{label}: {}", anchor.name),
        confidence: (confidence * 100.0).round() / 100.0,
        evidence,
        participants,
    }
}

fn role(id: &str, role: &str) -> Participant {
    Participant {
        symbol_id: id.into(),
        role: role.into(),
    }
}

fn singleton(s: &SymbolNode) -> Option<PatternInstance> {
    if s.kind != SymbolKind::Class {
        return None;
    }
    let mut evidence = Vec::new();
    let mut score: f32 = 0.0;
    let ctor_hidden = s
        .members
        .iter()
        .any(|m| m.kind == MemberKind::Constructor && m.visibility != Visibility::Public);
    let static_instance = s.members.iter().any(|m| {
        m.is_static && m.kind == MemberKind::Property && mentions(m.type_text.as_deref(), &s.name)
    });
    // A static method merely returning its own type is a named constructor, not an accessor.
    let accessor = s.members.iter().any(|m| {
        m.is_static
            && matches!(m.kind, MemberKind::Method | MemberKind::Getter)
            && ["getinstance", "instance", "shared", "default", "get"]
                .contains(&m.name.to_ascii_lowercase().as_str())
    });
    if ctor_hidden {
        score += 0.45;
        evidence.push("non-public constructor".into());
    }
    if static_instance {
        score += 0.3;
        evidence.push("static field of its own type".into());
    }
    if accessor {
        score += 0.3;
        evidence.push("static accessor returning the instance".into());
    }
    (score >= MIN_CONFIDENCE && (static_instance || accessor)).then(|| {
        instance(
            PatternKind::Singleton,
            s,
            score.min(0.98),
            evidence,
            vec![role(&s.id, "Singleton")],
        )
    })
}

fn factory(idx: &Index, s: &SymbolNode) -> Option<PatternInstance> {
    if !matches!(s.kind, SymbolKind::Class | SymbolKind::Interface) {
        return None;
    }
    let named = s.name.ends_with("Factory");
    let creators: Vec<&Member> = methods(s)
        .filter(|m| {
            starts_with_any(&m.name, &["create", "make", "new", "build"]) && m.name != "build"
        })
        .collect();
    if !named && creators.is_empty() {
        return None;
    }
    // Products: abstractions the factory depends on through creator return types.
    let mut products: Vec<&str> = idx
        .related(&s.id, &[EdgeKind::Dependency, EdgeKind::Association])
        .into_iter()
        .filter(|t| {
            let Some(target) = idx.by_id.get(t) else {
                return false;
            };
            creators
                .iter()
                .any(|m| mentions(m.type_text.as_deref(), &target.name))
        })
        .collect();
    products.sort();
    products.dedup();
    let abstract_products = products.iter().filter(|p| idx.is_abstraction(p)).count();

    let mut score: f32 = 0.0;
    let mut evidence = Vec::new();
    if named {
        score += 0.5;
        evidence.push("named *Factory".into());
    }
    if !creators.is_empty() {
        score += 0.25;
        evidence.push(format!("{} creator method(s)", creators.len()));
    }
    if abstract_products > 0 {
        score += 0.25;
        evidence.push("returns abstractions".into());
    } else if !products.is_empty() {
        score += 0.1;
    }
    if score < MIN_CONFIDENCE {
        return None;
    }
    let mut participants = vec![role(&s.id, "Factory")];
    participants.extend(products.iter().map(|p| role(p, "Product")));
    Some(instance(
        PatternKind::Factory,
        s,
        score.min(0.95),
        evidence,
        participants,
    ))
}

fn builder(idx: &Index, s: &SymbolNode) -> Option<PatternInstance> {
    if s.kind != SymbolKind::Class {
        return None;
    }
    let build = methods(s).find(|m| m.name == "build")?;
    let fluent = methods(s)
        .filter(|m| m.name != "build")
        .filter(|m| {
            m.type_text.as_deref() == Some("this")
                || m.type_text.as_deref() == Some(s.name.as_str())
        })
        .count();
    if fluent < 2 {
        return None;
    }
    let mut participants = vec![role(&s.id, "Builder")];
    let product = idx
        .related(&s.id, &[EdgeKind::Dependency, EdgeKind::Association])
        .into_iter()
        .find(|t| {
            idx.by_id
                .get(t)
                .is_some_and(|p| mentions(build.type_text.as_deref(), &p.name))
        });
    if let Some(p) = product {
        participants.push(role(p, "Product"));
    }
    let score = (0.6 + 0.08 * fluent as f32).min(0.95);
    Some(instance(
        PatternKind::Builder,
        s,
        score,
        vec![format!("{fluent} fluent setters"), "build() method".into()],
        participants,
    ))
}

fn observer(s: &SymbolNode) -> Option<PatternInstance> {
    if !matches!(s.kind, SymbolKind::Class | SymbolKind::Interface) {
        return None;
    }
    let subscribe = has_method(
        s,
        &[
            "subscribe",
            "on",
            "addListener",
            "addEventListener",
            "attach",
            "addObserver",
            "register",
        ],
    );
    let unsubscribe = has_method(
        s,
        &[
            "unsubscribe",
            "off",
            "removeListener",
            "removeEventListener",
            "detach",
            "removeObserver",
            "unregister",
        ],
    );
    let notify = has_method(
        s,
        &[
            "emit",
            "notify",
            "notifyAll",
            "publish",
            "dispatch",
            "trigger",
            "fire",
            "broadcast",
        ],
    );
    let listeners = s.members.iter().any(|m| {
        m.kind == MemberKind::Property
            && ["listener", "observer", "subscriber", "handler", "callback"]
                .iter()
                .any(|w| m.name.to_ascii_lowercase().contains(w))
    });
    if !subscribe || !(unsubscribe || notify) {
        return None;
    }
    let mut evidence = vec!["subscribe method".into()];
    let mut score: f32 = 0.45;
    if unsubscribe {
        score += 0.2;
        evidence.push("unsubscribe method".into());
    }
    if notify {
        score += 0.2;
        evidence.push("notify/emit method".into());
    }
    if listeners {
        score += 0.15;
        evidence.push("listener collection".into());
    }
    (score >= MIN_CONFIDENCE).then(|| {
        instance(
            PatternKind::Observer,
            s,
            score.min(0.95),
            evidence,
            vec![role(&s.id, "Subject")],
        )
    })
}

fn strategy(idx: &Index, model: &ArchModel, s: &SymbolNode) -> Option<PatternInstance> {
    if !idx.is_abstraction(&s.id) {
        return None;
    }
    let mut concrete = idx
        .implementers
        .get(s.id.as_str())
        .cloned()
        .unwrap_or_default();
    concrete.sort();
    concrete.dedup();
    if concrete.len() < 2 {
        return None;
    }
    let mut contexts: Vec<&str> = model
        .edges
        .iter()
        .filter(|e| {
            e.to == s.id && e.kind == EdgeKind::Association && !concrete.contains(&e.from.as_str())
        })
        .map(|e| e.from.as_str())
        .collect();
    contexts.sort();
    contexts.dedup();
    if contexts.is_empty() {
        return None;
    }
    let score = (0.7 + 0.05 * concrete.len() as f32).min(0.95);
    let mut participants = vec![role(&s.id, "Strategy")];
    participants.extend(concrete.iter().map(|c| role(c, "Concrete strategy")));
    participants.extend(contexts.iter().map(|c| role(c, "Context")));
    Some(instance(
        PatternKind::Strategy,
        s,
        score,
        vec![
            format!("{} implementations", concrete.len()),
            format!("held by {} context(s)", contexts.len()),
        ],
        participants,
    ))
}

fn decorator(idx: &Index, s: &SymbolNode) -> Option<PatternInstance> {
    if s.kind != SymbolKind::Class {
        return None;
    }
    let wraps: HashSet<&str> = idx
        .related(&s.id, &[EdgeKind::Association])
        .into_iter()
        .collect();
    let component = idx
        .related(&s.id, &[EdgeKind::Implements, EdgeKind::Extends])
        .into_iter()
        .find(|c| wraps.contains(c) && idx.is_abstraction(c))?;
    let named = ["Decorator", "Proxy", "Wrapper", "Cached", "Logging"]
        .iter()
        .any(|w| s.name.contains(w));
    let score = if named { 0.9 } else { 0.72 };
    let mut evidence = vec!["implements and wraps the same abstraction".into()];
    if named {
        evidence.push("decorator-like name".into());
    }
    Some(instance(
        PatternKind::Decorator,
        s,
        score,
        evidence,
        vec![role(&s.id, "Decorator"), role(component, "Component")],
    ))
}

fn adapter(idx: &Index, s: &SymbolNode) -> Option<PatternInstance> {
    if s.kind != SymbolKind::Class || !s.name.contains("Adapter") {
        return None;
    }
    let targets = idx.related(&s.id, &[EdgeKind::Implements, EdgeKind::Extends]);
    let target = targets.first()?;
    let adaptee = idx
        .related(&s.id, &[EdgeKind::Association])
        .into_iter()
        .find(|a| !targets.contains(a));
    let mut participants = vec![role(&s.id, "Adapter"), role(target, "Target")];
    let mut evidence = vec![
        "adapter-like name".into(),
        "implements a target interface".into(),
    ];
    let mut score = 0.7;
    if let Some(a) = adaptee {
        participants.push(role(a, "Adaptee"));
        evidence.push("wraps another type".into());
        score = 0.88;
    }
    Some(instance(
        PatternKind::Adapter,
        s,
        score,
        evidence,
        participants,
    ))
}

fn repository(idx: &Index, s: &SymbolNode) -> Option<PatternInstance> {
    if !matches!(s.kind, SymbolKind::Class | SymbolKind::Interface) {
        return None;
    }
    if !(s.name.ends_with("Repository") || s.name.ends_with("Repo")) {
        return None;
    }
    let crud = methods(s)
        .filter(|m| {
            starts_with_any(
                &m.name,
                &[
                    "find", "get", "list", "save", "create", "insert", "update", "upsert",
                    "delete", "remove", "count", "exists",
                ],
            )
        })
        .count();
    if crud < 2 {
        return None;
    }
    let mut participants = vec![role(&s.id, "Repository")];
    participants.extend(
        idx.related(&s.id, &[EdgeKind::Implements, EdgeKind::Extends])
            .into_iter()
            .map(|p| role(p, "Repository contract")),
    );
    Some(instance(
        PatternKind::Repository,
        s,
        (0.7 + 0.04 * crud as f32).min(0.95),
        vec![
            "named *Repository".into(),
            format!("{crud} data-access methods"),
        ],
        participants,
    ))
}

fn facade(idx: &Index, s: &SymbolNode) -> Option<PatternInstance> {
    if s.kind != SymbolKind::Class || !s.name.ends_with("Facade") {
        return None;
    }
    let collaborators = idx.related(&s.id, &[EdgeKind::Association]);
    if collaborators.len() < 2 {
        return None;
    }
    let mut participants = vec![role(&s.id, "Facade")];
    participants.extend(collaborators.iter().map(|c| role(c, "Subsystem")));
    Some(instance(
        PatternKind::Facade,
        s,
        (0.7 + 0.05 * collaborators.len() as f32).min(0.95),
        vec![
            "named *Facade".into(),
            format!("delegates to {} collaborators", collaborators.len()),
        ],
        participants,
    ))
}

/// All pattern instances in the model, most confident first.
pub fn detect(model: &ArchModel) -> Vec<PatternInstance> {
    let idx = Index::new(model);
    let mut found = Vec::new();
    for s in &model.symbols {
        found.extend(singleton(s));
        found.extend(factory(&idx, s));
        found.extend(builder(&idx, s));
        found.extend(observer(s));
        found.extend(strategy(&idx, model, s));
        found.extend(decorator(&idx, s));
        found.extend(adapter(&idx, s));
        found.extend(repository(&idx, s));
        found.extend(facade(&idx, s));
    }
    found.sort_by(|a, b| {
        b.confidence
            .total_cmp(&a.confidence)
            .then_with(|| a.name.cmp(&b.name))
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::{Edge, Range};

    fn member(name: &str, kind: MemberKind, ty: Option<&str>) -> Member {
        Member {
            name: name.into(),
            kind,
            signature: name.into(),
            visibility: Visibility::Public,
            is_static: false,
            is_async: false,
            is_abstract: false,
            type_text: ty.map(str::to_string),
            line: 1,
        }
    }

    fn sym(name: &str, kind: SymbolKind, members: Vec<Member>) -> SymbolNode {
        SymbolNode {
            id: format!("{name}.ts#{name}"),
            kind,
            name: name.into(),
            file: format!("{name}.ts"),
            range: Range {
                start_line: 1,
                end_line: 10,
            },
            package: "p".into(),
            exported: true,
            is_abstract: false,
            type_params: None,
            signature: String::new(),
            members,
        }
    }

    fn edge(from: &str, to: &str, kind: EdgeKind) -> Edge {
        Edge {
            from: format!("{from}.ts#{from}"),
            to: format!("{to}.ts#{to}"),
            kind,
        }
    }

    fn kinds(model: &ArchModel) -> Vec<(PatternKind, String)> {
        detect(model)
            .into_iter()
            .map(|p| (p.kind, p.name))
            .collect()
    }

    #[test]
    fn detects_singleton() {
        let mut ctor = member("constructor", MemberKind::Constructor, None);
        ctor.visibility = Visibility::Private;
        let mut field = member("instance", MemberKind::Property, Some("Config"));
        field.is_static = true;
        let mut get = member("getInstance", MemberKind::Method, Some("Config"));
        get.is_static = true;
        let model = ArchModel {
            root: String::new(),
            symbols: vec![sym("Config", SymbolKind::Class, vec![ctor, field, get])],
            edges: vec![],
        };
        let found = detect(&model);
        assert_eq!(found[0].kind, PatternKind::Singleton);
        assert!(found[0].confidence > 0.9);
    }

    #[test]
    fn named_constructor_is_not_a_singleton() {
        let mut ctor = member("constructor", MemberKind::Constructor, None);
        ctor.visibility = Visibility::Private;
        let mut from = member("fromToken", MemberKind::Method, Some("Grant"));
        from.is_static = true;
        let model = ArchModel {
            root: String::new(),
            symbols: vec![sym("Grant", SymbolKind::Class, vec![ctor, from])],
            edges: vec![],
        };
        assert!(detect(&model).is_empty());
    }

    #[test]
    fn plain_class_with_static_helper_is_not_a_singleton() {
        let mut helper = member("parse", MemberKind::Method, Some("number"));
        helper.is_static = true;
        let model = ArchModel {
            root: String::new(),
            symbols: vec![sym("Util", SymbolKind::Class, vec![helper])],
            edges: vec![],
        };
        assert!(detect(&model).is_empty());
    }

    #[test]
    fn detects_strategy_with_roles() {
        let model = ArchModel {
            root: String::new(),
            symbols: vec![
                sym("Payment", SymbolKind::Interface, vec![]),
                sym("Card", SymbolKind::Class, vec![]),
                sym("Paypal", SymbolKind::Class, vec![]),
                sym("Checkout", SymbolKind::Class, vec![]),
            ],
            edges: vec![
                edge("Card", "Payment", EdgeKind::Implements),
                edge("Paypal", "Payment", EdgeKind::Implements),
                edge("Checkout", "Payment", EdgeKind::Association),
            ],
        };
        let found = detect(&model);
        let strategy = found
            .iter()
            .find(|p| p.kind == PatternKind::Strategy)
            .unwrap();
        assert_eq!(strategy.id, "strategy:Payment.ts#Payment");
        let roles: Vec<_> = strategy
            .participants
            .iter()
            .map(|p| p.role.as_str())
            .collect();
        assert_eq!(
            roles,
            [
                "Strategy",
                "Concrete strategy",
                "Concrete strategy",
                "Context"
            ]
        );
    }

    #[test]
    fn interface_with_implementers_but_no_context_is_not_strategy() {
        let model = ArchModel {
            root: String::new(),
            symbols: vec![
                sym("Shape", SymbolKind::Interface, vec![]),
                sym("A", SymbolKind::Class, vec![]),
                sym("B", SymbolKind::Class, vec![]),
            ],
            edges: vec![
                edge("A", "Shape", EdgeKind::Implements),
                edge("B", "Shape", EdgeKind::Implements),
            ],
        };
        assert!(
            kinds(&model)
                .iter()
                .all(|(k, _)| *k != PatternKind::Strategy)
        );
    }

    #[test]
    fn detects_builder_observer_repository() {
        let builder = sym(
            "QueryBuilder",
            SymbolKind::Class,
            vec![
                member("where", MemberKind::Method, Some("this")),
                member("limit", MemberKind::Method, Some("this")),
                member("build", MemberKind::Method, Some("Query")),
            ],
        );
        let bus = sym(
            "EventBus",
            SymbolKind::Class,
            vec![
                member("on", MemberKind::Method, Some("void")),
                member("off", MemberKind::Method, Some("void")),
                member("emit", MemberKind::Method, Some("void")),
                member("listeners", MemberKind::Property, Some("Map<string, Fn[]>")),
            ],
        );
        let repo = sym(
            "UserRepository",
            SymbolKind::Class,
            vec![
                member("findById", MemberKind::Method, None),
                member("save", MemberKind::Method, None),
            ],
        );
        let model = ArchModel {
            root: String::new(),
            symbols: vec![builder, bus, repo],
            edges: vec![],
        };
        let found: HashSet<_> = kinds(&model).into_iter().map(|(k, _)| k).collect();
        assert!(found.contains(&PatternKind::Builder));
        assert!(found.contains(&PatternKind::Observer));
        assert!(found.contains(&PatternKind::Repository));
    }

    #[test]
    fn subscribe_without_unsubscribe_or_emit_is_not_observer() {
        let s = sym(
            "Thing",
            SymbolKind::Class,
            vec![member("on", MemberKind::Method, None)],
        );
        let model = ArchModel {
            root: String::new(),
            symbols: vec![s],
            edges: vec![],
        };
        assert!(detect(&model).is_empty());
    }

    #[test]
    fn detects_decorator_and_adapter() {
        let model = ArchModel {
            root: String::new(),
            symbols: vec![
                sym("Storage", SymbolKind::Interface, vec![]),
                sym("CachedStorage", SymbolKind::Class, vec![]),
                sym("S3Client", SymbolKind::Class, vec![]),
                sym("S3StorageAdapter", SymbolKind::Class, vec![]),
            ],
            edges: vec![
                edge("CachedStorage", "Storage", EdgeKind::Implements),
                edge("CachedStorage", "Storage", EdgeKind::Association),
                edge("S3StorageAdapter", "Storage", EdgeKind::Implements),
                edge("S3StorageAdapter", "S3Client", EdgeKind::Association),
            ],
        };
        let found = detect(&model);
        let decorator = found
            .iter()
            .find(|p| p.kind == PatternKind::Decorator)
            .unwrap();
        assert_eq!(decorator.name, "Decorator: CachedStorage");
        let adapter = found
            .iter()
            .find(|p| p.kind == PatternKind::Adapter)
            .unwrap();
        assert_eq!(adapter.participants.len(), 3);
    }

    #[test]
    fn detects_factory_returning_an_abstraction() {
        let model = ArchModel {
            root: String::new(),
            symbols: vec![
                sym("Transport", SymbolKind::Interface, vec![]),
                sym(
                    "TransportFactory",
                    SymbolKind::Class,
                    vec![member("create", MemberKind::Method, Some("Transport"))],
                ),
            ],
            edges: vec![edge("TransportFactory", "Transport", EdgeKind::Dependency)],
        };
        let factory = detect(&model)
            .into_iter()
            .find(|p| p.kind == PatternKind::Factory)
            .unwrap();
        assert!(factory.confidence >= 0.95);
        assert_eq!(factory.participants[1].role, "Product");
    }
}
