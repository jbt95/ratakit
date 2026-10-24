//! Line-oriented diff model built on the [`similar`] crate.

use std::fmt;
use std::path::Path;

use similar::{ChangeTag, TextDiff};

/// A word- or character-level span inside a [`DiffLine`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntraSpan {
    /// Whether this span is an inline insertion or deletion highlight.
    pub emphasized: bool,
    pub text: String,
}

/// One line in a hunk (equal, removed, or added).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub tag: ChangeTag,
    /// 1-based line number in the old file, if present.
    pub old_line_no: Option<u32>,
    /// 1-based line number in the new file, if present.
    pub new_line_no: Option<u32>,
    /// Line text without trailing newline.
    pub content: String,
    /// Inline highlights (empty when unchanged or not computed).
    pub intra: Vec<IntraSpan>,
}

/// A contiguous group of changes with a unified-diff header range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
    /// When `true`, UIs may render a collapsed summary instead of the hunk body.
    pub collapsed: bool,
}

/// Diff for a single file (rename supported via distinct old/new paths).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub hunks: Vec<DiffHunk>,
}

/// Multi-file diff suitable for widgets and agent tool cards.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffModel {
    pub files: Vec<FileDiff>,
    source_texts: Option<(String, String)>,
}

/// Error while parsing a unified diff string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffParseError {
    /// Unexpected input at the given 1-based line number.
    UnexpectedLine { line_no: usize, detail: String },
    /// Hunk header could not be parsed.
    InvalidHunkHeader { line: String },
    /// No file headers were found.
    EmptyPatch,
}

impl fmt::Display for DiffParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedLine { line_no, detail } => {
                write!(f, "line {line_no}: {detail}")
            }
            Self::InvalidHunkHeader { line } => write!(f, "invalid hunk header: {line}"),
            Self::EmptyPatch => write!(f, "empty patch"),
        }
    }
}

impl std::error::Error for DiffParseError {}

impl DiffModel {
    /// Build a single-file diff from two text snapshots.
    pub fn from_texts(path: &str, old: &str, new: &str) -> Self {
        let file = file_diff_from_texts(Some(path), Some(path), old, new);
        Self {
            files: vec![file],
            source_texts: Some((old.to_string(), new.to_string())),
        }
    }

    /// Original old/new snapshots when this model was built with [`Self::from_texts`].
    ///
    /// ACP recordings use these source snapshots to preserve a tool diff through
    /// serde replay. Parsed unified patches do not retain original file contents.
    pub fn source_texts(&self) -> Option<(&str, &str)> {
        self.source_texts
            .as_ref()
            .map(|(old, new)| (old.as_str(), new.as_str()))
    }

    /// Parse a unified diff (git or plain) into a [`DiffModel`].
    ///
    /// # Errors
    ///
    /// Returns [`DiffParseError`] when headers, hunk markers, or line prefixes are invalid.
    pub fn from_unified(patch: &str) -> Result<Self, DiffParseError> {
        parse_unified_patch(patch)
    }

    /// Toggle collapse for one hunk (bumps internal layout when used from widgets).
    pub fn set_hunk_collapsed(&mut self, file_index: usize, hunk_index: usize, collapsed: bool) {
        if let Some(file) = self.files.get_mut(file_index)
            && let Some(hunk) = file.hunks.get_mut(hunk_index)
        {
            hunk.collapsed = collapsed;
        }
    }

    /// Flip collapsed state for a hunk.
    pub fn toggle_hunk_collapsed(&mut self, file_index: usize, hunk_index: usize) {
        if let Some(file) = self.files.get_mut(file_index)
            && let Some(hunk) = file.hunks.get_mut(hunk_index)
        {
            hunk.collapsed = !hunk.collapsed;
        }
    }
}

