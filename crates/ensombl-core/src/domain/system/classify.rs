//! Decides what a piece of code is: its role, layer and feature. Decorators
//! win, then naming conventions, then the folder it lives in.

use super::model::{Layer, Role};

/// Decorators that make a class a component, with the role they imply.
const ROLE_DECORATORS: &[(&str, Role)] = &[
    ("Controller", Role::Controller),
    ("Resolver", Role::Controller),
    ("Component", Role::UiComponent),
    ("WebSocketGateway", Role::Gateway),
    ("Processor", Role::Job),
    ("Injectable", Role::Service),
    ("Service", Role::Service),
    ("Repository", Role::Repository),
    ("EntityRepository", Role::Repository),
];

/// Decorators for data shapes and wiring, which aren't components.
pub const NON_COMPONENT_DECORATORS: &[&str] = &[
    "Entity",
    "Schema",
    "ObjectType",
    "InputType",
    "ArgsType",
    "Module",
    "Table",
    "Model",
];

const SUFFIXES: &[(&str, Role)] = &[
    ("Component", Role::UiComponent),
    ("Controller", Role::Controller),
    ("Resolver", Role::Controller),
    ("Handler", Role::Controller),
    ("Gateway", Role::Gateway),
    ("Processor", Role::Job),
    ("Consumer", Role::Job),
    ("Worker", Role::Job),
    ("Job", Role::Job),
    ("Cron", Role::Job),
    ("Scheduler", Role::Job),
    ("Listener", Role::Job),
    ("Subscriber", Role::Job),
    ("Guard", Role::Middleware),
    ("Interceptor", Role::Middleware),
    ("Middleware", Role::Middleware),
    ("Pipe", Role::Middleware),
    ("Filter", Role::Middleware),
    ("Router", Role::Router),
    ("Routes", Role::Router),
    ("Repository", Role::Repository),
    ("Repo", Role::Repository),
    ("Dao", Role::Repository),
    ("Client", Role::Client),
    ("Api", Role::Client),
    ("Adapter", Role::Client),
    ("Gateway", Role::Gateway),
    ("Service", Role::Service),
    ("UseCase", Role::Service),
    ("Interactor", Role::Service),
    ("Manager", Role::Service),
    ("Provider", Role::Service),
];

/// Names that describe data shapes rather than behavior.
const DATA_SUFFIXES: &[&str] = &[
    "Dto",
    "DTO",
    "Entity",
    "Input",
    "Args",
    "Response",
    "Request",
    "Props",
    "Options",
    "Config",
    "Exception",
    "Error",
    "Event",
    "Schema",
    "Model",
];

const FOLDERS: &[(&str, Role)] = &[
    ("controllers", Role::Controller),
    ("routes", Role::Router),
    ("handlers", Role::Controller),
    ("resolvers", Role::Controller),
    ("jobs", Role::Job),
    ("workers", Role::Job),
    ("middleware", Role::Middleware),
    ("middlewares", Role::Middleware),
    ("guards", Role::Middleware),
    ("services", Role::Service),
    ("usecases", Role::Service),
    ("use-cases", Role::Service),
    ("repositories", Role::Repository),
    ("repos", Role::Repository),
    ("db", Role::Repository),
    ("clients", Role::Client),
    ("hooks", Role::Hook),
    ("components", Role::UiComponent),
    ("pages", Role::Page),
];

/// Folder names that organize by kind, not by feature.
const GENERIC_FOLDERS: &[&str] = &[
    "src",
    "app",
    "lib",
    "libs",
    "server",
    "client",
    "api",
    "modules",
    "features",
    "feature",
    "domain",
    "services",
    "service",
    "controllers",
    "repositories",
    "components",
    "hooks",
    "pages",
    "routes",
    "utils",
    "util",
    "common",
    "shared",
    "core",
    "infra",
    "infrastructure",
    "application",
    "interface",
    "presentation",
    "data",
    "db",
    "models",
    "entities",
    "dto",
    "dtos",
    "packages",
    "apps",
    "jobs",
    "workers",
    "middleware",
    "middlewares",
    "guards",
    "clients",
    "handlers",
    "resolvers",
    "helpers",
    "types",
    "store",
    "stores",
    "views",
    "screens",
    "ui",
];

pub fn role_from_decorators<'a>(decorators: impl IntoIterator<Item = &'a str>) -> Option<Role> {
    let names: Vec<&str> = decorators.into_iter().collect();
    // `@Injectable()` on a `ClaimsRepository` is still a repository; let the name refine it.
    ROLE_DECORATORS
        .iter()
        .find(|(d, role)| *role != Role::Service && names.contains(d))
        .or_else(|| ROLE_DECORATORS.iter().find(|(d, _)| names.contains(d)))
        .map(|(_, role)| *role)
}

