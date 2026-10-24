use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

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

/// Assert an insta snapshot for a widget at the given size.
pub fn assert_widget_snapshot<W: Widget>(name: &str, widget: W, width: u16, height: u16) {
    let rendered = render_widget(widget, width, height);
    insta::assert_snapshot!(name, rendered);
}
