//! [`WorkflowStore`] reading `.ensombl/workflows/*.yml` from a repository.

use std::path::Path;

use crate::application::ports::WorkflowStore;
use crate::domain::workflow::{Workflow, parse};

pub struct FsWorkflows;

pub const WORKFLOW_DIR: &str = ".ensombl/workflows";

impl WorkflowStore for FsWorkflows {
    fn list(&self, root: &Path) -> Vec<(String, Result<Workflow, String>)> {
        let Ok(entries) = std::fs::read_dir(root.join(WORKFLOW_DIR)) else {
            return vec![];
        };
        let mut found: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("yml" | "yaml")))
            .filter_map(|p| {
                let stem = p.file_stem()?.to_string_lossy().into_owned();
                let parsed = std::fs::read_to_string(&p)
                    .map_err(|e| e.to_string())
                    .and_then(|text| parse(&text));
                Some((stem, parsed))
            })
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_workflow_files_and_reports_broken_ones() {
        let dir = tempfile::tempdir().unwrap();
        let wf = dir.path().join(WORKFLOW_DIR);
        std::fs::create_dir_all(&wf).unwrap();
        std::fs::write(
            wf.join("ship.yml"),
            "name: Ship\nsteps:\n  - {id: a, prompt: go}",
        )
        .unwrap();
        std::fs::write(wf.join("broken.yaml"), "name: [").unwrap();
        std::fs::write(wf.join("notes.md"), "ignored").unwrap();
        let found = FsWorkflows.list(dir.path());
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].0, "broken");
        assert!(found[0].1.is_err());
        assert_eq!(found[1].1.as_ref().unwrap().name, "Ship");
        assert!(FsWorkflows.list(&dir.path().join("nope")).is_empty());
    }
}
