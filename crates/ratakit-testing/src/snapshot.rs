use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::buffer_text::buffer_to_string;

/// Render a [`Widget`] into a string snapshot (content + style appendix).
pub fn render_widget<W: Widget>(widget: W, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, width, height);
            widget.render(area, frame.buffer_mut());
        })
        .expect("draw");
    let buffer = terminal.backend().buffer();
    buffer_to_string(buffer)
}

/// Alias for [`render_widget`].
pub fn render_to_string<W: Widget>(widget: W, width: u16, height: u16) -> String {
    render_widget(widget, width, height)
}

/// Render a [`StatefulWidget`] into a string snapshot.
pub fn render_stateful<W: StatefulWidget>(
    widget: W,
    width: u16,
    height: u16,
    state: &mut W::State,
) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, width, height);
            widget.render(area, frame.buffer_mut(), state);
        })
        .expect("draw");
    let buffer = terminal.backend().buffer();
    buffer_to_string(buffer)
}

/// Assert an insta snapshot for a widget at the given size.
pub fn assert_widget_snapshot<W: Widget>(name: &str, widget: W, width: u16, height: u16) {
    let rendered = render_widget(widget, width, height);
    insta::assert_snapshot!(name, rendered);
}

/// Assert an insta snapshot for a [`StatefulWidget`].
pub fn assert_stateful_widget_snapshot<W: StatefulWidget>(
    name: &str,
    widget: W,
    width: u16,
    height: u16,
    state: &mut W::State,
) {
    let rendered = render_stateful(widget, width, height, state);
    insta::assert_snapshot!(name, rendered);
}

/// Alias for [`render_stateful`] with `(widget, state, width, height)` argument order.
pub fn render_stateful_widget<W: StatefulWidget>(
    widget: W,
    state: &mut W::State,
    width: u16,
    height: u16,
) -> String {
    render_stateful(widget, width, height, state)
}