fn file_diff_from_texts(
    old_path: Option<&str>,
    new_path: Option<&str>,
    old: &str,
    new: &str,
) -> FileDiff {
    let diff = TextDiff::from_lines(old, new);
    let mut hunks = Vec::new();

    for ops in diff.grouped_ops(3) {
        let mut lines = Vec::new();
        let mut old_start = 0u32;
        let mut new_start = 0u32;
        let mut old_count = 0u32;
        let mut new_count = 0u32;
        let mut started = false;

        for op in ops {
            for change in diff.iter_inline_changes(&op) {
                if !started {
                    old_start = change.old_index().map_or(1, line_number);
                    new_start = change.new_index().map_or(1, line_number);
                    started = true;
                }

                let mut content = change
                    .iter_strings_lossy()
                    .map(|(_, s)| s.into_owned())
                    .collect::<String>();
                if content.ends_with('\n') {
                    content.pop();
                }

                let intra: Vec<IntraSpan> = change
                    .iter_strings_lossy()
                    .map(|(emphasized, text)| {
                        let text = text.strip_suffix('\n').unwrap_or(text.as_ref()).to_string();
                        IntraSpan { emphasized, text }
                    })
                    .collect();

                let old_no = change.old_index().map(line_number);
                let new_no = change.new_index().map(line_number);

                match change.tag() {
                    ChangeTag::Equal => {
                        old_count += 1;
                        new_count += 1;
                    }
                    ChangeTag::Delete => old_count += 1,
                    ChangeTag::Insert => new_count += 1,
                }

                lines.push(DiffLine {
                    tag: change.tag(),
                    old_line_no: old_no,
                    new_line_no: new_no,
                    content,
                    intra,
                });
            }
        }

        if lines.is_empty() {
            continue;
        }

        hunks.push(DiffHunk {
            old_start,
            old_count,
            new_start,
            new_count,
            lines,
            collapsed: false,
        });
    }

    FileDiff {
        old_path: old_path.map(str::to_string),
        new_path: new_path.map(str::to_string),
        hunks,
    }
}

fn parse_unified_patch(patch: &str) -> Result<DiffModel, DiffParseError> {
    let lines: Vec<&str> = patch.lines().collect();
    if lines.is_empty() {
        return Err(DiffParseError::EmptyPatch);
    }

    let mut files = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        while index < lines.len() && lines[index].is_empty() {
            index += 1;
        }
        if index >= lines.len() {
            break;
        }

        let line = lines[index];
        if line.starts_with("diff --git") {
            index += 1;
            while index < lines.len()
                && !lines[index].starts_with("--- ")
                && !lines[index].starts_with("diff --git")
            {
                index += 1;
            }
            continue;
        }

        if !line.starts_with("--- ") {
            return Err(DiffParseError::UnexpectedLine {
                line_no: index + 1,
                detail: format!("expected '---' file header, got {line:?}"),
            });
        }

        let old_path = parse_diff_path(line.strip_prefix("--- ").unwrap_or(line));
        index += 1;
        if index >= lines.len() || !lines[index].starts_with("+++ ") {
            return Err(DiffParseError::UnexpectedLine {
                line_no: index + 1,
                detail: "expected '+++' file header".to_string(),
            });
        }
        let new_path = parse_diff_path(lines[index].strip_prefix("+++ ").unwrap_or(lines[index]));
        index += 1;

        let mut hunks = Vec::new();
        while index < lines.len() {
            let hline = lines[index];
            if hline.starts_with("diff --git") || hline.starts_with("--- ") {
                break;
            }
            if hline.starts_with("@@ ") {
                let (hunk, next) = parse_hunk(&lines, index)?;
                hunks.push(hunk);
                index = next;
            } else if hline.starts_with("\\ No newline at end of file") || hline.is_empty() {
                index += 1;
            } else {
                return Err(DiffParseError::UnexpectedLine {
                    line_no: index + 1,
                    detail: format!("unexpected content {hline:?}"),
                });
            }
        }

        files.push(FileDiff {
            old_path: old_path.clone(),
            new_path: new_path.clone(),
            hunks,
        });
    }

    if files.is_empty() {
        return Err(DiffParseError::EmptyPatch);
    }

    Ok(DiffModel {
        files,
        source_texts: None,
    })
}

fn parse_diff_path(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed == "/dev/null" {
        return None;
    }
    let path = trimmed
        .strip_prefix("a/")
        .or_else(|| trimmed.strip_prefix("b/"))
        .unwrap_or(trimmed);
    Some(path.to_string())
}

