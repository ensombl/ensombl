//! [`SourceParser`] for TypeScript/JavaScript built on tree-sitter.
//!
//! Extracts top-level classes, interfaces, enums, type aliases and functions
//! (including `const f = () => …`), their members with source-level
//! signatures, imports/re-exports, and the type names each symbol refers to.

use std::cell::RefCell;
use std::collections::HashSet;

use tree_sitter::{Language, Node, Parser};

use crate::application::ports::SourceParser;
use crate::application::{Error, Result};
use crate::domain::model::{Member, MemberKind, Range, SymbolKind, SymbolNode, Visibility};
use crate::domain::parsed::{
    CallSite, Decorator, DecoratorSite, DecoratorTarget, Import, ImportedName, JsxUse, ParsedFile,
    ReExport, SymbolRefs,
};

pub struct TreeSitterParser;

thread_local! {
    static TS: RefCell<Parser> = RefCell::new(parser_for(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()));
    static TSX: RefCell<Parser> = RefCell::new(parser_for(tree_sitter_typescript::LANGUAGE_TSX.into()));
}

fn parser_for(language: Language) -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("tree-sitter grammar version mismatch");
    parser
}

impl SourceParser for TreeSitterParser {
    fn version(&self) -> u32 {
        2
    }

    fn parse(&self, path: &str, package: &str, source: &str) -> Result<ParsedFile> {
        // Plain .ts can't use the TSX grammar (`<T>x` casts); everything else can.
        let tree = if path.ends_with(".ts") || path.ends_with(".mts") || path.ends_with(".cts") {
            TS.with(|p| p.borrow_mut().parse(source, None))
        } else {
            TSX.with(|p| p.borrow_mut().parse(source, None))
        }
        .ok_or_else(|| Error::Invalid(format!("failed to parse {path}")))?;

        let mut extractor = Extractor {
            src: source.as_bytes(),
            path,
            package,
            out: ParsedFile::default(),
            class_decorators: vec![],
        };
        extractor.out.path = path.into();
        extractor.program(tree.root_node());
        Ok(extractor.finish())
    }
}

struct Extractor<'a> {
    src: &'a [u8],
    path: &'a str,
    package: &'a str,
    out: ParsedFile,
    /// Decorators written before `export class`, which sit on the export statement.
    class_decorators: Vec<Decorator>,
}

/// Globals whose calls say nothing about architecture.
const NOISE_OBJECTS: &[&str] = &[
    "console", "JSON", "Math", "Object", "Array", "Promise", "Number", "String", "Date", "Reflect",
    "Symbol", "process", "window", "document", "Buffer",
];
/// Collection and string helpers, ignored unless called on an injected field.
const NOISE_METHODS: &[&str] = &[
    "then",
    "catch",
    "finally",
    "map",
    "filter",
    "forEach",
    "reduce",
    "push",
    "pop",
    "shift",
    "unshift",
    "join",
    "split",
    "trim",
    "toString",
    "slice",
    "splice",
    "concat",
    "includes",
    "indexOf",
    "some",
    "every",
    "sort",
    "replace",
    "toLowerCase",
    "toUpperCase",
    "startsWith",
    "endsWith",
    "flatMap",
    "padStart",
    "toFixed",
    "bind",
    "call",
    "apply",
];
const NOISE_FUNCTIONS: &[&str] = &[
    "require",
    "setTimeout",
    "setInterval",
    "clearTimeout",
    "clearInterval",
    "parseInt",
    "parseFloat",
    "String",
    "Number",
    "Boolean",
    "Array",
    "Error",
    "isNaN",
    "structuredClone",
];

fn is_noise(object: &[String], method: &str) -> bool {
    match object.first().map(String::as_str) {
        None => NOISE_FUNCTIONS.contains(&method),
        Some("this") if object.len() > 1 => false,
        Some(root) => NOISE_OBJECTS.contains(&root) || NOISE_METHODS.contains(&method),
    }
}

