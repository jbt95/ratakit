// Derived from openai/codex (codex-rs/tui/src/render/highlight.rs).
// Copyright 2025 OpenAI. Licensed under the Apache License, Version 2.0.
// Modifications Copyright 2026 The ratakit Authors.

use std::path::Path;
use std::sync::OnceLock;

use ratakit_core::{Rgb, SemanticColor, SyntaxThemeId, Theme};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle, Theme as SyntectTheme};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use two_face::theme::{EmbeddedLazyThemeSet, EmbeddedThemeName};

use crate::highlight::{Highlighter, HlState, HlStateInner, LangId};
use crate::styled::{StyledSpan, TextModifier, TextStyle};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<EmbeddedLazyThemeSet> = OnceLock::new();
static ACTIVE_THEME: OnceLock<SyntectTheme> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
}

fn theme_set() -> &'static EmbeddedLazyThemeSet {
    THEME_SET.get_or_init(two_face::theme::extra)
}

fn active_theme() -> &'static SyntectTheme {
    ACTIVE_THEME.get_or_init(|| theme_set().get(EmbeddedThemeName::Base16MochaDark).clone())
}

/// Syntect-backed highlighter using bundled grammars and themes from two-face.
#[derive(Debug, Default, Clone, Copy)]
pub struct SyntectHighlighter;

impl SyntectHighlighter {
    /// Seed the active syntect theme from a ratakit [`Theme`] (first call wins).
    pub fn apply_theme(theme: &Theme) {
        let embedded = syntax_theme_id_to_embedded(theme.syntax);
        let _ = ACTIVE_THEME.set(theme_set().get(embedded).clone());
    }
}

impl Highlighter for SyntectHighlighter {
    fn highlight_line(&self, state: &mut HlState, line: &str) -> Vec<StyledSpan> {
        let HlStateInner::Syntect(inner) = &mut state.inner else {
            state.inner = HlStateInner::Empty;
            return vec![StyledSpan::raw(line)];
        };
        let regions = inner
            .lines
            .highlight_line(line, syntax_set())
            .unwrap_or_default();
        syntect_regions_to_spans(&regions)
    }

    fn language_for_path(&self, path: &Path) -> Option<LangId> {
        let set = syntax_set();
        let file_name = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        set.find_syntax_by_extension(ext)
            .or_else(|| set.find_syntax_by_name(&file_name))
            .map(|s| LangId(s.name.clone()))
    }

    fn language_for_name(&self, name: &str) -> Option<LangId> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return None;
        }
        let set = syntax_set();
        set.find_syntax_by_token(trimmed)
            .or_else(|| set.find_syntax_by_extension(trimmed))
            .map(|s| LangId(s.name.clone()))
    }

    fn new_state(&self, lang: LangId) -> HlState {
        let syntax = find_syntax(&lang.0);
        HlState {
            inner: HlStateInner::Syntect(Box::new(SyntectHlState {
                lines: HighlightLines::new(syntax, active_theme()),
            })),
        }
    }
}

/// Incremental syntect state for one code block.
pub struct SyntectHlState {
    pub(crate) lines: HighlightLines<'static>,
}

impl std::fmt::Debug for SyntectHlState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntectHlState").finish_non_exhaustive()
    }
}

fn find_syntax(name: &str) -> &'static SyntaxReference {
    let set = syntax_set();
    set.find_syntax_by_token(name)
        .or_else(|| set.find_syntax_by_extension(name))
        .unwrap_or_else(|| set.find_syntax_plain_text())
}

fn syntax_theme_id_to_embedded(_id: SyntaxThemeId) -> EmbeddedThemeName {
    EmbeddedThemeName::Base16MochaDark
}

fn syntect_regions_to_spans(regions: &[(SyntectStyle, &str)]) -> Vec<StyledSpan> {
    regions
        .iter()
        .filter(|(_, text)| !text.is_empty())
        .map(|(style, text)| StyledSpan::styled(*text, syntect_style_to_text(*style)))
        .collect()
}

fn syntect_style_to_text(style: SyntectStyle) -> TextStyle {
    let fg = style.foreground;
    let bg = style.background;
    let mut modifier = TextModifier::empty();
    if style.font_style.contains(FontStyle::BOLD) {
        modifier |= TextModifier::BOLD;
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        modifier |= TextModifier::ITALIC;
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        modifier |= TextModifier::UNDERLINE;
    }
    TextStyle {
        fg: Some(syntect_color_to_semantic(fg)),
        bg: if bg.a == 0 {
            None
        } else {
            Some(syntect_color_to_semantic(bg))
        },
        modifier,
    }
}

fn syntect_color_to_semantic(c: syntect::highlighting::Color) -> SemanticColor {
    SemanticColor::Rgb(Rgb {
        r: c.r,
        g: c.g,
        b: c.b,
    })
}

/// Detect language from the first line shebang (`#!/usr/bin/env python`).
pub fn language_from_shebang(first_line: &str) -> Option<LangId> {
    let trimmed = first_line.trim();
    if !trimmed.starts_with("#!") {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    for token in [
        "python", "ruby", "perl", "bash", "sh", "node", "lua", "rust",
    ] {
        if lower.contains(token) {
            return SyntectHighlighter.language_for_name(token);
        }
    }
    None
}
