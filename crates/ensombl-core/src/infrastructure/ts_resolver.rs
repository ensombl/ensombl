//! [`ModuleResolver`] approximating TypeScript's resolution inside one
//! worktree: relative paths, tsconfig `baseUrl`/`paths` (nearest config),
//! and workspace packages by `package.json` name. `node_modules` is ignored;
//! only first-party code becomes part of the diagram.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ignore::WalkBuilder;
use serde_json::Value;

use crate::application::ports::{ModuleResolver, ModuleResolverFactory};

use super::fs_scanner::to_rel;

const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

pub struct TsResolverFactory;

impl ModuleResolverFactory for TsResolverFactory {
    fn for_root(&self, root: &Path) -> Box<dyn ModuleResolver> {
        Box::new(TsResolver::load(root))
    }
}

#[derive(Debug, Default, Clone)]
struct TsConfig {
    /// Repo-relative directory the config lives in.
    dir: String,
    /// Repo-relative `baseUrl`.
    base_url: Option<String>,
    /// `paths` with targets made repo-relative.
    paths: Vec<(String, Vec<String>)>,
}

pub struct TsResolver {
    root: PathBuf,
    /// Package name to repo-relative directory.
    packages: Vec<(String, String)>,
    /// tsconfigs, deepest directory first.
    configs: Vec<TsConfig>,
    cache: Mutex<HashMap<(String, String), Option<String>>>,
}

/// Joins and normalizes `.`/`..` segments of a `/`-separated relative path.
fn normalize(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    out.join("/")
}

fn join(dir: &str, rel: &str) -> String {
    if dir.is_empty() {
        normalize(rel)
    } else {
        normalize(&format!("{dir}/{rel}"))
    }
}

fn parent(path: &str) -> &str {
    path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

/// Strips `//` and `/* */` comments and trailing commas so tsconfig parses as JSON.
fn strip_jsonc(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                for n in chars.by_ref() {
                    if n == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut prev = ' ';
                for n in chars.by_ref() {
                    if prev == '*' && n == '/' {
                        break;
                    }
                    prev = n;
                }
            }
            _ => out.push(c),
        }
    }
    // Trailing commas before `}` or `]`.
    let mut cleaned = String::with_capacity(out.len());
    let bytes: Vec<char> = out.chars().collect();
    for (i, &c) in bytes.iter().enumerate() {
        if c == ',' {
            let next = bytes[i + 1..].iter().find(|c| !c.is_whitespace());
            if matches!(next, Some('}') | Some(']')) {
                continue;
            }
        }
        cleaned.push(c);
    }
    cleaned
}

fn read_json(path: &Path) -> Option<Value> {
    let src = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&strip_jsonc(&src)).ok()
}

