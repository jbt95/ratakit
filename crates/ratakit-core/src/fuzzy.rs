//! Fuzzy string matching for pickers and completion menus.
//!
//! This module is only available with the **`fuzzy`** feature, which enables the
//! [`nucleo-matcher`](https://crates.io/crates/nucleo-matcher) dependency (MPL-2.0).
//! Nucleo is used as a normal crate dependency; its source is not vendored or copied
//! into ratakit.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::Revisioned;

/// How path-like delimiters are scored when matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FuzzyDelimiterMode {
    /// Nucleo's default delimiter set (`/,:;|`).
    #[default]
    Default,
    /// Bonuses tuned for file paths (see [`Config::match_paths`]).
    Paths,
}

/// Case and normalization behaviour for parsed patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FuzzyOptions {
    pub case_matching: CaseMatching,
    pub normalization: Normalization,
    pub delimiter_mode: FuzzyDelimiterMode,
}

/// Stateful fuzzy matcher used by pickers (`FuzzyPicker`) and completion menus.
///
/// Holds a reusable [`Matcher`] scratch buffer (expensive to allocate) and the current query
/// [`Pattern`]. Call [`set_pattern`](Self::set_pattern) when the user types; use [`rank`](Self::rank)
/// or [`score`](Self::score) against candidate labels.
#[derive(Debug)]
pub struct FuzzyFilter {
    matcher: Matcher,
    pattern: Pattern,
    pattern_text: String,
    options: FuzzyOptions,
    revision: u64,
    utf32_buf: Vec<char>,
    indices_scratch: Vec<u32>,
}

impl Default for FuzzyFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl FuzzyFilter {
    /// Creates a matcher with [`FuzzyOptions::default`].
    pub fn new() -> Self {
        Self::with_options(FuzzyOptions::default())
    }

    /// Creates a matcher with the given options.
    pub fn with_options(options: FuzzyOptions) -> Self {
        let matcher = matcher_for_mode(options.delimiter_mode);
        Self {
            matcher,
            pattern: Pattern::default(),
            pattern_text: String::new(),
            options,
            revision: 0,
            utf32_buf: Vec::new(),
            indices_scratch: Vec::new(),
        }
    }

    /// Current query string (what the user typed).
    pub fn pattern(&self) -> &str {
        &self.pattern_text
    }

    /// Options used when parsing [`set_pattern`](Self::set_pattern) input.
    pub fn options(&self) -> &FuzzyOptions {
        &self.options
    }

    /// Updates matcher options. Re-parses the current pattern when it changes scoring behaviour.
    pub fn set_options(&mut self, options: FuzzyOptions) {
        if self.options == options {
            return;
        }
        self.options = options;
        self.matcher = matcher_for_mode(options.delimiter_mode);
        self.reparse_pattern();
        self.revision += 1;
    }

    /// Sets the fuzzy query. Supports nucleo pattern syntax (`^prefix`, `'substring`, `!negate`, …).
    pub fn set_pattern(&mut self, pattern: &str) {
        if self.pattern_text == pattern {
            return;
        }
        self.pattern_text = pattern.to_owned();
        self.reparse_pattern();
        self.revision += 1;
    }

    /// Returns `true` when the query is empty (all candidates match with score 0).
    pub fn matches_everything(&self) -> bool {
        self.pattern.atoms.is_empty()
    }

    /// Scores a single haystack. Returns `Some(0)` when the query is empty.
    pub fn score(&mut self, haystack: &str) -> Option<u32> {
        let hay = Utf32Str::new(haystack, &mut self.utf32_buf);
        self.pattern.score(hay, &mut self.matcher)
    }

    /// Scores a haystack and writes grapheme indices for highlighting into `indices`.
    ///
    /// The vector is cleared first. Indices are sorted and deduplicated (see
    /// [`normalize_match_indices`]).
    pub fn score_with_indices(&mut self, haystack: &str, indices: &mut Vec<u32>) -> Option<u32> {
        indices.clear();
        let hay = Utf32Str::new(haystack, &mut self.utf32_buf);
        let score = self.pattern.indices(hay, &mut self.matcher, indices)?;
        normalize_match_indices(indices);
        Some(score)
    }

    /// Convenience wrapper that uses an internal buffer for indices.
    pub fn score_with_indices_scratch(&mut self, haystack: &str) -> Option<(u32, Vec<u32>)> {
        self.indices_scratch.clear();
        let hay = Utf32Str::new(haystack, &mut self.utf32_buf);
        let score = self
            .pattern
            .indices(hay, &mut self.matcher, &mut self.indices_scratch)?;
        normalize_match_indices(&mut self.indices_scratch);
        let indices = self.indices_scratch.clone();
        Some((score, indices))
    }

    /// Ranks `items` by fuzzy score (descending) and returns at most `limit` matches.
    ///
    /// Each item is addressed by its index in the original slice. When the query is empty,
    /// the first `limit` items are returned in order, each with score `0`.
    pub fn rank(&mut self, items: &[impl AsRef<str>], limit: usize) -> Vec<FuzzyRankedMatch> {
        if limit == 0 {
            return Vec::new();
        }

        if self.matches_everything() {
            return items
                .iter()
                .enumerate()
                .take(limit)
                .map(|(index, _)| FuzzyRankedMatch {
                    item_index: index,
                    score: 0,
                    indices: Vec::new(),
                })
                .collect();
        }

        let mut ranked: Vec<FuzzyRankedMatch> = Vec::new();
        for (item_index, label) in items.iter().enumerate() {
            self.indices_scratch.clear();
            let hay = Utf32Str::new(label.as_ref(), &mut self.utf32_buf);
            if let Some(score) =
                self.pattern
                    .indices(hay, &mut self.matcher, &mut self.indices_scratch)
            {
                normalize_match_indices(&mut self.indices_scratch);
                ranked.push(FuzzyRankedMatch {
                    item_index,
                    score,
                    indices: self.indices_scratch.clone(),
                });
            }
        }

        ranked.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.item_index.cmp(&right.item_index))
        });
        ranked.truncate(limit);
        ranked
    }

    fn reparse_pattern(&mut self) {
        self.pattern = Pattern::parse(
            &self.pattern_text,
            self.options.case_matching,
            self.options.normalization,
        );
    }
}

impl Revisioned for FuzzyFilter {
    fn revision(&self) -> u64 {
        self.revision
    }
}

/// A single entry produced by [`FuzzyFilter::rank`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzyRankedMatch {
    /// Index into the slice passed to [`FuzzyFilter::rank`].
    pub item_index: usize,
    /// Nucleo ranking score (higher is better).
    pub score: u32,
    /// Grapheme indices in the matched label for highlighting.
    pub indices: Vec<u32>,
}

/// Sorts and deduplicates match indices (after [`Pattern::indices`]).
pub fn normalize_match_indices(indices: &mut Vec<u32>) {
    indices.sort_unstable();
    indices.dedup();
}

fn matcher_for_mode(mode: FuzzyDelimiterMode) -> Matcher {
    let config = match mode {
        FuzzyDelimiterMode::Default => Config::DEFAULT,
        FuzzyDelimiterMode::Paths => Config::DEFAULT.match_paths(),
    };
    Matcher::new(config)
}
