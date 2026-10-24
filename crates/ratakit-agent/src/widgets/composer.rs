//! Composer widget: [`TextArea`] + [`CompletionMenu`], optional [`FuzzyPicker`] overlay.

use std::borrow::Cow;

use ratakit_core::{PickerDisplayMode, PickerItem, PickerSection, PickerState, Theme};
use ratakit_widgets::{CompletionMenu, CompletionMenuState, FuzzyPicker, TextArea, TextAreaState};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::widgets::StatefulWidget;

use crate::state::composer::{ComposerState, FileIndex};

/// One model row for the optional in-composer model picker overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposerModelItem {
    pub provider: String,
    pub model: String,
    pub detail: Option<String>,
}

impl PickerItem for ComposerModelItem {
    fn label(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{} / {}", self.provider, self.model))
    }

    fn detail(&self) -> Option<Cow<'_, str>> {
        self.detail.as_deref().map(Cow::Borrowed)
    }
}

/// Per-frame view state for [`Composer`] (scroll, completion layout, overlays).
#[derive(Debug)]
pub struct ComposerWidgetState {
    pub menu: CompletionMenuState,
    textarea: TextAreaState,
    pub model_picker: PickerState<ComposerModelItem>,
}

impl Default for ComposerWidgetState {
    fn default() -> Self {
        Self {
            menu: CompletionMenuState::default(),
            textarea: TextAreaState::new(),
            model_picker: PickerState::new(),
        }
    }
}

impl ComposerWidgetState {
    pub fn open_model_picker(&mut self, sections: Vec<PickerSection<ComposerModelItem>>) {
        self.model_picker.set_sections(sections);
        self.model_picker.open();
    }

    pub fn model_picker_open(&self) -> bool {
        self.model_picker.is_open()
    }
}

/// Agent composer backed by [`ComposerState`].
pub struct Composer<'a> {
    theme: &'a Theme,
    files: &'a dyn FileIndex,
    placeholder: Option<&'a str>,
    max_height: Option<u16>,
}

impl<'a> Composer<'a> {
    pub fn new(theme: &'a Theme, files: &'a dyn FileIndex) -> Self {
        Self {
            theme,
            files,
            placeholder: None,
            max_height: None,
        }
    }

    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = Some(text);
        self
    }

    pub fn max_height(mut self, height: u16) -> Self {
        self.max_height = Some(height);
        self
    }
}

impl StatefulWidget for Composer<'_> {
    type State = (ComposerState, ComposerWidgetState);

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (composer, view) = state;
        composer.sync_completions(self.files);
        view.textarea.buffer = composer.buffer.clone();

        let title = composer.block_title();
        let textarea_widget = TextArea::new(self.theme)
            .placeholder(self.placeholder.unwrap_or("Type a message…"))
            .max_height(self.max_height.unwrap_or(area.height))
            .block_title(&title)
            .bordered(!self.theme.screen_reader);
        textarea_widget.render(area, buf, &mut view.textarea);

        if composer.completion.is_open() {
            let menu = CompletionMenu::new(self.theme, &mut view.textarea, &composer.completion)
                .max_rows(6)
                .textarea_max_height(self.max_height)
                .textarea_bordered(!self.theme.screen_reader);
            menu.render(area, buf, &mut view.menu);
        }

        if view.model_picker.is_open() {
            let widget =
                FuzzyPicker::new("Model", self.theme).display_mode(PickerDisplayMode::Centered {
                    max_width: area.width.saturating_sub(4).max(20),
                    max_height: area.height.saturating_sub(2).max(8),
                });
            widget.render(area, buf, &mut view.model_picker);
        }
    }
}
