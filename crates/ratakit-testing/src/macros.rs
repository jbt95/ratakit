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

/// Render a stateful widget with [`crate::render_stateful_widget`] and compare to an insta snapshot.
#[macro_export]
macro_rules! assert_stateful_widget_snapshot {
    ($name:expr, $widget:expr, $state:expr, $width:expr, $height:expr) => {{
        let rendered = $crate::render_stateful_widget($widget, $state, $width, $height);
        ::insta::assert_snapshot!($name, rendered);
    }};
}
