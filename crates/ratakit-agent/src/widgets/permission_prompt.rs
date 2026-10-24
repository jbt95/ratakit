//! Permission prompt modal built on [`DialogState`](ratakit_core::DialogState).

use ratakit_core::{
    Component, DialogState, Event, FocusId, FocusRing, Keymap, Outcome, Revisioned, SemanticColor,
    Theme,
};
use ratakit_text::Highlighter;
use ratakit_widgets::{CodeBlock, CodeBlockState, DialogButton, DiffMode, DiffView, DiffViewState};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::text::{Line, Span};
use ratatui_core::widgets::{StatefulWidget, Widget};
use ratatui_widgets::block::Block;
use ratatui_widgets::borders::BorderType;
use ratatui_widgets::clear::Clear;
use ratatui_widgets::paragraph::Paragraph;

use crate::state::{
    PermissionIntent, PermissionOption, PermissionOptionId, PermissionPreview, PermissionRequest,
};

/// Focus, preview scroll state, and dialog selection for a permission request.
#[derive(Debug)]
pub struct PermissionPromptState {
    dialog: DialogState,
    code: CodeBlockState,
    diff: DiffViewState,
    preview: PermissionPreview,
    option_ids: Vec<PermissionOptionId>,
}

impl PermissionPromptState {
    /// Closed prompt with empty preview.
    pub fn new() -> Self {
        Self {
            dialog: DialogState::new(),
            code: CodeBlockState::new(),
            diff: DiffViewState::new(),
            preview: PermissionPreview::Command {
                text: String::new(),
            },
            option_ids: Vec::new(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.dialog.is_open()
    }

    pub fn selected_index(&self) -> usize {
        self.dialog.selected_button()
    }

    pub fn preview(&self) -> &PermissionPreview {
        &self.preview
    }

    /// Opens the prompt and loads preview/options from `request`.
    pub fn open_for(&mut self, request: &PermissionRequest) {
        self.load_request(request);
        self.dialog.open();
    }

    pub fn close(&mut self) {
        self.dialog.close();
    }

    pub fn open_with_trap(
        &mut self,
        request: &PermissionRequest,
        focus: &mut FocusRing,
        trap: FocusId,
    ) {
        self.load_request(request);
        self.dialog.open_with_trap(focus, trap);
    }

    pub fn close_with_trap(&mut self, focus: &mut FocusRing) {
        self.dialog.close_with_trap(focus);
    }

    fn load_request(&mut self, request: &PermissionRequest) {
        self.preview = request.preview();
        self.option_ids = request.options.iter().map(|opt| opt.id.clone()).collect();
        self.dialog.set_button_count(self.option_ids.len());
        if let PermissionPreview::Command { text } = &self.preview {
            self.code.set_source(text.clone());
        }
    }

    fn intent_from_dialog(&mut self, index: usize) -> PermissionIntent {
        let id = self
            .option_ids
            .get(index)
            .cloned()
            .unwrap_or_else(|| PermissionOptionId::new("unknown"));
        PermissionIntent::Chose(id)
    }
}

impl Default for PermissionPromptState {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for PermissionPromptState {
    type Intent = PermissionIntent;

    fn handle(&mut self, ev: &Event, keys: &Keymap) -> Outcome<Self::Intent> {
        if !self.dialog.is_open() {
            return Outcome::Ignored;
        }
        match self.dialog.handle(ev, keys) {
            Outcome::Intent(ratakit_core::DialogIntent::Confirmed(index)) => {
                Outcome::Intent(self.intent_from_dialog(index))
            }
            Outcome::Intent(ratakit_core::DialogIntent::Dismissed) => {
                Outcome::Intent(PermissionIntent::Dismissed)
            }
            Outcome::Ignored => Outcome::Ignored,
            Outcome::Consumed => Outcome::Consumed,
        }
    }

    fn revision(&self) -> u64 {
        Component::revision(&self.dialog) + self.code.revision() + self.diff.revision()
    }
}

/// Modal permission UI for a single [`PermissionRequest`].
pub struct PermissionPrompt<'a> {
    request: &'a PermissionRequest,
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
}

impl<'a> PermissionPrompt<'a> {
    pub fn new(
        request: &'a PermissionRequest,
        theme: &'a Theme,
        highlighter: &'a dyn Highlighter,
    ) -> Self {
        Self {
            request,
            theme,
            highlighter,
        }
    }
}

impl StatefulWidget for PermissionPrompt<'_> {
    type State = PermissionPromptState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.dialog.set_button_count(self.request.options.len());
        if !state.dialog.is_open() {
            return;
        }
        if state.preview != self.request.preview() {
            state.load_request(self.request);
        }

        let max_w = area.width.clamp(24, 72);
        let max_h = area.height.clamp(10, area.height);
        let dialog_area = centered_rect(max_w, max_h, area);
        Clear.render(dialog_area, buf);

        let title = self.request.tool_call.title.as_str();
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .style(palette_bg(self.theme).patch(border(self.theme)));
        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);

