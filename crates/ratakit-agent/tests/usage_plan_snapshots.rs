use ratakit_agent::state::{ModeId, Plan, PlanEntry, PlanEntryId, PlanEntryStatus, Usage};
use ratakit_agent::{PlanView, UsageBar, plan_cell_height, render_plan_cell};
use ratakit_core::{GlyphSet, ListState, Theme};
use ratakit_testing::{assert_stateful_widget_snapshot, assert_widget_snapshot};
use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;

fn sample_usage() -> Usage {
    Usage {
        input_tokens: 128_450,
        output_tokens: 3_200,
        cached_tokens: 12_000,
        context_used: Some(0.42),
        cost_usd: Some(0.0384),
    }
}

fn sample_plan() -> Plan {
    Plan {
        entries: vec![
            PlanEntry {
                id: PlanEntryId::new("1"),
                title: "Read design doc".to_string(),
                status: PlanEntryStatus::Completed,
            },
            PlanEntry {
                id: PlanEntryId::new("2"),
                title: "Implement UsageBar and PlanView widgets".to_string(),
                status: PlanEntryStatus::InProgress,
            },
            PlanEntry {
                id: PlanEntryId::new("3"),
                title: "Open pull request".to_string(),
                status: PlanEntryStatus::Pending,
            },
        ],
    }
}

#[test]
fn usage_bar_wide_120x1() {
    let theme = Theme::dark();
    let usage = sample_usage();
    let mode = ModeId::new("auto");
    let widget = UsageBar::new(&usage, &mode, &theme).git_branch(Some("cursor/a7"));
    assert_widget_snapshot!("usage_bar_wide_120x1", widget, 120, 1);
}

#[test]
fn usage_bar_narrow_40x1() {
    let theme = Theme::dark();
    let usage = sample_usage();
    let mode = ModeId::new("plan");
    let widget = UsageBar::new(&usage, &mode, &theme).git_branch(Some("main"));
    assert_widget_snapshot!("usage_bar_narrow_40x1", widget, 40, 1);
}

#[test]
fn usage_bar_screen_reader_uses_text_instead_of_a_gauge() {
    let mut theme = Theme::dark();
    theme.enable_screen_reader();
    let usage = sample_usage();
    let mode = ModeId::new("auto");
    let rendered = ratakit_testing::render_widget(
        UsageBar::new(&usage, &mode, &theme).git_branch(Some("main")),
        120,
        1,
    );

    assert!(rendered.contains("Context 42%"));
    assert!(rendered.contains("Mode auto"));
    assert!(rendered.contains("Branch main"));
    assert!(!rendered.contains('█'));
    assert!(!rendered.contains('░'));
}

#[test]
fn plan_view_states_48x8() {
    let theme = Theme::dark();
    let plan = sample_plan();
    let widget = PlanView::new(&plan, &theme);
    let mut state = ListState::new();
    assert_stateful_widget_snapshot!("plan_view_states_48x8", widget, &mut state, 48, 8);
}

#[test]
fn plan_cell_transcript_40x6() {
    let theme = Theme::dark();
    let plan = sample_plan();
    let height = plan_cell_height(&plan, 40, theme.glyphs);
    let rendered = render_plan_cell_snapshot(&plan, &theme, 40, height);
    insta::assert_snapshot!("plan_cell_transcript_40x6", rendered);
}

fn render_plan_cell_snapshot(plan: &Plan, theme: &Theme, width: u16, height: u16) -> String {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    render_plan_cell(plan, theme, area, &mut buf, 0);
    ratakit_testing::buffer_to_string(&buf)
}

#[test]
fn plan_view_ascii_40x5() {
    let mut theme = Theme::light();
    theme.with_glyphs(GlyphSet::Ascii);
    let plan = sample_plan();
    let widget = PlanView::new(&plan, &theme);
    let mut state = ListState::new();
    assert_stateful_widget_snapshot!("plan_view_ascii_40x5", widget, &mut state, 40, 5);
}

#[test]
fn plan_view_screen_reader_names_each_status() {
    let mut theme = Theme::dark();
    theme.enable_screen_reader();
    let plan = sample_plan();
    let mut state = ListState::new();
    let rendered =
        ratakit_testing::render_stateful(PlanView::new(&plan, &theme), 80, 6, &mut state);

    for status in ["Completed:", "In progress:", "Pending:"] {
        assert!(
            rendered.contains(status),
            "missing {status:?} in {rendered}"
        );
    }
}
