//! Tabbed gallery of ratakit generic widgets.
//!
//! Run interactively: `cargo run -p gallery`
//!
//! Headless CI smoke: `RATAKIT_GALLERY_HEADLESS=1 cargo run -p gallery --quiet`

use std::io::stdout;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use ratakit::core::{
    ActionLabels, Component, Event, FuzzyFilter, KeyCode, Keymap, KeymapPreset, ListState, Motion,
    MotionMode, TableState, TabsState, Theme, Toast, ToastLevel, ToastQueue, key_hints_from_keymap,
};
use ratakit::text::{LangId, MarkdownOptions, SyntectHighlighter};
use ratakit::widgets::{
    CodeBlock, CodeBlockState, ConfirmDialog, DialogState, FrameScheduler, KeyHints, Markdown,
    MarkdownState, RowRenderer, Spinner, StatusBar, StatusSegment, TabItem as WidgetTabItem, Table,
    TableColumn, Tabs, TextArea, TextAreaState, Toasts, Tree, VirtualList, enter_terminal,
    event_stream,
};
use ratakit::widgets::{TreeSource, TreeState};
use ratakit_testing::render_stateful;
use ratatui::Terminal;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::StatefulWidget;
use ratatui_crossterm::CrosstermBackend;

const GALLERY_TABS: [&str; 8] = [
    "Input", "List", "Tree", "Code", "Markdown", "Chrome", "Data", "Fuzzy",
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
    markdown: MarkdownState,
    motion: Motion,
    toasts: ToastQueue,
    dialog: DialogState,
    inner_tabs: TabsState,
    table: TableState,
    fuzzy: FuzzyFilter,
    fuzzy_query: String,
    fuzzy_hits: Vec<String>,
    fuzzy_candidates: Vec<String>,
    hint_labels: ActionLabels,
    highlighter: SyntectHighlighter,
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
            markdown,
            motion: Motion::new(),
            toasts,
            dialog: DialogState::new(),
            inner_tabs,
            table,
            fuzzy: FuzzyFilter::new(),
            fuzzy_query: "gal".into(),
            fuzzy_hits: Vec::new(),
            fuzzy_candidates: [
                "gallery",
                "gamma",
                "global",
                "git",
                "grep",
                "go",
                "grep files",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            hint_labels,
            highlighter: SyntectHighlighter,
            should_quit: false,
        }
    }

    fn refresh_fuzzy(&mut self) {
        self.fuzzy.set_pattern(&self.fuzzy_query);
        let ranked = self.fuzzy.rank(&self.fuzzy_candidates, 8);
        self.fuzzy_hits = ranked
            .iter()
            .map(|m| self.fuzzy_candidates[m.item_index].clone())
            .collect();
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
            7 => {
                if let Event::Key(key) = ev {
                    if key.kind == ratakit::core::KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char(c) => {
                                self.fuzzy_query.push(c);
                                self.refresh_fuzzy();
                            }
                            KeyCode::Backspace => {
                                self.fuzzy_query.pop();
                                self.refresh_fuzzy();
                            }
                            _ => {}
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
                let widget = Markdown::new(&self.theme, &self.highlighter);
                widget.render(chunks[1], frame.buffer_mut(), &mut self.markdown);
            }
            5 => {
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
            6 => {
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
            7 => {
                use ratatui::style::Style;
                use ratatui::text::Line;
                use ratatui::widgets::Paragraph;
                let body = format!(
                    "Query: {}\n\nMatches:\n{}",
                    self.fuzzy_query,
                    self.fuzzy_hits.join("\n")
                );
                ratatui::widgets::Widget::render(
                    Paragraph::new(body.lines().map(Line::from).collect::<Vec<_>>())
                        .style(Style::default()),
                    chunks[1],
                    frame.buffer_mut(),
                );
            }
            _ => {}
        }

        let footer = format!(
            "Tab/1-8: sections · m: modal · q: quit · motion: {}",
            match self.theme.motion {
                MotionMode::Full => "full",
                MotionMode::Reduced => "reduced",
            }
        );
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

fn run_headless() {
    let mut app = App::new();
    app.refresh_fuzzy();
    let theme = Theme::dark();
    let tab_items: Vec<WidgetTabItem> = GALLERY_TABS
        .iter()
        .map(|title| WidgetTabItem::new(*title))
        .collect();
    let tabs = Tabs::new(&theme, &tab_items);
    let mut tabs_state = app.gallery_tab.clone();
    let snapshot = render_stateful(tabs, 80, 3, &mut tabs_state);
    assert!(snapshot.contains("Input"));
    assert!(snapshot.contains("Tree"));
    assert!(snapshot.contains("Markdown"));
    assert!(snapshot.contains("Fuzzy"));
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
    app.refresh_fuzzy();

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
