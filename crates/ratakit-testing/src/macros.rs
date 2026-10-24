/// Render a widget with [`crate::render_widget`] and compare to an insta snapshot.
///
/// ```ignore
/// assert_widget_snapshot!("name", widget, 80, 12);
/// ```
#[macro_export]
macro_rules! assert_widget_snapshot {
    ($name:expr, $widget:expr, $width:expr, $height:expr) => {{
        let rendered = $crate::render_widget($widget, $width, $height);
        ::insta::assert_snapshot!($name, rendered);
    }};
}
