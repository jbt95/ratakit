/// Render a widget with [`crate::render_widget`] and compare to an insta snapshot.
///
/// ```ignore
/// assert_widget_snapshot!("name", widget, 80, 12);
/// assert_widget_snapshot!(widget, 80, 12, &mut state);
/// ```
#[macro_export]
macro_rules! assert_widget_snapshot {
    ($name:expr, $widget:expr, $width:expr, $height:expr) => {{
        let rendered = $crate::render_widget($widget, $width, $height);
        ::insta::assert_snapshot!($name, rendered);
    }};
    ($name:expr, $widget:expr, $width:expr, $height:expr, $state:expr) => {{
        let rendered = $crate::render_stateful($widget, $width, $height, $state);
        ::insta::assert_snapshot!($name, rendered);
    }};
}