        let button_h = 1u16;
        let chunks =
            Layout::vertical([Constraint::Min(3), Constraint::Length(button_h)]).split(inner);

        PreviewRender {
            theme: self.theme,
            highlighter: self.highlighter,
            preview: &state.preview,
            request: self.request,
        }
        .render(chunks[0], buf, &mut state.code, &mut state.diff);

        let buttons: Vec<DialogButton> = self
            .request
            .options
            .iter()
            .map(dialog_button_for_option)
            .collect();
        render_option_buttons(
            self.theme,
            &buttons,
            state.dialog.selected_button(),
            chunks[1],
            buf,
        );
    }
}

/// Full-screen overlay that dims the backdrop and centers [`PermissionPrompt`].
pub struct PermissionPromptModal<'a> {
    prompt: PermissionPrompt<'a>,
}

impl<'a> PermissionPromptModal<'a> {
    pub fn new(prompt: PermissionPrompt<'a>) -> Self {
        Self { prompt }
    }
}

impl StatefulWidget for PermissionPromptModal<'_> {
    type State = PermissionPromptState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.dialog.is_open() {
            return;
        }
        let dim = Style::default().bg(ratatui_core::style::Color::Rgb(0, 0, 0));
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                cell.set_bg(dim.bg.unwrap_or(ratatui_core::style::Color::Black));
            }
        }
        StatefulWidget::render(self.prompt, area, buf, state);
    }
}

struct PreviewRender<'a> {
    theme: &'a Theme,
    highlighter: &'a dyn Highlighter,
    preview: &'a PermissionPreview,
    request: &'a PermissionRequest,
}

impl PreviewRender<'_> {
    fn render(
        self,
        area: Rect,
        buf: &mut Buffer,
        code: &mut CodeBlockState,
        diff: &mut DiffViewState,
    ) {
        match self.preview {
            PermissionPreview::Command { text: _ } => {
                if let Some(path) = self.request.tool_call.locations.first() {
                    code.detect_language_from_path(path, self.highlighter);
                }
                let widget = CodeBlock::new(self.theme, self.highlighter).line_numbers(true);
                StatefulWidget::render(widget, area, buf, code);
            }
            PermissionPreview::Diff(model) => {
                let min_width = 56;
                let widget = DiffView::new(model, self.theme, self.highlighter)
                    .mode(DiffMode::Auto { min_width })
                    .line_numbers(true);
                StatefulWidget::render(widget, area, buf, diff);
            }
        }
    }
}

fn dialog_button_for_option(option: &PermissionOption) -> DialogButton {
    let id = option.id.0.to_ascii_lowercase();
    let allow = id.contains("allow") || option.label.to_ascii_lowercase().contains("allow");
    if allow {
        DialogButton::primary(option.label.clone())
    } else {
        DialogButton::new(option.label.clone())
    }
}

fn render_option_buttons(
    theme: &Theme,
    buttons: &[DialogButton],
    selected: usize,
    area: Rect,
    buf: &mut Buffer,
) {
    if buttons.is_empty() || area.height == 0 {
        return;
    }

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, btn) in buttons.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let label = format!("[{}] {}", i + 1, btn.label);
        let style = if i == selected {
            cursor_style(theme)
        } else if btn.primary {
            accent(theme).add_modifier(Modifier::BOLD)
        } else {
            fg(theme)
        };
        spans.push(Span::styled(label, style));
    }

    let line = Line::from(spans).alignment(Alignment::Center);
    Paragraph::new(line).render(area, buf);
}

fn semantic_color(color: SemanticColor) -> Color {
    match color {
        SemanticColor::Rgb(rgb) => Color::Rgb(rgb.r, rgb.g, rgb.b),
        SemanticColor::Index(i) => Color::Indexed(i),
    }
}

fn fg(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.foreground))
}

fn accent(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.accent))
}

fn border(theme: &Theme) -> Style {
    Style::default().fg(semantic_color(theme.palette.border))
}

fn cursor_style(theme: &Theme) -> Style {
    Style::default()
        .fg(semantic_color(theme.palette.background))
        .bg(semantic_color(theme.palette.accent))
        .add_modifier(Modifier::BOLD)
}

fn palette_bg(theme: &Theme) -> Style {
    Style::default().bg(semantic_color(theme.palette.background))
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let horizontal = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width),
        Constraint::Fill(1),
    ])
    .flex(Flex::Center)
    .split(area);
    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .flex(Flex::Center)
    .split(horizontal[1]);
    vertical[1]
}