fn parse_hunk(lines: &[&str], start: usize) -> Result<(DiffHunk, usize), DiffParseError> {
    let header = lines[start];
    let (old_start, old_count, new_start, new_count) = parse_hunk_header(header)?;

    let mut diff_lines = Vec::new();
    let mut index = start + 1;
    let mut old_line = old_start;
    let mut new_line = new_start;

    while index < lines.len() {
        let line = lines[index];
        if line.starts_with("@@ ") || line.starts_with("--- ") || line.starts_with("diff --git") {
            break;
        }
        if line.is_empty() {
            index += 1;
            continue;
        }
        if line.starts_with("\\ No newline at end of file") {
            index += 1;
            continue;
        }

        let (tag, payload) = match line.chars().next() {
            Some(' ') => (ChangeTag::Equal, line.get(1..).unwrap_or("")),
            Some('-') => (ChangeTag::Delete, line.get(1..).unwrap_or("")),
            Some('+') => (ChangeTag::Insert, line.get(1..).unwrap_or("")),
            _ => {
                return Err(DiffParseError::UnexpectedLine {
                    line_no: index + 1,
                    detail: format!("expected diff marker, got {line:?}"),
                });
            }
        };

        let (old_no, new_no) = match tag {
            ChangeTag::Equal => {
                let o = old_line;
                let n = new_line;
                old_line += 1;
                new_line += 1;
                (Some(o), Some(n))
            }
            ChangeTag::Delete => {
                let o = old_line;
                old_line += 1;
                (Some(o), None)
            }
            ChangeTag::Insert => {
                let n = new_line;
                new_line += 1;
                (None, Some(n))
            }
        };

        let intra = vec![IntraSpan {
            emphasized: false,
            text: payload.to_string(),
        }];

        diff_lines.push(DiffLine {
            tag,
            old_line_no: old_no,
            new_line_no: new_no,
            content: payload.to_string(),
            intra,
        });
        index += 1;
    }

    let hunk = DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        lines: diff_lines,
        collapsed: false,
    };
    Ok((hunk, index))
}

fn parse_hunk_header(header: &str) -> Result<(u32, u32, u32, u32), DiffParseError> {
    let trimmed = header.trim();
    if !trimmed.starts_with("@@ ") || !trimmed.ends_with(" @@") {
        return Err(DiffParseError::InvalidHunkHeader {
            line: header.to_string(),
        });
    }
    let inner = trimmed
        .strip_prefix("@@ ")
        .and_then(|s| s.strip_suffix(" @@"))
        .unwrap_or("");
    let parts: Vec<&str> = inner.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(DiffParseError::InvalidHunkHeader {
            line: header.to_string(),
        });
    }
    let (old_start, old_count) = parse_range(parts[0])?;
    let (new_start, new_count) = parse_range(parts[1])?;
    Ok((old_start, old_count, new_start, new_count))
}

fn line_number(index: usize) -> u32 {
    u32::try_from(index).map(|i| i + 1).unwrap_or(u32::MAX)
}

fn parse_range(token: &str) -> Result<(u32, u32), DiffParseError> {
    let token = token
        .strip_prefix('-')
        .or_else(|| token.strip_prefix('+'))
        .unwrap_or(token);
    let (start, count) = if let Some((s, c)) = token.split_once(',') {
        (s.parse::<u32>(), c.parse::<u32>())
    } else {
        (token.parse::<u32>(), Ok(1))
    };
    match (start, count) {
        (Ok(start), Ok(count)) => Ok((start.max(1), count)),
        _ => Err(DiffParseError::InvalidHunkHeader {
            line: token.to_string(),
        }),
    }
}

/// Recompute inline highlights for every line in the model (for patches parsed without them).
pub fn enrich_intra_line(model: &mut DiffModel) {
    for file in &mut model.files {
        for hunk in &mut file.hunks {
            for line in &mut hunk.lines {
                if line.tag == ChangeTag::Equal || line.intra.iter().any(|s| s.emphasized) {
                    continue;
                }
                line.intra = vec![IntraSpan {
                    emphasized: true,
                    text: line.content.clone(),
                }];
            }
        }
    }
}

/// Path used to pick a syntax highlighter language.
pub fn display_path(file: &FileDiff) -> Option<&str> {
    file.new_path.as_deref().or(file.old_path.as_deref())
}

/// Language lookup helper for widgets.
pub fn path_for_side(file: &FileDiff, side: DiffSide) -> Option<&Path> {
    let raw = match side {
        DiffSide::Old => file.old_path.as_deref(),
        DiffSide::New => file.new_path.as_deref(),
    };
    raw.map(Path::new)
}

/// Old or new side of a split diff view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffSide {
    Old,
    New,
}
