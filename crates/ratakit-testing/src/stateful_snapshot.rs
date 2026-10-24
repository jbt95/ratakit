use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;

use crate::buffer_text::buffer_to_string;

/// Render a [`StatefulWidget`] into a string snapshot (content + style appendix).
pub fn render_stateful_widget<W: StatefulWidget>(
    widget: W,
    state: &mut W::State,
    width: u16,
    height: u16,
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

/// Assert an insta snapshot for a stateful widget at the given size.
pub fn assert_stateful_widget_snapshot<W: StatefulWidget>(
    name: &str,
    widget: W,
    state: &mut W::State,
    width: u16,
    height: u16,
) {
    let rendered = render_stateful_widget(widget, state, width, height);
    insta::assert_snapshot!(name, rendered);
}
