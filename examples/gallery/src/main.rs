//! Tabbed gallery of ratakit generic widgets.
//!
//! Run interactively: `cargo run -p gallery`
//!
//! Headless CI smoke: `RATAKIT_GALLERY_HEADLESS=1 cargo run -p gallery --quiet`

use std::io::stdout;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use ratakit::agent::state::{
    Effort, ModeId, ModelCatalogEntry, ModelChoice, Plan, PlanEntry, PlanEntryId, PlanEntryStatus,
    Usage,
};
use ratakit::agent::widgets::{ModelPicker, ModelPickerState};
use ratakit::agent::{PlanView, UsageBar};
use ratakit::core::{
    ActionLabels, CheckboxState, Component, Event, KeyCode, Keymap, KeymapPreset, ListState,
    Motion, MotionMode, PickerSection, RadioGroupState, SelectState, SwitchState, TableState,
    TabsState, Theme, Toast, ToastLevel, ToastQueue, key_hints_from_keymap,
};
use ratakit::text::{DiffModel, LangId, MarkdownOptions, SyntectHighlighter};
use ratakit::widgets::{
    Badge, Button, ButtonSize, Card, Checkbox, CodeBlock, CodeBlockState, CommandPalette,
    ConfirmDialog, DialogState, DiffView, DiffViewState, FrameScheduler, KeyHints, Markdown,
    MarkdownState, Orientation, PickerItem, PickerState, Progress, RadioGroup, RowRenderer, Select,
    SelectOption, Separator, Skeleton, Spinner, StatusBar, StatusSegment, Switch,
    TabItem as WidgetTabItem, Table, TableColumn, Tabs, TextArea, TextAreaState, Toasts, Tree,
    Variant, VirtualList, enter_terminal, event_stream,
};
use ratakit::widgets::{TreeSource, TreeState};
use ratakit_testing::render_stateful;
use ratatui::Terminal;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::StatefulWidget;
use ratatui_crossterm::CrosstermBackend;

const GALLERY_TABS: [&str; 12] = [
    "Input", "List", "Tree", "Code", "Diff", "Markdown", "Chrome", "Data", "Command", "Models",
    "Agent", "UI",
];

fn gallery_markdown_sample() -> String {
    [
        "# ratakit gallery",
        "",
        "Streaming **markdown** with `code` and lists:",
        "",
        "- TextArea on tab 1",
        "- VirtualList on tab 2",
        "",
        "```rust",
        "println!(\"hello\");",
        "```",
    ]
    .join("\n")
}

struct DemoRows {
    labels: Vec<String>,
}

