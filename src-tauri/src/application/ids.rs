use std::path::Path;

/// Stable, path-derived id (the same path gives the same id across runs).
pub fn id_for_path(path: &Path) -> String {
    let hash = blake3::hash(path.to_string_lossy().as_bytes());
    hash.to_hex()[..12].to_string()
}

/// Folder name of a path, used as a display name.
pub fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Turns a branch name into a filesystem-friendly folder name.
pub fn slug(branch: &str) -> String {
    let s: String = branch
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    s.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_stable_and_short() {
        let a = id_for_path(Path::new("/repo"));
        assert_eq!(a, id_for_path(Path::new("/repo")));
        assert_ne!(a, id_for_path(Path::new("/repo2")));
        assert_eq!(a.len(), 12);
    }

    #[test]
    fn slugs_branch_names() {
        assert_eq!(slug("feat/new thing"), "feat-new-thing");
        assert_eq!(slug("/x/"), "x");
    }
}