fn named_children<'t>(node: Node<'t>) -> Vec<Node<'t>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn children<'t>(node: Node<'t>) -> Vec<Node<'t>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

fn has_token(node: Node, token: &str) -> bool {
    children(node).iter().any(|c| c.kind() == token)
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl<'a> Extractor<'a> {
    fn text(&self, node: Node) -> &'a str {
        node.utf8_text(self.src).unwrap_or("")
    }

    fn range(&self, node: Node) -> Range {
        Range {
            start_line: node.start_position().row as u32 + 1,
            end_line: node.end_position().row as u32 + 1,
        }
    }

    fn finish(mut self) -> ParsedFile {
        // Local `export { A, B as C }` marks A as exported under its own name.
        let local_exports: HashSet<String> = self
            .out
            .reexports
            .iter()
            .filter_map(|r| match r {
                ReExport::Named {
                    local,
                    exported,
                    specifier: None,
                } if local == exported => Some(local.clone()),
                _ => None,
            })
            .collect();
        for s in &mut self.out.symbols {
            if local_exports.contains(&s.name) {
                s.exported = true;
            }
        }
        // Function overloads produce one symbol; keep the first declaration.
        let mut seen = HashSet::new();
        self.out.symbols.retain(|s| seen.insert(s.id.clone()));
        let mut seen = HashSet::new();
        self.out.refs.retain(|r| seen.insert(r.symbol_id.clone()));
        self.out
    }

    fn program(&mut self, root: Node) {
        for node in named_children(root) {
            if node.kind() == "expression_statement" {
                self.walk(node, None, None);
            }
            self.statement(node, false);
        }
    }

    fn statement(&mut self, node: Node, exported: bool) {
        match node.kind() {
            "export_statement" => self.export_statement(node),
            "import_statement" => self.import_statement(node),
            "ambient_declaration" => {
                for child in named_children(node) {
                    self.statement(child, exported);
                }
            }
            "class_declaration" | "abstract_class_declaration" | "class" => {
                self.class(node, exported);
            }
            "interface_declaration" => self.interface(node, exported),
            "enum_declaration" => self.enum_decl(node, exported),
            "type_alias_declaration" => self.type_alias(node, exported),
            "function_declaration" | "generator_function_declaration" | "function_signature" => {
                self.function(node, exported);
            }
            "lexical_declaration" | "variable_declaration" => self.variables(node, exported),
            _ => {}
        }
    }

    fn export_statement(&mut self, node: Node) {
        let source = node
            .child_by_field_name("source")
            .map(|s| self.string_value(s));
        let is_default = has_token(node, "default");

        if let Some(decl) = node.child_by_field_name("declaration") {
            self.class_decorators = children(node)
                .into_iter()
                .filter(|c| c.kind() == "decorator")
                .filter_map(|c| self.decorator(c))
                .collect();
            let before = self.out.symbols.len();
            self.statement(decl, true);
            self.class_decorators.clear();
            if is_default && self.out.symbols.len() > before {
                self.out.default_export = Some(self.out.symbols[before].name.clone());
            }
            return;
        }
        if let Some(value) = node.child_by_field_name("value") {
            match value.kind() {
                "identifier" => self.out.default_export = Some(self.text(value).into()),
                "class" | "class_declaration" => {
                    let before = self.out.symbols.len();
                    self.class(value, true);
                    if self.out.symbols.len() > before {
                        self.out.default_export = Some(self.out.symbols[before].name.clone());
                    }
                }
                _ => {}
            }
            return;
        }

        let mut had_clause = false;
        for child in named_children(node) {
            match child.kind() {
                "export_clause" => {
                    had_clause = true;
                    for spec in named_children(child)
                        .into_iter()
                        .filter(|s| s.kind() == "export_specifier")
                    {
                        let Some(name) = spec.child_by_field_name("name") else {
                            continue;
                        };
                        let local = self.text(name).to_string();
                        let exported = spec
                            .child_by_field_name("alias")
                            .map(|a| self.text(a).to_string())
                            .unwrap_or_else(|| local.clone());
                        self.out.reexports.push(ReExport::Named {
                            local,
                            exported,
                            specifier: source.clone(),
                        });
                    }
                }
                "namespace_export" => had_clause = true,
                _ => {}
            }
        }
        if !had_clause && let Some(specifier) = source {
            self.out.reexports.push(ReExport::All { specifier });
        }
    }

    fn import_statement(&mut self, node: Node) {
        let Some(source) = node.child_by_field_name("source") else {
            return;
        };
        let specifier = self.string_value(source);
        let Some(clause) = named_children(node)
            .into_iter()
            .find(|c| c.kind() == "import_clause")
        else {
            return;
        };
        for part in named_children(clause) {
            match part.kind() {
                "identifier" => self.out.imports.push(Import {
                    local: self.text(part).into(),
                    imported: ImportedName::Default,
                    specifier: specifier.clone(),
                }),
                "namespace_import" => {
                    if let Some(id) = named_children(part)
                        .into_iter()
                        .find(|c| c.kind() == "identifier")
                    {
                        self.out.imports.push(Import {
                            local: self.text(id).into(),
                            imported: ImportedName::Namespace,
                            specifier: specifier.clone(),
                        });
                    }
                }
                "named_imports" => {
                    for spec in named_children(part)
                        .into_iter()
                        .filter(|s| s.kind() == "import_specifier")
                    {
                        let Some(name) = spec.child_by_field_name("name") else {
                            continue;
                        };
                        let imported = self.text(name).to_string();
                        let local = spec
                            .child_by_field_name("alias")
                            .map(|a| self.text(a).to_string())
                            .unwrap_or_else(|| imported.clone());
                        self.out.imports.push(Import {
                            local,
                            imported: ImportedName::Named(imported),
                            specifier: specifier.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    fn string_value(&self, node: Node) -> String {
        self.text(node)
            .trim_matches(|c| c == '"' || c == '\'' || c == '`')
            .to_string()
    }

    fn type_params(&self, node: Node) -> (Option<String>, HashSet<String>) {
        let Some(tp) = node.child_by_field_name("type_parameters") else {
            return (None, HashSet::new());
        };
        let names = named_children(tp)
            .into_iter()
            .filter_map(|p| p.child_by_field_name("name"))
            .map(|n| self.text(n).to_string())
            .collect();
        (Some(squash(self.text(tp))), names)
    }

    fn symbol(
        &self,
        node: Node,
        kind: SymbolKind,
        name: &str,
        exported: bool,
        signature: String,
    ) -> SymbolNode {
        SymbolNode {
            id: format!("{}#{}", self.path, name),
            kind,
            name: name.into(),
            file: self.path.into(),
            range: self.range(node),
            package: self.package.into(),
            exported,
            is_abstract: false,
            type_params: None,
            signature,
            members: vec![],
        }
    }

    /// Collects type names referenced anywhere under `node` (`Foo`, `ns.Foo`).
    fn type_refs(&self, node: Node, skip: &HashSet<String>, out: &mut Vec<String>) {
        match node.kind() {
            "type_identifier" => {
                let name = self.text(node);
                if !skip.contains(name) && !out.iter().any(|n| n == name) {
                    out.push(name.into());
                }
                return;
            }
            "nested_type_identifier" => {
                let name = self.text(node).to_string();
                if !out.contains(&name) {
                    out.push(name);
                }
                return;
            }
            _ => {}
        }
        for child in named_children(node) {
            self.type_refs(child, skip, out);
        }
    }

    fn class(&mut self, node: Node, exported: bool) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.text(name_node).to_string();
        let (type_params, generics) = self.type_params(node);
        let is_abstract = node.kind() == "abstract_class_declaration";

        let mut refs = SymbolRefs {
            symbol_id: format!("{}#{}", self.path, name),
            ..Default::default()
        };
        let mut header_end = name_node.end_byte();
        for child in named_children(node) {
            if child.kind() != "class_heritage" {
                continue;
            }
            header_end = child.end_byte();
            for clause in named_children(child) {
                match clause.kind() {
                    "extends_clause" => {
                        if let Some(value) = clause.child_by_field_name("value") {
                            refs.extends.push(self.text(value).to_string());
                        }
                        if let Some(args) = clause.child_by_field_name("type_arguments") {
                            self.type_refs(args, &generics, &mut refs.dependencies);
                        }
                    }
                    "implements_clause" => {
                        self.heritage_types(clause, &generics, &mut refs.implements)
                    }
                    _ => {}
                }
            }
        }

        // Decorators are children of the class node; keep them out of the header.
        let start = children(node)
            .into_iter()
            .find(|c| c.kind() != "decorator")
            .map_or(node.start_byte(), |c| c.start_byte());
        let signature = squash(std::str::from_utf8(&self.src[start..header_end]).unwrap_or(""));
        let mut symbol = self.symbol(node, SymbolKind::Class, &name, exported, signature);
        symbol.type_params = type_params;
        symbol.is_abstract = is_abstract;

        if let Some(body) = node.child_by_field_name("body") {
            for member in named_children(body) {
                self.class_member(member, &generics, &mut symbol.members, &mut refs);
            }
        }
        let own: Vec<Decorator> = children(node)
            .into_iter()
            .filter(|c| c.kind() == "decorator")
            .filter_map(|c| self.decorator(c))
            .collect();
        let decorators = std::mem::take(&mut self.class_decorators);
        for decorator in decorators.into_iter().chain(own) {
            self.out.facts.decorators.push(DecoratorSite {
                symbol_id: symbol.id.clone(),
                target: DecoratorTarget::Class,
                decorator,
            });
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.class_facts(body, &symbol.id.clone());
        }
        self.out.symbols.push(symbol);
        self.out.refs.push(refs);
    }

    /// Member and parameter decorators, and the calls each member makes.
    fn class_facts(&mut self, body: Node, symbol_id: &str) {
        let mut pending: Vec<Decorator> = vec![];
        for child in named_children(body) {
            if child.kind() == "decorator" {
                pending.extend(self.decorator(child));
                continue;
            }
            let Some(name) = child.child_by_field_name("name") else {
                pending.clear();
                continue;
            };
            let member = self.text(name).to_string();
            let own: Vec<Decorator> = children(child)
                .into_iter()
                .filter(|c| c.kind() == "decorator")
                .filter_map(|c| self.decorator(c))
                .collect();
            for decorator in pending.drain(..).chain(own) {
                self.out.facts.decorators.push(DecoratorSite {
                    symbol_id: symbol_id.into(),
                    target: DecoratorTarget::Member(member.clone()),
                    decorator,
                });
            }
            if let Some(params) = child.child_by_field_name("parameters") {
                for param in named_children(params) {
                    let Some(pattern) = param.child_by_field_name("pattern") else {
                        continue;
                    };
                    let param_name = self.text(pattern).to_string();
                    for dec in children(param)
                        .into_iter()
                        .filter(|c| c.kind() == "decorator")
                    {
                        if let Some(decorator) = self.decorator(dec) {
                            self.out.facts.decorators.push(DecoratorSite {
                                symbol_id: symbol_id.into(),
                                target: DecoratorTarget::Param {
                                    member: member.clone(),
                                    param: param_name.clone(),
                                },
                                decorator,
                            });
                        }
                    }
                }
            }
            if let Some(code) = child
                .child_by_field_name("body")
                .or_else(|| child.child_by_field_name("value"))
            {
                self.walk(code, Some(symbol_id), Some(&member));
            }
        }
    }

    fn decorator(&self, node: Node) -> Option<Decorator> {
        let inner = named_children(node).into_iter().next()?;
        let (callee, args) = match inner.kind() {
            "call_expression" => (
                inner.child_by_field_name("function")?,
                inner.child_by_field_name("arguments"),
            ),
            _ => (inner, None),
        };
        let mut decorator = Decorator {
            name: self.text(callee).rsplit('.').next()?.to_string(),
            ..Default::default()
        };
        for arg in args.map(named_children).unwrap_or_default() {
            match arg.kind() {
                "string" | "template_string" => decorator.strings.push(self.literal(arg)),
                "identifier" => decorator.idents.push(self.text(arg).into()),
                "object" => {
                    for pair in named_children(arg)
                        .into_iter()
                        .filter(|p| p.kind() == "pair")
                    {
                        let (Some(key), Some(value)) = (
                            pair.child_by_field_name("key"),
                            pair.child_by_field_name("value"),
                        ) else {
                            continue;
                        };
                        decorator
                            .fields
                            .push((self.string_value(key), self.list_values(value)));
                    }
                }
                _ => {}
            }
        }
        Some(decorator)
    }

    /// Identifiers and strings in `[A, B, 'c']` or a single value.
    fn list_values(&self, node: Node) -> Vec<String> {
        let items = if node.kind() == "array" {
            named_children(node)
        } else {
            vec![node]
        };
        items
            .into_iter()
            .filter_map(|n| match n.kind() {
                "identifier" | "member_expression" => Some(self.text(n).to_string()),
                "string" | "template_string" => Some(self.literal(n)),
                _ => None,
            })
            .collect()
    }

    /// A string or template literal's text; template holes become `*`.
    fn literal(&self, node: Node) -> String {
        if node.kind() != "template_string" {
            return self.string_value(node);
        }
        let mut out = String::new();
        for part in children(node) {
            match part.kind() {
                "`" => {}
                "template_substitution" => out.push('*'),
                _ => out.push_str(self.text(part)),
            }
        }
        out
    }

    /// Records calls and JSX under `node`, attributed to the enclosing symbol and member.
    fn walk(&mut self, node: Node, symbol: Option<&str>, member: Option<&str>) {
        match node.kind() {
            "call_expression" => self.call(node, symbol, member),
            "jsx_opening_element" | "jsx_self_closing_element" => {
                if let Some(name) = node.child_by_field_name("name") {
                    let name = self.text(name);
                    let last = name.rsplit('.').next().unwrap_or(name);
                    if last.starts_with(|c: char| c.is_ascii_uppercase()) {
                        self.out.facts.jsx.push(JsxUse {
                            symbol_id: symbol.map(Into::into),
                            name: name.into(),
                            line: node.start_position().row as u32 + 1,
                        });
                    }
                }
            }
            _ => {}
        }
        for child in named_children(node) {
            self.walk(child, symbol, member);
        }
    }

    fn call(&mut self, node: Node, symbol: Option<&str>, member: Option<&str>) {
        let Some(callee) = node.child_by_field_name("function") else {
            return;
        };
        let (object, method) = match callee.kind() {
            "identifier" => (vec![], self.text(callee).to_string()),
            "member_expression" => {
                let (Some(object), Some(property)) = (
                    callee.child_by_field_name("object"),
                    callee.child_by_field_name("property"),
                ) else {
                    return;
                };
                let Some(path) = self.object_path(object) else {
                    return;
                };
                (path, self.text(property).to_string())
            }
            _ => return,
        };
        if is_noise(&object, &method) {
            return;
        }
        let mut site = CallSite {
            symbol_id: symbol.map(Into::into),
            member: member.map(Into::into),
            object,
            method,
            line: node.start_position().row as u32 + 1,
            ..Default::default()
        };
        if let Some(args) = node.child_by_field_name("arguments") {
            for arg in named_children(args) {
                match arg.kind() {
                    "string" | "template_string" => site.strings.push(self.literal(arg)),
                    "identifier" => site.idents.push(self.text(arg).into()),
                    _ => {}
                }
            }
        }
        self.out.facts.calls.push(site);
    }

    /// `this.repo` → `["this", "repo"]`; `None` when the object isn't a plain path.
    fn object_path(&self, node: Node) -> Option<Vec<String>> {
        match node.kind() {
            "this" | "super" | "identifier" => Some(vec![self.text(node).into()]),
            "member_expression" => {
                let mut path = self.object_path(node.child_by_field_name("object")?)?;
                path.push(self.text(node.child_by_field_name("property")?).into());
                Some(path)
            }
            "non_null_expression" | "parenthesized_expression" => {
                self.object_path(named_children(node).into_iter().next()?)
            }
            _ => None,
        }
    }

    /// Type names in an `implements`/`extends` type list (`A, B<T>, ns.C`).
    fn heritage_types(&self, clause: Node, generics: &HashSet<String>, out: &mut Vec<String>) {
        for ty in named_children(clause) {
            match ty.kind() {
                "type_identifier" | "nested_type_identifier" => out.push(self.text(ty).into()),
                "generic_type" => {
                    if let Some(name) = ty.child_by_field_name("name") {
                        out.push(self.text(name).into());
                    }
                }
                _ => self.type_refs(ty, generics, out),
            }
        }
    }

    fn modifiers(&self, node: Node) -> (Visibility, bool, bool, bool) {
        let mut visibility = Visibility::Public;
        let (mut is_static, mut is_async, mut is_abstract) = (false, false, false);
        for child in children(node) {
            match child.kind() {
                "accessibility_modifier" => {
                    visibility = match self.text(child) {
                        "private" => Visibility::Private,
                        "protected" => Visibility::Protected,
                        _ => Visibility::Public,
                    }
                }
                "static" => is_static = true,
                "async" => is_async = true,
                "abstract" => is_abstract = true,
                _ => {}
            }
        }
        if let Some(name) = node.child_by_field_name("name")
            && name.kind() == "private_property_identifier"
        {
            visibility = Visibility::Private;
        }
        (visibility, is_static, is_async, is_abstract)
    }

    /// Text from the member name to the start of its body/initializer.
    fn member_signature(&self, node: Node, name: Node) -> String {
        let end = node
            .child_by_field_name("body")
            .or_else(|| node.child_by_field_name("value"))
            .map(|b| b.start_byte())
            .unwrap_or(node.end_byte());
        let raw = std::str::from_utf8(&self.src[name.start_byte()..end.max(name.end_byte())])
            .unwrap_or("");
        squash(raw.trim_end().trim_end_matches([';', '=', ',']).trim_end())
    }

    fn class_member(
        &self,
        node: Node,
        generics: &HashSet<String>,
        members: &mut Vec<Member>,
        refs: &mut SymbolRefs,
    ) {
        match node.kind() {
            "method_definition" | "method_signature" | "abstract_method_signature" => {
                let Some(name) = node.child_by_field_name("name") else {
                    return;
                };
                let member_name = self.text(name).to_string();
                let (visibility, is_static, is_async, mut is_abstract) = self.modifiers(node);
                is_abstract |= node.kind() == "abstract_method_signature";
                let kind = if member_name == "constructor" {
                    MemberKind::Constructor
                } else if has_token(node, "get") {
                    MemberKind::Getter
                } else if has_token(node, "set") {
                    MemberKind::Setter
                } else {
                    MemberKind::Method
                };
                // An overload followed by its implementation: keep the first signature only.
                if members
                    .iter()
                    .any(|m| m.name == member_name && m.kind == kind && m.is_static == is_static)
                {
                    return;
                }
                let return_type = node.child_by_field_name("return_type");
                if let Some(params) = node.child_by_field_name("parameters") {
                    if kind == MemberKind::Constructor {
                        self.parameter_properties(params, generics, members, refs);
                    }
                    self.type_refs(params, generics, &mut refs.dependencies);
                }
                if let Some(rt) = return_type {
                    self.type_refs(rt, generics, &mut refs.dependencies);
                }
                members.push(Member {
                    name: member_name,
                    kind,
                    signature: self.member_signature(node, name),
                    visibility,
                    is_static,
                    is_async,
                    is_abstract,
                    type_text: return_type
                        .map(|t| squash(self.text(t).trim_start_matches(':').trim())),
                    line: node.start_position().row as u32 + 1,
                });
            }
            "public_field_definition" | "property_signature" => {
                let Some(name) = node.child_by_field_name("name") else {
                    return;
                };
                let (visibility, is_static, _, is_abstract) = self.modifiers(node);
                let ty = node.child_by_field_name("type");
                if let Some(ty) = ty {
                    self.type_refs(ty, generics, &mut refs.associations);
                }
                members.push(Member {
                    name: self.text(name).into(),
                    kind: MemberKind::Property,
                    signature: self.member_signature(node, name),
                    visibility,
                    is_static,
                    is_async: false,
                    is_abstract,
                    type_text: ty.map(|t| squash(self.text(t).trim_start_matches(':').trim())),
                    line: node.start_position().row as u32 + 1,
                });
            }
            _ => {}
        }
    }

    /// `constructor(private readonly repo: Repo)` declares a property too.
    fn parameter_properties(
        &self,
        params: Node,
        generics: &HashSet<String>,
        members: &mut Vec<Member>,
        refs: &mut SymbolRefs,
    ) {
        for param in named_children(params) {
            let is_property = children(param)
                .iter()
                .any(|c| c.kind() == "accessibility_modifier" || c.kind() == "readonly");
            if !is_property {
                continue;
            }
            let Some(pattern) = param.child_by_field_name("pattern") else {
                continue;
            };
            let (visibility, ..) = self.modifiers(param);
            let ty = param.child_by_field_name("type");
            if let Some(ty) = ty {
                self.type_refs(ty, generics, &mut refs.associations);
            }
            let type_text = ty.map(|t| squash(self.text(t).trim_start_matches(':').trim()));
            members.push(Member {
                name: self.text(pattern).into(),
                kind: MemberKind::Property,
                signature: match &type_text {
                    Some(t) => format!("{}: {t}", self.text(pattern)),
                    None => self.text(pattern).into(),
                },
                visibility,
                is_static: false,
                is_async: false,
                is_abstract: false,
                type_text,
                line: param.start_position().row as u32 + 1,
            });
        }
    }

    fn interface(&mut self, node: Node, exported: bool) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.text(name_node).to_string();
        let (type_params, generics) = self.type_params(node);
        let mut refs = SymbolRefs {
            symbol_id: format!("{}#{}", self.path, name),
            ..Default::default()
        };

        let mut header_end = name_node.end_byte();
        if let Some(tp) = node.child_by_field_name("type_parameters") {
            header_end = tp.end_byte();
        }
        for child in named_children(node) {
            if child.kind() == "extends_type_clause" {
                header_end = child.end_byte();
                self.heritage_types(child, &generics, &mut refs.extends);
            }
        }
        let signature =
            squash(std::str::from_utf8(&self.src[node.start_byte()..header_end]).unwrap_or(""));
        let mut symbol = self.symbol(node, SymbolKind::Interface, &name, exported, signature);
        symbol.type_params = type_params;

        if let Some(body) = node.child_by_field_name("body") {
            for member in named_children(body) {
                self.class_member(member, &generics, &mut symbol.members, &mut refs);
            }
        }
        self.out.symbols.push(symbol);
        self.out.refs.push(refs);
    }

    fn enum_decl(&mut self, node: Node, exported: bool) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.text(name_node).to_string();
        let mut symbol = self.symbol(
            node,
            SymbolKind::Enum,
            &name,
            exported,
            format!("enum {name}"),
        );
        if let Some(body) = node.child_by_field_name("body") {
            for member in named_children(body) {
                let name = match member.kind() {
                    "property_identifier" | "string" => Some(member),
                    "enum_assignment" => member.child_by_field_name("name"),
                    _ => None,
                };
                if let Some(n) = name {
                    symbol.members.push(Member {
                        name: self.text(n).into(),
                        kind: MemberKind::EnumMember,
                        signature: squash(self.text(member)),
                        visibility: Visibility::Public,
                        is_static: true,
                        is_async: false,
                        is_abstract: false,
                        type_text: None,
                        line: member.start_position().row as u32 + 1,
                    });
                }
            }
        }
        self.out.symbols.push(symbol);
    }

    fn type_alias(&mut self, node: Node, exported: bool) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.text(name_node).to_string();
        let (type_params, generics) = self.type_params(node);
        let mut refs = SymbolRefs {
            symbol_id: format!("{}#{}", self.path, name),
            ..Default::default()
        };
        if let Some(value) = node.child_by_field_name("value") {
            self.type_refs(value, &generics, &mut refs.dependencies);
        }
        let mut symbol = self.symbol(
            node,
            SymbolKind::TypeAlias,
            &name,
            exported,
            squash(self.text(node).trim_end_matches(';')),
        );
        symbol.type_params = type_params;
        self.out.symbols.push(symbol);
        self.out.refs.push(refs);
    }

    fn function(&mut self, node: Node, exported: bool) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = self.text(name_node).to_string();
        self.function_like(node, node, &name, exported, has_token(node, "async"));
    }

    /// `const f = (…) => …` / `const f = function (…) {…}` count as functions.
    fn variables(&mut self, node: Node, exported: bool) {
        for decl in named_children(node)
            .into_iter()
            .filter(|d| d.kind() == "variable_declarator")
        {
            let (Some(name), Some(value)) = (
                decl.child_by_field_name("name"),
                decl.child_by_field_name("value"),
            ) else {
                continue;
            };
            match value.kind() {
                "arrow_function" | "function_expression" | "function" => {
                    let name = self.text(name).to_string();
                    self.function_like(decl, value, &name, exported, has_token(value, "async"));
                }
                "class" => {
                    // `const Foo = class { … }`: treat as a class named Foo.
                    let before = self.out.symbols.len();
                    self.class(value, exported);
                    if self.out.symbols.len() == before {
                        continue;
                    }
                }
                _ => {
                    let name = self.text(name).to_string();
                    // `const Card = memo(() => …)`: a wrapped component is still a function.
                    let wrapped = (value.kind() == "call_expression"
                        && name.starts_with(|c: char| c.is_ascii_uppercase()))
                    .then(|| value.child_by_field_name("arguments"))
                    .flatten()
                    .and_then(|args| named_children(args).into_iter().next())
                    .filter(|f| matches!(f.kind(), "arrow_function" | "function_expression"));
                    match wrapped {
                        Some(func) => self.function_like(
                            decl,
                            func,
                            &name,
                            exported,
                            has_token(func, "async"),
                        ),
                        // Module-level wiring like `const app = express()`.
                        None => self.walk(value, None, None),
                    }
                }
            }
        }
    }

    fn function_like(
        &mut self,
        outer: Node,
        func: Node,
        name: &str,
        exported: bool,
        is_async: bool,
    ) {
        let (type_params, generics) = self.type_params(func);
        let mut refs = SymbolRefs {
            symbol_id: format!("{}#{}", self.path, name),
            ..Default::default()
        };
        let params = func
            .child_by_field_name("parameters")
            .or_else(|| func.child_by_field_name("parameter"));
        let return_type = func.child_by_field_name("return_type");
        if let Some(p) = params {
            self.type_refs(p, &generics, &mut refs.dependencies);
        }
        if let Some(rt) = return_type {
            self.type_refs(rt, &generics, &mut refs.dependencies);
        }
        let mut signature = String::new();
        if is_async {
            signature.push_str("async ");
        }
        signature.push_str(name);
        if let Some(tp) = &type_params {
            signature.push_str(tp);
        }
        match params {
            Some(p) if p.kind() == "formal_parameters" => signature.push_str(&squash(self.text(p))),
            Some(p) => signature.push_str(&format!("({})", squash(self.text(p)))),
            None => signature.push_str("()"),
        }
        if let Some(rt) = return_type {
            signature.push_str(&squash(self.text(rt)));
        }
        let mut symbol = self.symbol(outer, SymbolKind::Function, name, exported, signature);
        symbol.type_params = type_params;
        let id = symbol.id.clone();
        self.out.symbols.push(symbol);
        self.out.refs.push(refs);
        if let Some(body) = func.child_by_field_name("body") {
            self.walk(body, Some(&id), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(path: &str, src: &str) -> ParsedFile {
        TreeSitterParser.parse(path, "pkg", src).unwrap()
    }

    fn sym<'a>(f: &'a ParsedFile, name: &str) -> &'a SymbolNode {
        f.symbols
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("no symbol {name}: {:#?}", f.symbols))
    }

    fn refs<'a>(f: &'a ParsedFile, name: &str) -> &'a SymbolRefs {
        f.refs
            .iter()
            .find(|r| r.symbol_id.ends_with(&format!("#{name}")))
            .unwrap()
    }

    const SERVICE: &str = r#"
import { Repo, type Entity } from "./repo";
import Logger from "./logger";
import * as events from "./events";

/** A user service. */
export class UserService<T extends Entity> extends BaseService<T> implements Service, events.Emitter {
  private static instance: UserService<any>;
  readonly name: string = "users";
  #secret = 1;

  constructor(private readonly repo: Repo<T>, protected log: Logger, plain: number) {
    super();
  }

  static getInstance(): UserService<any> { return this.instance; }

  async save(user: T, opts?: SaveOptions): Promise<void> {}

  get count(): number { return 0; }

  find(id: string): T;
  find(id: number): T;
  find(id: any): T { return null as any; }

  protected abstract validate(input: Input): boolean;
}
"#;

    #[test]
    fn extracts_class_header_and_heritage() {
        let f = parse("src/user.ts", SERVICE);
        let c = sym(&f, "UserService");
        assert_eq!(c.kind, SymbolKind::Class);
        assert!(c.exported);
        assert_eq!(c.id, "src/user.ts#UserService");
        assert_eq!(c.type_params.as_deref(), Some("<T extends Entity>"));
        assert_eq!(
            c.signature,
            "export class UserService<T extends Entity> extends BaseService<T> implements Service, events.Emitter"
                .replace("export ", "")
        );
        let r = refs(&f, "UserService");
        assert_eq!(r.extends, ["BaseService"]);
        assert_eq!(r.implements, ["Service", "events.Emitter"]);
    }

    #[test]
    fn extracts_members_with_signatures_and_modifiers() {
        let f = parse("src/user.ts", SERVICE);
        let c = sym(&f, "UserService");
        let m = |name: &str| {
            c.members
                .iter()
                .find(|m| m.name == name)
                .unwrap_or_else(|| panic!("no member {name}"))
        };

        let instance = m("instance");
        assert_eq!(instance.visibility, Visibility::Private);
        assert!(instance.is_static);
        assert_eq!(instance.signature, "instance: UserService<any>");

        assert_eq!(m("name").signature, "name: string");
        assert_eq!(m("#secret").visibility, Visibility::Private);

        let save = m("save");
        assert_eq!(
            save.signature,
            "save(user: T, opts?: SaveOptions): Promise<void>"
        );
        assert!(save.is_async);
        assert_eq!(save.type_text.as_deref(), Some("Promise<void>"));

        assert_eq!(m("count").kind, MemberKind::Getter);
        assert_eq!(
            m("getInstance").signature,
            "getInstance(): UserService<any>"
        );
        assert!(m("validate").is_abstract);
        assert_eq!(m("validate").visibility, Visibility::Protected);

        assert_eq!(c.members.iter().filter(|m| m.name == "find").count(), 1);
        assert_eq!(m("find").signature, "find(id: string): T");

        let ctor = m("constructor");
        assert_eq!(ctor.kind, MemberKind::Constructor);
        let repo = m("repo");
        assert_eq!(repo.visibility, Visibility::Private);
        assert_eq!(repo.signature, "repo: Repo<T>");
        assert_eq!(m("log").visibility, Visibility::Protected);
        assert!(!c.members.iter().any(|m| m.name == "plain"));
    }

    #[test]
    fn classifies_type_references() {
        let f = parse("src/user.ts", SERVICE);
        let r = refs(&f, "UserService");
        for name in ["Repo", "Logger", "UserService"] {
            assert!(
                r.associations.contains(&name.to_string()),
                "association {name}: {r:?}"
            );
        }
        for name in ["SaveOptions", "Promise", "Input"] {
            assert!(
                r.dependencies.contains(&name.to_string()),
                "dependency {name}: {r:?}"
            );
        }
        assert!(
            !r.dependencies.contains(&"T".to_string()),
            "generic params are not refs"
        );
    }

    #[test]
    fn extracts_imports() {
        let f = parse("src/user.ts", SERVICE);
        assert_eq!(
            f.imports,
            vec![
                Import {
                    local: "Repo".into(),
                    imported: ImportedName::Named("Repo".into()),
                    specifier: "./repo".into()
                },
                Import {
                    local: "Entity".into(),
                    imported: ImportedName::Named("Entity".into()),
                    specifier: "./repo".into()
                },
                Import {
                    local: "Logger".into(),
                    imported: ImportedName::Default,
                    specifier: "./logger".into()
                },
                Import {
                    local: "events".into(),
                    imported: ImportedName::Namespace,
                    specifier: "./events".into()
                },
            ]
        );
    }

    #[test]
    fn extracts_interfaces_enums_aliases_and_functions() {
        let f = parse(
            "src/types.ts",
            r#"
export interface Repo<T> extends Base, Readable<T> {
  find(id: string): Promise<T | undefined>;
  readonly size: number;
}
enum Color { Red, Green = "g" }
export type Id = string | UserId;
export async function load<T>(repo: Repo<T>, id: Id): Promise<T> { return null as any }
export const make = (cfg: Config): Repo<User> => null as any;
const notAFunction = 42;
"#,
        );
        let repo = sym(&f, "Repo");
        assert_eq!(repo.kind, SymbolKind::Interface);
        assert_eq!(
            repo.signature,
            "interface Repo<T> extends Base, Readable<T>"
        );
        assert_eq!(repo.members.len(), 2);
        assert_eq!(
            repo.members[0].signature,
            "find(id: string): Promise<T | undefined>"
        );
        assert_eq!(refs(&f, "Repo").extends, ["Base", "Readable"]);

        let color = sym(&f, "Color");
        assert!(!color.exported);
        assert_eq!(
            color
                .members
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>(),
            ["Red", "Green"]
        );

        assert_eq!(sym(&f, "Id").kind, SymbolKind::TypeAlias);
        assert_eq!(refs(&f, "Id").dependencies, ["UserId"]);

        let load = sym(&f, "load");
        assert_eq!(
            load.signature,
            "async load<T>(repo: Repo<T>, id: Id): Promise<T>"
        );
        assert_eq!(refs(&f, "load").dependencies, ["Repo", "Id", "Promise"]);

        assert_eq!(sym(&f, "make").signature, "make(cfg: Config): Repo<User>");
        assert!(f.symbols.iter().all(|s| s.name != "notAFunction"));
    }

    #[test]
    fn extracts_exports_and_defaults() {
        let f = parse(
            "src/index.ts",
            r#"
export * from "./a";
export { B as Bee, C } from "./bc";
export * as ns from "./ns";
class Local {}
export { Local };
export default class Main {}
"#,
        );
        assert_eq!(f.default_export.as_deref(), Some("Main"));
        assert!(sym(&f, "Local").exported);
        assert!(sym(&f, "Main").exported);
        assert!(f.reexports.contains(&ReExport::All {
            specifier: "./a".into()
        }));
        assert!(f.reexports.contains(&ReExport::Named {
            local: "B".into(),
            exported: "Bee".into(),
            specifier: Some("./bc".into())
        }));
        assert_eq!(
            f.reexports
                .iter()
                .filter(|r| matches!(r, ReExport::All { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn default_export_of_identifier() {
        let f = parse("a.ts", "class A {}\nexport default A;");
        assert_eq!(f.default_export.as_deref(), Some("A"));
    }

    #[test]
    fn parses_tsx_and_js() {
        let f = parse(
            "src/App.tsx",
            "export function App({ title }: Props) { return <div>{title}</div> }",
        );
        assert_eq!(sym(&f, "App").signature, "App({ title }: Props)");
        let f = parse(
            "lib/x.js",
            "export class Store { add(item) { this.items.push(item) } }",
        );
        assert_eq!(sym(&f, "Store").members[0].signature, "add(item)");
    }

    #[test]
    fn decorators_stay_out_of_class_signatures() {
        let f = parse("a.ts", "@Injectable()\nexport class Svc {}");
        assert_eq!(sym(&f, "Svc").signature, "class Svc");
    }

    #[test]
    fn records_line_ranges() {
        let f = parse("a.ts", "\n\nclass A {\n  m() {}\n}\n");
        let a = sym(&f, "A");
        assert_eq!(
            a.range,
            Range {
                start_line: 3,
                end_line: 5
            }
        );
        assert_eq!(a.members[0].line, 4);
    }

    #[test]
    fn extracts_decorators_on_classes_members_and_params() {
        let f = parse(
            "claims.controller.ts",
            r#"
@Controller('claims')
export class ClaimsController {
  constructor(private readonly claims: ClaimsService, @InjectRepository(Claim) private repo: Repository<Claim>) {}
  @Get(':id')
  find(@Param('id') id: string) { return this.claims.find(id); }
}
@Module({ providers: [A, B], controllers: [ClaimsController] })
export class ClaimsModule {}
"#,
        );
        let d = &f.facts.decorators;
        let find = |name: &str| d.iter().find(|s| s.decorator.name == name).unwrap();
        assert_eq!(find("Controller").target, DecoratorTarget::Class);
        assert_eq!(find("Controller").decorator.strings, ["claims"]);
        assert_eq!(find("Get").target, DecoratorTarget::Member("find".into()));
        assert_eq!(
            find("InjectRepository").target,
            DecoratorTarget::Param {
                member: "constructor".into(),
                param: "repo".into()
            }
        );
        assert_eq!(find("InjectRepository").decorator.idents, ["Claim"]);
        assert_eq!(
            find("Module").decorator.fields,
            [
                (
                    "providers".to_string(),
                    vec!["A".to_string(), "B".to_string()]
                ),
                (
                    "controllers".to_string(),
                    vec!["ClaimsController".to_string()]
                ),
            ]
        );
    }

    #[test]
    fn extracts_calls_with_their_receivers_and_literals() {
        let f = parse(
            "svc.ts",
            r#"
export class Svc {
  handle = async () => { this.events.emit('claim.created', 1); };
  run(id: string) {
    console.log(id);
    return this.repo.save(id).then(() => fetch(`${base}/claims/${id}`));
  }
}
const app = express();
app.use('/api', router);
"#,
        );
        let calls: Vec<_> = f
            .facts
            .calls
            .iter()
            .map(|c| (c.member.as_deref(), c.object.join("."), c.method.as_str()))
            .collect();
        assert_eq!(
            calls,
            [
                (Some("handle"), "this.events".into(), "emit"),
                (Some("run"), "this.repo".into(), "save"),
                (Some("run"), "".into(), "fetch"),
                (None, "".into(), "express"),
                (None, "app".into(), "use"),
            ]
        );
        let fetch = &f.facts.calls[2];
        assert_eq!(fetch.strings, ["*/claims/*"]);
        assert_eq!(fetch.symbol_id.as_deref(), Some("svc.ts#Svc"));
        assert_eq!(f.facts.calls[4].idents, ["router"]);
    }

    #[test]
    fn extracts_jsx_and_hooks_in_components() {
        let f = parse(
            "page.tsx",
            "export function Page() { const q = useClaims(); return <Layout><ClaimForm a={1}/><div/></Layout> }\nexport const Card = memo(() => <Badge />)",
        );
        let jsx: Vec<_> = f
            .facts
            .jsx
            .iter()
            .map(|j| (j.symbol_id.as_deref().unwrap(), j.name.as_str()))
            .collect();
        assert_eq!(
            jsx,
            [
                ("page.tsx#Page", "Layout"),
                ("page.tsx#Page", "ClaimForm"),
                ("page.tsx#Card", "Badge")
            ]
        );
        assert_eq!(f.facts.calls[0].method, "useClaims");
        assert!(f.symbols.iter().any(|s| s.name == "Card"));
    }
}
