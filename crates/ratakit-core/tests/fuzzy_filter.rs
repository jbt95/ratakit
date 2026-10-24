#![cfg(feature = "fuzzy")]

use nucleo_matcher::pattern::{CaseMatching, Normalization};
use ratakit_core::{
    FuzzyDelimiterMode, FuzzyFilter, FuzzyOptions, Revisioned, normalize_match_indices,
};

#[test]
fn empty_pattern_matches_all_in_order() {
    let mut filter = FuzzyFilter::new();
    filter.set_pattern("");
    let items = ["alpha", "beta", "gamma"];
    let ranked = filter.rank(&items, 2);
    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].item_index, 0);
    assert_eq!(ranked[0].score, 0);
    assert_eq!(ranked[1].item_index, 1);
}

#[test]
fn ranks_by_score_descending() {
    let mut filter = FuzzyFilter::new();
    filter.set_pattern("foo bar");
    let paths = ["foo/bar", "bar/foo", "foobar", "quux"];
    let ranked = filter.rank(&paths, 10);
    let indices: Vec<usize> = ranked.iter().map(|m| m.item_index).collect();
    assert_eq!(indices, vec![0, 1, 2]);
    assert!(ranked[0].score >= ranked[1].score);
    assert!(ranked[1].score >= ranked[2].score);
}

#[test]
fn path_delimiter_mode_prefers_segment_start() {
    let mut default_filter = FuzzyFilter::new();
    default_filter.set_pattern("src");

    let mut path_filter = FuzzyFilter::with_options(FuzzyOptions {
        delimiter_mode: FuzzyDelimiterMode::Paths,
        ..FuzzyOptions::default()
    });
    path_filter.set_pattern("src");

    let items = ["project/src/lib.rs", "resource.txt"];
    let default_ranked = default_filter.rank(&items, 1);
    let path_ranked = path_filter.rank(&items, 1);

    assert_eq!(default_ranked[0].item_index, 0);
    assert_eq!(path_ranked[0].item_index, 0);
    assert!(
        path_ranked[0].score >= default_ranked[0].score,
        "path mode should not reduce the score for a path segment match"
    );
}

#[test]
fn prefix_literal_syntax() {
    let mut filter = FuzzyFilter::new();
    filter.set_pattern("^foo");
    let paths = ["foo/bar", "bar/foo", "foobar"];
    let ranked = filter.rank(&paths, 10);
    let indices: Vec<usize> = ranked.iter().map(|m| m.item_index).collect();
    assert_eq!(indices, vec![0, 2]);
}

#[test]
fn highlight_indices_sorted_and_unique() {
    let mut filter = FuzzyFilter::new();
    filter.set_pattern("fb");
    let mut indices = Vec::new();
    let score = filter
        .score_with_indices("foo/bar", &mut indices)
        .expect("should match");
    assert!(score > 0);
    assert!(!indices.is_empty());
    for window in indices.windows(2) {
        assert!(window[0] < window[1]);
    }
}

#[test]
fn normalize_match_indices_helper() {
    let mut indices = vec![3, 1, 2, 2, 0];
    normalize_match_indices(&mut indices);
    assert_eq!(indices, vec![0, 1, 2, 3]);
}

#[test]
fn revision_bumps_only_when_pattern_changes() {
    let mut filter = FuzzyFilter::new();
    assert_eq!(filter.revision(), 0);
    filter.set_pattern("a");
    assert_eq!(filter.revision(), 1);
    filter.set_pattern("a");
    assert_eq!(filter.revision(), 1);
    filter.set_pattern("ab");
    assert_eq!(filter.revision(), 2);
}

#[test]
fn smart_case_respects_uppercase_in_query() {
    let mut filter = FuzzyFilter::with_options(FuzzyOptions {
        case_matching: CaseMatching::Smart,
        normalization: Normalization::Never,
        delimiter_mode: FuzzyDelimiterMode::Default,
    });
    filter.set_pattern("Foo");
    assert!(filter.score("foo").is_none());
    assert!(filter.score("Foo").is_some());
}
