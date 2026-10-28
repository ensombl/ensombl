//! [`ArchitectureStore`] as `.crm/architecture.json` in the worktree, so the
//! team shares it through git and agents can read it.

use std::path::{Path, PathBuf};

use crate::application::ports::ArchitectureStore;
use crate::application::{Error, Result};
use crate::domain::system::overrides::ArchitectureFile;

pub struct JsonArchitecture;

fn path(root: &Path) -> PathBuf {
    root.join(".crm").join("architecture.json")
}

impl ArchitectureStore for JsonArchitecture {
    fn load(&self, root: &Path) -> ArchitectureFile {
        std::fs::read(path(root))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save(&self, root: &Path, file: &ArchitectureFile) -> Result<()> {
        let target = path(root);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut json =
            serde_json::to_vec_pretty(file).map_err(|e| Error::Storage(e.to_string()))?;
        json.push(b'\n');
        let tmp = target.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, target)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::system::overrides::ComponentOverride;

    #[test]
    fn saves_into_the_worktree_and_loads_back() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            JsonArchitecture.load(dir.path()),
            ArchitectureFile::default()
        );
        let mut file = ArchitectureFile::default();
        file.set_override(
            "x",
            ComponentOverride {
                description: Some("d".into()),
                ..Default::default()
            },
        );
        JsonArchitecture.save(dir.path(), &file).unwrap();
        assert!(dir.path().join(".crm/architecture.json").exists());
        assert_eq!(JsonArchitecture.load(dir.path()), file);
    }
}
