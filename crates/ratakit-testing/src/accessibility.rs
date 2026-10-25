use ratakit_core::Palette;

/// Assert that every measurable semantic text color meets the requested contrast ratio.
///
/// Indexed terminal colors cannot be measured portably and cause the assertion to fail, so this
/// helper is intended for palettes that promise a concrete contrast level.
pub fn assert_palette_text_contrast(palette: &Palette, minimum_ratio: f64) {
    assert!(
        minimum_ratio.is_finite() && minimum_ratio > 0.0,
        "minimum contrast ratio must be finite and positive"
    );
    let failures: Vec<_> = palette
        .text_contrast_ratios()
        .into_iter()
        .filter(|entry| entry.ratio.is_none_or(|ratio| ratio < minimum_ratio))
        .collect();
    assert!(
        failures.is_empty(),
        "palette does not meet {minimum_ratio}:1 text contrast: {failures:?}"
    );
}

#[cfg(test)]
mod tests {
    use ratakit_core::{Palette, SemanticColor};

    use super::assert_palette_text_contrast;

    #[test]
    fn accepts_high_contrast_palette() {
        assert_palette_text_contrast(&Palette::high_contrast_dark(), 7.0);
    }

    #[test]
    #[should_panic(expected = "palette does not meet 4.5:1 text contrast")]
    fn rejects_unmeasurable_and_low_contrast_colors() {
        let mut palette = Palette::dark_defaults();
        palette.muted = SemanticColor::Index(8);
        assert_palette_text_contrast(&palette, 4.5);
    }
}