impl DemoRows {
    fn new(labels: &[&str]) -> Self {
        Self {
            labels: labels.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

impl ratakit::core::ItemHeights for DemoRows {
    fn len(&self) -> usize {
        self.labels.len()
    }

    fn height(&self, index: usize, width: u16) -> u16 {
        let label = &self.labels[index];
        let cols = width.max(1) as usize;
        label.len().div_ceil(cols).max(1) as u16
    }
}

impl RowRenderer for DemoRows {
    fn render_row(
        &self,
        index: usize,
        area: Rect,
        buf: &mut ratatui::buffer::Buffer,
        skip_top: u16,
    ) {
        use ratatui::style::Style;
        use ratatui::text::Line;
        use ratatui::widgets::Paragraph;

        let text = &self.labels[index];
        let cols = area.width.max(1) as usize;
        let lines: Vec<Line> = text
            .chars()
            .collect::<Vec<_>>()
            .chunks(cols)
            .map(|chunk| Line::from(chunk.iter().collect::<String>()))
            .collect();
        let start = skip_top.min(lines.len() as u16) as usize;
        let visible = lines.into_iter().skip(start).take(area.height as usize);
        ratatui::widgets::Widget::render(
            Paragraph::new(visible.collect::<Vec<_>>()).style(Style::default()),
            area,
            buf,
        );
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum GalleryNode {
    Src,
    MainRs,
    LibRs,
}

#[derive(Clone)]
struct GalleryFile {
    path: String,
    detail: Option<String>,
}

impl PickerItem for GalleryFile {
    fn label(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.path)
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        self.detail.as_deref().map(std::borrow::Cow::Borrowed)
    }
}

struct GalleryFileTree;

impl TreeSource<GalleryNode> for GalleryFileTree {
    fn child_count(&self, parent: &[GalleryNode]) -> usize {
        match parent {
            [] => 1,
            [GalleryNode::Src] => 2,
            _ => 0,
        }
    }

    fn child_at(&self, parent: &[GalleryNode], index: usize) -> GalleryNode {
        match parent {
            [] => GalleryNode::Src,
            [GalleryNode::Src] => match index {
                0 => GalleryNode::MainRs,
                _ => GalleryNode::LibRs,
            },
            _ => GalleryNode::MainRs,
        }
    }

    fn label(&self, path: &[GalleryNode]) -> &str {
        match path.last() {
            Some(GalleryNode::Src) => "src",
            Some(GalleryNode::MainRs) => "main.rs",
            Some(GalleryNode::LibRs) => "lib.rs",
            None => "",
        }
    }

    fn is_branch(&self, path: &[GalleryNode]) -> bool {
        matches!(path, [] | [GalleryNode::Src])
    }

    fn children_loaded(&self, _path: &[GalleryNode]) -> bool {
        true
    }
}

struct App {
    theme: Theme,
    keys: Keymap,
    gallery_tab: TabsState,
    textarea: TextAreaState,
    list_rows: DemoRows,
    list_state: ListState,
    file_tree: GalleryFileTree,
    tree_state: TreeState<GalleryNode>,
    code: CodeBlockState,
    diff_model: DiffModel,
    diff_state: DiffViewState,
    markdown: MarkdownState,
    motion: Motion,
    toasts: ToastQueue,
    dialog: DialogState,
    inner_tabs: TabsState,
    table: TableState,
    picker: PickerState<GalleryFile>,
    model_picker: ModelPickerState,
    checkbox: CheckboxState,
    switch: SwitchState,
    radio_group: RadioGroupState,
    select: SelectState,
    select_options: Vec<SelectOption>,
    hint_labels: ActionLabels,
    highlighter: SyntectHighlighter,
    demo_usage: Usage,
    demo_mode: ModeId,
    demo_plan: Plan,
    plan_list: ListState,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        let mut gallery_tab = TabsState::new();
        gallery_tab.set_tab_count(GALLERY_TABS.len());

        let mut inner_tabs = TabsState::new();
        inner_tabs.set_tab_count(3);

        let mut table = TableState::new();
        table.set_dimensions(4, 2);

        let mut textarea = TextAreaState::new();
        textarea
            .buffer
            .insert_str("Edit me — gallery TextArea demo");

        let mut code = CodeBlockState::new();
        code.set_source("fn main() {\n    println!(\"hello\");\n}\n");
        code.set_language(LangId("Rust".to_string()));

        let diff_model = DiffModel::from_texts(
            "greet.rs",
            "fn greet() {\n    println!(\"hi\");\n}\n",
            "fn greet() {\n    println!(\"hello\");\n}\n",
        );

        let mut markdown = MarkdownState::new(MarkdownOptions::default());
        markdown.push_delta(&gallery_markdown_sample());
        markdown.finish();

        let mut toasts = ToastQueue::new();
        toasts.push(Toast::persistent(
            "Welcome to the gallery",
            ToastLevel::Info,
        ));

        let theme = Theme::dark();
        SyntectHighlighter::apply_theme(&theme);

        let mut hint_labels = ActionLabels::new();
        hint_labels.insert("focus.next", "next");
        hint_labels.insert("focus.prev", "prev");
        hint_labels.insert("app.quit", "quit");

        let demo_plan = Plan {
            entries: vec![
                PlanEntry {
                    id: PlanEntryId::new("a"),
                    title: "Explore gallery".to_string(),
                    status: PlanEntryStatus::Completed,
                },
                PlanEntry {
                    id: PlanEntryId::new("b"),
                    title: "Try UsageBar and PlanView".to_string(),
                    status: PlanEntryStatus::InProgress,
                },
                PlanEntry {
                    id: PlanEntryId::new("c"),
                    title: "Ship milestone A7".to_string(),
                    status: PlanEntryStatus::Pending,
                },
            ],
        };

        Self {
            theme,
            keys: Keymap::from_preset(KeymapPreset::Default),
            gallery_tab,
            textarea,
            list_rows: DemoRows::new(&[
                "alpha",
                "beta (two lines of text for height)",
                "gamma",
                "delta",
                "epsilon",
                "zeta",
                "eta",
            ]),
            list_state: ListState::new(),
            file_tree: GalleryFileTree,
            tree_state: {
                let mut tree = TreeState::new();
                tree.open(vec![GalleryNode::Src]);
                tree
            },
            code,
            diff_model,
            diff_state: DiffViewState::new(),
            markdown,
            motion: Motion::new(),
            toasts,
            dialog: DialogState::new(),
            inner_tabs,
            table,
            picker: {
                let mut picker = PickerState::new();
                picker.set_sections(vec![
                    PickerSection {
                        title: "src".into(),
                        items: vec![
                            GalleryFile {
                                path: "src/main.rs".into(),
                                detail: Some("fn main() { … }".into()),
                            },
                            GalleryFile {
                                path: "src/lib.rs".into(),
                                detail: Some("pub mod widgets;".into()),
                            },
                        ],
                    },
                    PickerSection {
                        title: "docs".into(),
                        items: vec![GalleryFile {
                            path: "docs/DISENO.md".into(),
                            detail: Some("Design document".into()),
                        }],
                    },
                ]);
                picker.open();
                picker
            },
            model_picker: {
                let mut picker = ModelPickerState::new();
                picker.set_catalog(vec![
                    ModelCatalogEntry::new("OpenAI", "gpt-4.1")
                        .with_detail("General reasoning")
                        .with_efforts(vec![Effort::Medium, Effort::High]),
                    ModelCatalogEntry::new("OpenAI", "gpt-4.1-mini")
                        .with_efforts(vec![Effort::Low]),
                    ModelCatalogEntry::new("Anthropic", "claude-sonnet-4")
                        .with_detail("Balanced")
                        .with_efforts(vec![Effort::Low, Effort::Medium, Effort::High]),
                ]);
                picker.set_current(Some(ModelChoice {
                    provider: "OpenAI".into(),
                    model: "gpt-4.1".into(),
                    effort: Some(Effort::Medium),
                }));
                picker.open();
                picker
            },
            checkbox: CheckboxState::new(true),
            switch: SwitchState::new(false),
            radio_group: {
                let mut state = RadioGroupState::new();
                state.set_option_count(3);
                state.select(1);
                state
            },
            select: {
                let mut state = SelectState::new();
                state.set_option_count(3);
                state.select(0);
                state
            },
            select_options: vec![
                SelectOption::new("System"),
                SelectOption::new("Dark"),
                SelectOption::new("Light"),
            ],
            hint_labels,
            highlighter: SyntectHighlighter,
            demo_usage: Usage {
                input_tokens: 42_000,
                output_tokens: 1_800,
                cached_tokens: 9_500,
                context_used: Some(0.55),
                cost_usd: Some(0.12),
            },
            demo_mode: ModeId::new("ask"),
            demo_plan,
            plan_list: ListState::new(),
            should_quit: false,
        }
    }

    fn handle(&mut self, ev: &Event) {
        if self.dialog.is_open() {
            if let ratakit::core::Outcome::Intent(intent) = self.dialog.handle(ev, &self.keys) {
                match intent {
                    ratakit::core::DialogIntent::Confirmed(_)
                    | ratakit::core::DialogIntent::Dismissed => {
                        self.dialog.close();
                    }
                }
                return;
            }
        }

        if let Event::Key(key) = ev {
            if key.kind == ratakit::core::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                self.should_quit = true;
                return;
            }
            if key.kind == ratakit::core::KeyEventKind::Press && key.code == KeyCode::Char('m') {
                self.dialog.set_button_count(2);
                self.dialog.open();
                return;
            }
        }

        match self.gallery_tab.selected() {
            0 => {
                let _ = self.textarea.handle(ev, &self.keys);
            }
            1 => {
                let _ = self.list_state.handle(ev, &self.keys);
            }
            2 => {
                let _ = self
                    .tree_state
                    .handle_with_source(ev, &self.keys, &self.file_tree);
            }
            8 => {
                let _ = self.picker.handle(ev, &self.keys);
            }
            9 => {
                let _ = self.model_picker.handle(ev, &self.keys);
            }
            10 => {
                let _ = self.plan_list.handle(ev, &self.keys);
            }
            11 => {
                if let Event::Key(key) = ev {
                    if key.kind == ratakit::core::KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('c') => {
                                self.checkbox.toggle();
                            }
                            KeyCode::Char('s') => {
                                self.switch.toggle();
                            }
                            KeyCode::Char('e') if !self.select.is_open() => {
                                self.select.open();
                            }
                            _ if self.select.is_open() => {
                                let _ = self.select.handle(ev, &self.keys);
                            }
                            _ => {
                                let _ = self.radio_group.handle(ev, &self.keys);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        if let Event::Key(key) = ev {
            if key.kind == ratakit::core::KeyEventKind::Press {
                match key.code {
                    KeyCode::Tab => {
                        let next = (self.gallery_tab.selected() + 1) % GALLERY_TABS.len();
                        self.gallery_tab.select(next);
                    }
                    KeyCode::Char('0') => {
                        self.gallery_tab.select(9);
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        self.gallery_tab.select(10);
                    }
                    KeyCode::Char('u') | KeyCode::Char('U') => {
                        self.gallery_tab.select(11);
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let idx = (c as u8 - b'1') as usize;
                        if idx < GALLERY_TABS.len() {
                            self.gallery_tab.select(idx);
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Event::Tick(_) = ev {
            let _ = self.motion.handle(ev, &self.keys);
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(area);

        let tab_items: Vec<WidgetTabItem> = GALLERY_TABS
            .iter()
            .enumerate()
            .map(|(i, title)| {
                let mut item = WidgetTabItem::new(*title);
                if i == self.gallery_tab.selected() {
                    item = item.with_active(true);
                }
                item
            })
            .collect();
        let tabs = Tabs::new(&self.theme, &tab_items);
        tabs.render(chunks[0], frame.buffer_mut(), &mut self.gallery_tab);

        match self.gallery_tab.selected() {
            0 => {
                let widget = TextArea::new(&self.theme)
                    .placeholder("Type here…")
                    .block_title("TextArea");
                widget.render(chunks[1], frame.buffer_mut(), &mut self.textarea);
            }
            1 => {
                let widget = VirtualList::new(&self.list_rows, &self.theme).scrollbar(true);
                self.list_state
                    .prepare_layout(&self.list_rows, chunks[1].width, chunks[1].height);
                widget.render(chunks[1], frame.buffer_mut(), &mut self.list_state);
            }
            2 => {
                let widget = Tree::new(&self.file_tree, &self.theme).scrollbar(true);
                widget.render(chunks[1], frame.buffer_mut(), &mut self.tree_state);
            }
            3 => {
                let widget = CodeBlock::new(&self.theme, &self.highlighter);
                widget.render(chunks[1], frame.buffer_mut(), &mut self.code);
            }
            4 => {
                let widget = DiffView::new(&self.diff_model, &self.theme, &self.highlighter);
                widget.render(chunks[1], frame.buffer_mut(), &mut self.diff_state);
            }
            5 => {
                let widget = Markdown::new(&self.theme, &self.highlighter);
                widget.render(chunks[1], frame.buffer_mut(), &mut self.markdown);
            }
            6 => {
                let inner = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(1),
                    ])
                    .split(chunks[1]);
                Spinner::new(&self.theme).label("Loading").render(
                    inner[0],
                    frame.buffer_mut(),
                    &mut self.motion,
                );
                let segments = [
                    StatusSegment::left("gallery", 12),
                    StatusSegment::center("tab chrome", 10),
                    StatusSegment::right("ok", 4),
                ];
                ratatui::widgets::Widget::render(
                    StatusBar::new(&self.theme, &segments),
                    inner[1],
                    frame.buffer_mut(),
                );
                let hints = key_hints_from_keymap(&self.keys, &self.hint_labels, 4);
                ratatui::widgets::Widget::render(
                    KeyHints::new(&self.theme, &hints),
                    inner[2],
                    frame.buffer_mut(),
                );
                Toasts::new(&self.theme).render(inner[3], frame.buffer_mut(), &mut self.toasts);
            }
            7 => {
                let inner = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(3)])
                    .split(chunks[1]);
                let items = [
                    WidgetTabItem::new("One"),
                    WidgetTabItem::new("Two").with_badge(3),
                    WidgetTabItem::new("Three"),
                ];
                Tabs::new(&self.theme, &items).render(
                    inner[0],
                    frame.buffer_mut(),
                    &mut self.inner_tabs,
                );
                let columns = [
                    TableColumn::new(
                        "Name",
                        vec![
                            "alpha".into(),
                            "beta".into(),
                            "gamma".into(),
                            "delta".into(),
                        ],
                    ),
                    TableColumn::new(
                        "Size",
                        vec!["1".into(), "22".into(), "333".into(), "4".into()],
                    ),
                ];
                Table::new(&self.theme, &columns).render(
                    inner[1],
                    frame.buffer_mut(),
                    &mut self.table,
                );
            }
            8 => {
                let widget = CommandPalette::new("Open file", &self.theme).preview(true);
                widget.render(chunks[1], frame.buffer_mut(), &mut self.picker);
            }
            9 => {
                let widget = ModelPicker::new(&self.theme);
                widget.render(chunks[1], frame.buffer_mut(), &mut self.model_picker);
            }
            10 => {
                let inner = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Min(3)])
                    .split(chunks[1]);
                ratatui::widgets::Widget::render(
                    UsageBar::new(&self.demo_usage, &self.demo_mode, &self.theme)
                        .git_branch(Some("gallery")),
                    inner[0],
                    frame.buffer_mut(),
                );
                let plan = PlanView::new(&self.demo_plan, &self.theme).scrollbar(true);
                plan.render(inner[1], frame.buffer_mut(), &mut self.plan_list);
            }
            11 => self.draw_components(chunks[1], frame.buffer_mut()),
            _ => {}
        }

        let footer = if self.gallery_tab.selected() == 11 {
            "c checkbox · s switch · ↑/↓ radio · e select · Enter/↑/↓/Esc menu".to_owned()
        } else {
            format!(
                "Tab/1-9/0/a/u: sections · m: modal · q: quit · motion: {}",
                match self.theme.motion {
                    MotionMode::Full => "full",
                    MotionMode::Reduced => "reduced",
                }
            )
        };
        use ratatui::style::Style;
        use ratatui::widgets::Paragraph;
        ratatui::widgets::Widget::render(
            Paragraph::new(footer).style(Style::default()),
            chunks[2],
            frame.buffer_mut(),
        );

        if self.dialog.is_open() {
            let dialog = ConfirmDialog::new(&self.theme, "Gallery modal", "Press OK or Cancel.");
            dialog.render(area, frame.buffer_mut(), &mut self.dialog);
        }
    }
}

impl App {
    fn draw_components(&mut self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let sections = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(5),
        ])
        .split(area);
        let button_areas = Layout::horizontal([
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(18),
            Constraint::Length(16),
            Constraint::Length(12),
            Constraint::Min(1),
        ])
        .split(sections[0]);
        ratatui::widgets::Widget::render(Button::new(&self.theme, "Save"), button_areas[0], buf);
        ratatui::widgets::Widget::render(
            Button::new(&self.theme, "Cancel").variant(Variant::Secondary),
            button_areas[1],
            buf,
        );
        ratatui::widgets::Widget::render(
            Button::new(&self.theme, "Delete").variant(Variant::Destructive),
            button_areas[2],
            buf,
        );
        ratatui::widgets::Widget::render(
            Button::new(&self.theme, "Focused")
                .variant(Variant::Outline)
                .focused(true),
            button_areas[3],
            buf,
        );
        ratatui::widgets::Widget::render(
            Button::new(&self.theme, "Disabled")
                .size(ButtonSize::Small)
                .disabled(true),
            button_areas[4],
            buf,
        );

        let badge_areas = Layout::horizontal([
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(16),
            Constraint::Length(14),
            Constraint::Min(1),
        ])
        .split(sections[1]);
        ratatui::widgets::Widget::render(Badge::new(&self.theme, "Default"), badge_areas[0], buf);
        ratatui::widgets::Widget::render(
            Badge::new(&self.theme, "Secondary").variant(Variant::Secondary),
            badge_areas[1],
            buf,
        );
        ratatui::widgets::Widget::render(
            Badge::new(&self.theme, "Destructive").variant(Variant::Destructive),
            badge_areas[2],
            buf,
        );
        ratatui::widgets::Widget::render(
            Badge::new(&self.theme, "Outline").variant(Variant::Outline),
            badge_areas[3],
            buf,
        );

        ratatui::widgets::Widget::render(
            Separator::new(&self.theme).orientation(Orientation::Horizontal),
            sections[2],
            buf,
        );

        let columns = Layout::horizontal([Constraint::Percentage(48), Constraint::Percentage(52)])
            .split(sections[3]);
        let controls = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Min(1),
        ])
        .split(columns[0]);
        Checkbox::new(&self.theme, "Email alerts").render(controls[0], buf, &mut self.checkbox);
        Switch::new(&self.theme, "Compact mode").render(controls[1], buf, &mut self.switch);
        RadioGroup::new(&self.theme, &["System", "Compact", "Comfortable"]).render(
            controls[2],
            buf,
            &mut self.radio_group,
        );
        Select::new(&self.theme, &self.select_options)
            .placeholder("Choose a theme")
            .render(controls[3], buf, &mut self.select);

        let card = Card::new(&self.theme).title("Composed surface");
        let inner = card.inner(columns[1]);
        ratatui::widgets::Widget::render(card, columns[1], buf);
        let card_rows = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);
        ratatui::widgets::Widget::render(
            ratatui::widgets::Paragraph::new("Cards are surfaces, not app shells."),
            card_rows[0],
            buf,
        );
        ratatui::widgets::Widget::render(Progress::new(&self.theme, 0.72), card_rows[1], buf);
        ratatui::widgets::Widget::render(Skeleton::new(&self.theme), card_rows[3], buf);
    }
}

fn run_headless() {
    let mut app = App::new();
    let theme = Theme::dark();
    let tab_items: Vec<WidgetTabItem> = GALLERY_TABS
        .iter()
        .map(|title| WidgetTabItem::new(*title))
        .collect();
    let tabs = Tabs::new(&theme, &tab_items);
    let mut tabs_state = app.gallery_tab.clone();
    let snapshot = render_stateful(tabs, 120, 3, &mut tabs_state);
    assert!(snapshot.contains("Input"));
    assert!(snapshot.contains("Tree"));
    assert!(snapshot.contains("Diff"));
    assert!(snapshot.contains("Markdown"));
    assert!(snapshot.contains("Command"));
    assert!(snapshot.contains("Models"));
    assert!(snapshot.contains("Agent"));
    assert!(snapshot.contains("UI"));

    let picker_widget = CommandPalette::new("Open file", &theme).preview(true);
    let picker_snapshot = render_stateful(picker_widget, 80, 20, &mut app.picker);
    assert!(picker_snapshot.contains("Open file"));
    assert!(picker_snapshot.contains("src/main.rs"));
    println!("ok: gallery headless snapshot {} bytes", snapshot.len());
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    if std::env::var("RATAKIT_GALLERY_HEADLESS").is_ok() {
        run_headless();
        return Ok(());
    }

    let _session = enter_terminal()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut app = App::new();

    let mut scheduler = FrameScheduler::with_motion(app.theme.motion);
    scheduler.request();

    let mut events = event_stream();
    let mut tick = tokio::time::interval(Duration::from_millis(80));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    while !app.should_quit {
        tokio::select! {
            maybe = events.next() => {
                match maybe {
                    Some(Ok(ev)) => {
                        if let Event::Resize { width, height } = ev {
                            terminal.resize(Rect::new(0, 0, width, height))?;
                        }
                        app.handle(&ev);
                        scheduler.request();
                    }
                    Some(Err(err)) => return Err(err),
                    None => break,
                }
            }
            _ = scheduler.next_frame() => {
                terminal.draw(|f| {
                    app.draw(f);
                })?;
            }
            _ = tick.tick() => {
                app.handle(&Event::Tick(Instant::now()));
                scheduler.request();
            }
        }
    }

    Ok(())
}
