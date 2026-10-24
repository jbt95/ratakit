use std::fmt;

/// Non-textual embedded item (mention, collapsed paste, attachment marker).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    pub kind: AtomKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomKind {
    Mention { label: String },
    PastedBlock { line_count: u32 },
    Attachment { label: String },
}

impl Atom {
    pub fn mention(label: impl Into<String>) -> Self {
        Self {
            kind: AtomKind::Mention {
                label: label.into(),
            },
        }
    }

    pub fn pasted_block(line_count: u32) -> Self {
        Self {
            kind: AtomKind::PastedBlock { line_count },
        }
    }

    pub fn attachment(label: impl Into<String>) -> Self {
        Self {
            kind: AtomKind::Attachment {
                label: label.into(),
            },
        }
    }

    pub fn display(&self) -> String {
        match &self.kind {
            AtomKind::Mention { label } => label.clone(),
            AtomKind::PastedBlock { line_count } => {
                format!("[Pasted {line_count} lines]")
            }
            AtomKind::Attachment { label } => format!("[{label}]"),
        }
    }

    pub fn display_width(&self) -> usize {
        unicode_width::UnicodeWidthStr::width(self.display().as_str())
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}