impl TsResolver {
    pub fn load(root: &Path) -> Self {
        let mut packages = Vec::new();
        let mut configs = Vec::new();
        let walker = WalkBuilder::new(root)
            .hidden(true)
            .require_git(false)
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != "node_modules" && name != ".git" && name != "dist"
            })
            .build();
        for entry in walker.flatten() {
            let name = entry.file_name().to_string_lossy();
            let Some(rel) = to_rel(root, entry.path()) else {
                continue;
            };
            if name == "package.json" {
                if let Some(pkg) = read_json(entry.path())
                    .and_then(|j| j.get("name")?.as_str().map(str::to_string))
                {
                    packages.push((pkg, parent(&rel).to_string()));
                }
            } else if name == "tsconfig.json" || name == "jsconfig.json" {
                configs.push(Self::load_config(root, &rel, 0));
            }
        }
        // Longest names first so `@acme/ui-kit` wins over `@acme/ui`.
        packages.sort_by_key(|p| std::cmp::Reverse(p.0.len()));
        configs.sort_by_key(|c| std::cmp::Reverse(c.dir.len()));
        Self {
            root: root.into(),
            packages,
            configs,
            cache: Mutex::default(),
        }
    }

    /// Reads a tsconfig, following relative `extends` (child settings win).
    fn load_config(root: &Path, rel: &str, depth: usize) -> TsConfig {
        let dir = parent(rel).to_string();
        let mut config = TsConfig {
            dir: dir.clone(),
            ..Default::default()
        };
        let Some(json) = read_json(&root.join(rel)) else {
            return config;
        };

        if depth < 5
            && let Some(ext) = json
                .get("extends")
                .and_then(Value::as_str)
                .filter(|e| e.starts_with('.'))
        {
            let mut target = join(&dir, ext);
            if !target.ends_with(".json") {
                target.push_str(".json");
            }
            let base = Self::load_config(root, &target, depth + 1);
            config.base_url = base.base_url;
            config.paths = base.paths;
        }

        let options = json.get("compilerOptions");
        let base_url = options
            .and_then(|o| o.get("baseUrl"))
            .and_then(Value::as_str);
        if let Some(b) = base_url {
            config.base_url = Some(join(&dir, b));
        }
        if let Some(paths) = options
            .and_then(|o| o.get("paths"))
            .and_then(Value::as_object)
        {
            // `paths` are relative to baseUrl when set, otherwise to the config's directory.
            let anchor = config.base_url.clone().unwrap_or_else(|| dir.clone());
            config.paths = paths
                .iter()
                .map(|(pattern, targets)| {
                    let targets = targets
                        .as_array()
                        .map(|t| {
                            t.iter()
                                .filter_map(Value::as_str)
                                .map(|t| join(&anchor, t))
                                .collect()
                        })
                        .unwrap_or_default();
                    (pattern.clone(), targets)
                })
                .collect();
        }
        config
    }

    fn config_for(&self, from: &str) -> Option<&TsConfig> {
        self.configs
            .iter()
            .find(|c| c.dir.is_empty() || from == c.dir || from.starts_with(&format!("{}/", c.dir)))
    }

    fn is_file(&self, rel: &str) -> bool {
        !rel.is_empty() && self.root.join(rel).is_file()
    }

    /// Tries `rel` as written, with TS extensions (including `.js` → `.ts`), and as a directory index.
    fn try_file(&self, rel: &str) -> Option<String> {
        let rel = normalize(rel);
        if let Some((stem, ext)) = rel.rsplit_once('.') {
            if matches!(ext, "js" | "jsx" | "mjs" | "cjs") {
                for ts in ["ts", "tsx", "mts", "cts"] {
                    let candidate = format!("{stem}.{ts}");
                    if self.is_file(&candidate) {
                        return Some(candidate);
                    }
                }
            }
            if TS_EXTENSIONS.contains(&ext) && self.is_file(&rel) {
                return Some(rel);
            }
        }
        for ext in TS_EXTENSIONS {
            let candidate = format!("{rel}.{ext}");
            if self.is_file(&candidate) {
                return Some(candidate);
            }
        }
        for ext in TS_EXTENSIONS {
            let candidate = join(&rel, &format!("index.{ext}"));
            if self.is_file(&candidate) {
                return Some(candidate);
            }
        }
        None
    }

    fn resolve_paths(&self, config: &TsConfig, spec: &str) -> Option<String> {
        for (pattern, targets) in &config.paths {
            let captured = match pattern.split_once('*') {
                Some((prefix, suffix)) => spec
                    .strip_prefix(prefix)
                    .and_then(|rest| rest.strip_suffix(suffix))
                    .map(str::to_string),
                None => (pattern == spec).then(String::new),
            };
            let Some(captured) = captured else { continue };
            for target in targets {
                if let Some(found) = self.try_file(&target.replace('*', &captured)) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn resolve_package(&self, spec: &str) -> Option<String> {
        let (name, dir) = self
            .packages
            .iter()
            .find(|(name, _)| spec == name || spec.starts_with(&format!("{name}/")))?;
        let sub = spec[name.len()..].trim_start_matches('/');
        if !sub.is_empty() {
            return self
                .try_file(&join(dir, sub))
                .or_else(|| self.try_file(&join(dir, &format!("src/{sub}"))));
        }
        let manifest = read_json(&self.root.join(dir).join("package.json"));
        for field in ["source", "types", "typings", "module", "main"] {
            let Some(entry) = manifest
                .as_ref()
                .and_then(|m| m.get(field))
                .and_then(Value::as_str)
            else {
                continue;
            };
            // Built outputs usually mirror sources: dist/index.d.ts → src/index.ts.
            let entry = entry.trim_end_matches(".d.ts");
            for candidate in [
                entry.to_string(),
                entry
                    .replacen("dist/", "src/", 1)
                    .replacen("lib/", "src/", 1),
            ] {
                if let Some(found) = self.try_file(&join(dir, &candidate)) {
                    return Some(found);
                }
            }
        }
        self.try_file(&join(dir, "src/index"))
            .or_else(|| self.try_file(&join(dir, "index")))
    }

    fn resolve_uncached(&self, from: &str, spec: &str) -> Option<String> {
        if spec.starts_with('.') {
            return self.try_file(&join(parent(from), spec));
        }
        if let Some(config) = self.config_for(from) {
            if let Some(found) = self.resolve_paths(config, spec) {
                return Some(found);
            }
            if let Some(base) = &config.base_url
                && let Some(found) = self.try_file(&join(base, spec))
            {
                return Some(found);
            }
        }
        self.resolve_package(spec)
    }
}

impl ModuleResolver for TsResolver {
    fn resolve(&self, from: &str, specifier: &str) -> Option<String> {
        let key = (parent(from).to_string(), specifier.to_string());
        if let Some(hit) = self.cache.lock().unwrap().get(&key) {
            return hit.clone();
        }
        let result = self.resolve_uncached(from, specifier);
        self.cache.lock().unwrap().insert(key, result.clone());
        result
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

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(
            r,
            "tsconfig.base.json",
            r#"{ "compilerOptions": { "paths": { "@shared/*": ["./libs/shared/*"] } } }"#,
        );
        write(
            r,
            "apps/web/tsconfig.json",
            r#"{
              // comments and trailing commas are fine
              "extends": "../../tsconfig.base.json",
              "compilerOptions": { "paths": { "@/*": ["./src/*"], }, },
            }"#,
        );
        write(r, "apps/web/src/app.tsx", "");
        write(r, "apps/web/src/lib/utils.ts", "");
        write(r, "apps/web/src/components/index.ts", "");
        write(r, "apps/web/src/esm.ts", "");
        write(r, "libs/shared/log.ts", "");
        write(
            r,
            "packages/ui/package.json",
            r#"{ "name": "@acme/ui", "types": "dist/index.d.ts" }"#,
        );
        write(r, "packages/ui/src/index.ts", "");
        write(r, "packages/ui/src/button.tsx", "");
        dir
    }

    #[test]
    fn resolves_relative_paths_extensions_and_indexes() {
        let dir = fixture();
        let r = TsResolver::load(dir.path());
        let from = "apps/web/src/app.tsx";
        assert_eq!(
            r.resolve(from, "./lib/utils").as_deref(),
            Some("apps/web/src/lib/utils.ts")
        );
        assert_eq!(
            r.resolve(from, "./components").as_deref(),
            Some("apps/web/src/components/index.ts")
        );
        assert_eq!(
            r.resolve(from, "./esm.js").as_deref(),
            Some("apps/web/src/esm.ts")
        );
        assert_eq!(
            r.resolve("apps/web/src/lib/utils.ts", "../app").as_deref(),
            Some("apps/web/src/app.tsx")
        );
        assert_eq!(r.resolve(from, "./missing"), None);
    }

    #[test]
    fn resolves_tsconfig_paths_with_extends() {
        let dir = fixture();
        let r = TsResolver::load(dir.path());
        let from = "apps/web/src/app.tsx";
        assert_eq!(
            r.resolve(from, "@/lib/utils").as_deref(),
            Some("apps/web/src/lib/utils.ts")
        );
        assert_eq!(
            r.resolve(from, "@shared/log").as_deref(),
            None,
            "child paths replace inherited ones"
        );
        assert_eq!(
            r.resolve("libs/shared/x.ts", "@shared/log").as_deref(),
            None,
            "no config above libs/"
        );
    }

    #[test]
    fn paths_are_relative_to_base_url_when_set() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "tsconfig.json",
            r#"{ "compilerOptions": { "baseUrl": "src", "paths": { "~/*": ["*"] } } }"#,
        );
        write(dir.path(), "src/core/db.ts", "");
        let r = TsResolver::load(dir.path());
        assert_eq!(
            r.resolve("src/a.ts", "~/core/db").as_deref(),
            Some("src/core/db.ts")
        );
        assert_eq!(
            r.resolve("src/a.ts", "core/db").as_deref(),
            Some("src/core/db.ts")
        );
    }

    #[test]
    fn resolves_workspace_packages() {
        let dir = fixture();
        let r = TsResolver::load(dir.path());
        let from = "apps/web/src/app.tsx";
        assert_eq!(
            r.resolve(from, "@acme/ui").as_deref(),
            Some("packages/ui/src/index.ts")
        );
        assert_eq!(
            r.resolve(from, "@acme/ui/button").as_deref(),
            Some("packages/ui/src/button.tsx")
        );
        assert_eq!(r.resolve(from, "react"), None);
    }

    #[test]
    fn strips_jsonc() {
        let src = "{ \"a\": \"// not a comment\", /* x */ \"b\": [1, 2,], }";
        let v: Value = serde_json::from_str(&strip_jsonc(src)).unwrap();
        assert_eq!(v["a"], "// not a comment");
        assert_eq!(v["b"][1], 2);
    }
}
