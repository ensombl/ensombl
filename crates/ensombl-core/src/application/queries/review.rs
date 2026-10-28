//! Reading source for the peek view and listing review comments.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::cqrs::{Query, QueryHandler};
use crate::application::ports::{CommentStore, SessionRepository, Settings, SettingsRepository};
use crate::application::worktrees::{safe_join, worktree_root};
use crate::application::{Error, Result};
use crate::domain::review::{AnchoredComment, reanchor};
use crate::domain::workspace::WorktreeId;

/// Source text of a file in a worktree.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReadSource {
    pub worktree_id: WorktreeId,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SourceText {
    pub path: String,
    pub content: String,
}

/// All review comments in a worktree, re-anchored against the current code.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListComments {
    pub worktree_id: WorktreeId,
}

pub struct GetSettings;

impl Query for ReadSource {
    type Output = SourceText;
}
impl Query for ListComments {
    type Output = Vec<AnchoredComment>;
}
impl Query for GetSettings {
    type Output = Settings;
}

const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;

pub struct ReviewQueries {
    sessions: Arc<dyn SessionRepository>,
    comments: Arc<dyn CommentStore>,
    settings: Arc<dyn SettingsRepository>,
}

impl ReviewQueries {
    pub fn new(
        sessions: Arc<dyn SessionRepository>,
        comments: Arc<dyn CommentStore>,
        settings: Arc<dyn SettingsRepository>,
    ) -> Self {
        Self {
            sessions,
            comments,
            settings,
        }
    }
}

impl QueryHandler<ReadSource> for ReviewQueries {
    fn handle(&self, q: ReadSource) -> Result<SourceText> {
        let path = safe_join(
            &worktree_root(self.sessions.as_ref(), &q.worktree_id)?,
            &q.path,
        )?;
        if std::fs::metadata(&path)?.len() > MAX_SOURCE_BYTES {
            return Err(Error::Invalid(format!(
                "{} is too large to preview",
                q.path
            )));
        }
        Ok(SourceText {
            path: q.path,
            content: std::fs::read_to_string(path)?,
        })
    }
}

impl QueryHandler<ListComments> for ReviewQueries {
    fn handle(&self, q: ListComments) -> Result<Vec<AnchoredComment>> {
        let root = worktree_root(self.sessions.as_ref(), &q.worktree_id)?;
        let mut sources = std::collections::HashMap::new();
        let mut out: Vec<_> = self
            .comments
            .load(&q.worktree_id)
            .into_iter()
            .map(|comment| {
                let source = sources.entry(comment.file.clone()).or_insert_with(|| {
                    safe_join(&root, &comment.file)
                        .ok()
                        .and_then(|p| std::fs::read_to_string(p).ok())
                });
                let current_line = source.as_deref().and_then(|s| reanchor(&comment, s));
                AnchoredComment {
                    comment,
                    current_line,
                }
            })
            .collect();
        out.sort_by(|a, b| {
            a.comment
                .file
                .cmp(&b.comment.file)
                .then(a.current_line.cmp(&b.current_line))
        });
        Ok(out)
    }
}

impl QueryHandler<GetSettings> for ReviewQueries {
    fn handle(&self, _: GetSettings) -> Result<Settings> {
        Ok(self.settings.get())
    }
}
