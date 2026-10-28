//! GitHub-style review comments on code lines. Each comment remembers the
//! text of its line so it can follow the code when lines shift, and is
//! marked outdated when that code is gone.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReviewComment {
    pub id: String,
    /// Repo-relative path.
    pub file: String,
    /// 1-based line the comment was written on (updated when re-anchored).
    pub line: u32,
    pub body: String,
    /// Trimmed text of the commented line when the comment was made.
    pub anchor: String,
    pub resolved: bool,
    /// Unix millis.
    #[ts(type = "number")]
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnchoredComment {
    pub comment: ReviewComment,
    /// Where the comment's code is now; `None` when it can't be found (outdated).
    pub current_line: Option<u32>,
}

/// How far from the original line we look for the anchor text.
const SEARCH_RADIUS: usize = 200;

/// Finds where a comment's anchor line is in `source` now, preferring the closest match.
pub fn reanchor(comment: &ReviewComment, source: &str) -> Option<u32> {
    let lines: Vec<&str> = source.lines().collect();
    let target = comment.anchor.trim();
    let original = (comment.line as usize).saturating_sub(1);
    if lines.get(original).is_some_and(|l| l.trim() == target) {
        return Some(comment.line);
    }
    if target.is_empty() {
        return None;
    }
    (1..=SEARCH_RADIUS).find_map(|d| {
        [original.checked_sub(d), Some(original + d)]
            .into_iter()
            .flatten()
            .find(|&i| lines.get(i).is_some_and(|l| l.trim() == target))
            .map(|i| i as u32 + 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comment(line: u32, anchor: &str) -> ReviewComment {
        ReviewComment {
            id: "c1".into(),
            file: "a.ts".into(),
            line,
            body: "rename this".into(),
            anchor: anchor.into(),
            resolved: false,
            created_at: 0,
        }
    }

    #[test]
    fn stays_put_when_the_line_is_unchanged() {
        assert_eq!(
            reanchor(&comment(2, "let b = 2"), "let a = 1\nlet b = 2\n"),
            Some(2)
        );
    }

    #[test]
    fn follows_the_code_when_lines_shift() {
        let source = "// header\n// more\nlet a = 1\n  let b = 2\n";
        assert_eq!(reanchor(&comment(2, "let b = 2"), source), Some(4));
    }

    #[test]
    fn prefers_the_closest_duplicate() {
        let source = "x\nreturn null\ny\nz\nw\nreturn null\n";
        assert_eq!(reanchor(&comment(5, "return null"), source), Some(6));
    }

    #[test]
    fn is_outdated_when_the_code_is_gone() {
        assert_eq!(reanchor(&comment(1, "deleted()"), "something else\n"), None);
    }
}