pub fn role_from_name(name: &str) -> Option<Role> {
    SUFFIXES
        .iter()
        .find(|(suffix, _)| name.len() > suffix.len() && name.ends_with(suffix))
        .map(|(_, role)| *role)
}

pub fn is_data_shape(name: &str) -> bool {
    DATA_SUFFIXES.iter().any(|s| name.ends_with(s))
}

pub fn role_from_path(file: &str) -> Option<Role> {
    let dirs: Vec<&str> = file.split('/').collect();
    let dirs = &dirs[..dirs.len().saturating_sub(1)];
    // Next.js route files.
    let base = file.rsplit('/').next().unwrap_or(file);
    let stem = base.split('.').next().unwrap_or(base);
    if dirs.contains(&"app") && stem == "page" {
        return Some(Role::Page);
    }
    if dirs.contains(&"app") && stem == "route" {
        return Some(Role::Controller);
    }
    if let Some(at) = dirs.iter().position(|d| *d == "pages")
        && dirs[at + 1..].first() == Some(&"api")
    {
        return Some(Role::Controller);
    }
    dirs.iter()
        .rev()
        .find_map(|d| FOLDERS.iter().find(|(f, _)| f == d).map(|(_, r)| *r))
}

/// A function is a hook if it's named like one, a UI component if it renders JSX.
pub fn function_role(name: &str, renders_jsx: bool) -> Option<Role> {
    let is_hook = name.len() > 3
        && name.starts_with("use")
        && name[3..].starts_with(|c: char| c.is_ascii_uppercase());
    if is_hook {
        Some(Role::Hook)
    } else if renders_jsx && name.starts_with(|c: char| c.is_ascii_uppercase()) {
        Some(Role::UiComponent)
    } else {
        None
    }
}

pub fn layer_of(role: Role) -> Layer {
    match role {
        Role::Controller
        | Role::Gateway
        | Role::Job
        | Role::Router
        | Role::Middleware
        | Role::Page
        | Role::UiComponent => Layer::Interface,
        Role::Hook | Role::Service | Role::Channel | Role::Other => Layer::Application,
        Role::Repository | Role::Client => Layer::Data,
        Role::External => Layer::External,
    }
}

/// Code under a `domain/` folder is domain code whatever its role.
pub fn layer_for(role: Role, file: &str) -> Layer {
    let in_domain = file.split('/').any(|d| d == "domain" || d == "entities");
    match role {
        Role::Other | Role::Service if in_domain => Layer::Domain,
        _ => layer_of(role),
    }
}

/// The first folder that names a feature (`src/modules/claims/...` → `claims`).
pub fn feature_of(file: &str) -> Option<String> {
    let dirs: Vec<&str> = file.split('/').collect();
    dirs[..dirs.len().saturating_sub(1)]
        .iter()
        .find(|d| !GENERIC_FOLDERS.contains(d) && !d.starts_with(['(', '[', '_', '.', '@']))
        .map(|d| d.to_string())
}

/// `ClaimsModule` → `claims`.
pub fn feature_from_module(name: &str) -> String {
    let base = name.strip_suffix("Module").unwrap_or(name);
    let mut out = String::new();
    for (i, c) in base.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('-');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorators_then_names_then_folders() {
        assert_eq!(role_from_decorators(["Controller"]), Some(Role::Controller));
        assert_eq!(
            role_from_decorators(["Injectable", "Repository"]),
            Some(Role::Repository)
        );
        assert_eq!(role_from_name("ClaimsRepository"), Some(Role::Repository));
        assert_eq!(role_from_name("Service"), None);
        assert_eq!(
            role_from_path("src/services/claims.ts"),
            Some(Role::Service)
        );
        assert_eq!(
            role_from_path("web/app/claims/[id]/page.tsx"),
            Some(Role::Page)
        );
        assert_eq!(
            role_from_path("web/pages/api/claims.ts"),
            Some(Role::Controller)
        );
        assert_eq!(function_role("useClaims", false), Some(Role::Hook));
        assert_eq!(function_role("user", false), None);
        assert_eq!(function_role("ClaimForm", true), Some(Role::UiComponent));
        assert_eq!(role_from_decorators(["Component"]), Some(Role::UiComponent));
        assert_eq!(role_from_path("apps/api/src/content/x.ts"), None);
        assert_eq!(
            layer_for(Role::Other, "src/claims/domain/claim.ts"),
            Layer::Domain
        );
        assert_eq!(
            layer_for(Role::Other, "src/claims/claim.ts"),
            Layer::Application
        );
    }

    #[test]
    fn features_come_from_meaningful_folders() {
        assert_eq!(
            feature_of("src/modules/claims/claims.service.ts").as_deref(),
            Some("claims")
        );
        assert_eq!(feature_of("src/services/claims.ts"), None);
        assert_eq!(
            feature_of("web/app/(auth)/login/page.tsx").as_deref(),
            Some("web")
        );
        assert_eq!(feature_from_module("ClaimLinesModule"), "claim-lines");
    }
}
