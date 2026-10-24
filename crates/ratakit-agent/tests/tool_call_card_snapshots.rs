#![cfg(feature = "widgets")]

use std::path::PathBuf;

use ratakit_agent::state::{
    AgentEvent, ToolCall, ToolCallCell, ToolCallId, ToolContent, ToolKind, ToolStatus,
    TranscriptState,
};
use ratakit_agent::widgets::{ToolCallCard, ToolCallCardAux};
use ratakit_core::{MotionMode, Theme};
use ratakit_testing::render_stateful;
use ratakit_text::{DiffModel, SyntectHighlighter};

fn compact_snapshot(rendered: String) -> String {
    let Some((text, styles)) = rendered.split_once("--- styles ---\n") else {
        return rendered;
    };
    let compact_styles = styles
        .lines()
        .filter(|line| !line.is_empty())
        .filter(|line| interesting_style_line(line))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{text}--- styles ---\n{compact_styles}\n")
}

fn interesting_style_line(line: &str) -> bool {
    let Some(style) = line.split(": ").nth(1) else {
        return false;
    };
    style.contains("bold")
        || style.contains("italic")
        || style.contains("fg=rgb(137,180,250)")
        || style.contains("fg=rgb(166,227,161)")
        || style.contains("fg=rgb(243,139,168)")
        || style.contains("fg=rgb(249,226,175)")
        || style.contains("fg=rgb(108,112,134)")
}

fn sample_cell(
    status: ToolStatus,
    kind: ToolKind,
    output: &str,
    folded: bool,
    with_diff: bool,
) -> ToolCallCell {
    let mut call = ToolCall {
        id: ToolCallId::new("t1"),
        title: "Read project files".into(),
        kind,
        status,
        locations: vec![PathBuf::from("src/main.rs"), PathBuf::from("src/lib.rs")],
        content: Vec::new(),
    };
    if status == ToolStatus::Pending || status == ToolStatus::InProgress {
        call.content.push(ToolContent::Text("rg --files".into()));
    }
    if !output.is_empty() {
        call.content.push(ToolContent::Text(output.into()));
    }
    if with_diff {
        let model = DiffModel::from_texts("main.rs", "old\n", "new\n");
        call.content.push(ToolContent::Diff {
            path: PathBuf::from("main.rs"),
            model,
        });
    }
    cell_from_call(call, folded)
}

fn cell_from_call(call: ToolCall, folded: bool) -> ToolCallCell {
    let mut transcript = TranscriptState::new();
    transcript.apply(AgentEvent::ToolCallStarted(call));
    let idx = transcript.cells().len() - 1;
    let cell = match &transcript.cells()[idx] {
        ratakit_agent::state::Cell::ToolCall(c) => c,
        _ => panic!("expected tool call cell"),
    };
    ToolCallCell {
        call: cell.call.clone(),
        input: cell.input.clone(),
        output: cell.output.clone(),
        diff: cell.diff.clone(),
        output_folded: folded,
    }
}

fn long_output() -> String {
    (1..=12)
        .map(|n| format!("line {n:02}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_card(cell: &ToolCallCell, theme: &Theme, width: u16, height: u16) -> String {
    SyntectHighlighter::apply_theme(theme);
    let highlighter = SyntectHighlighter;
    let widget = ToolCallCard::new(theme, &highlighter, cell);
    let mut aux = ToolCallCardAux::default();
    render_stateful(widget, width, height, &mut aux)
}

fn assert_compact(name: &str, cell: ToolCallCell, theme: Theme, w: u16, h: u16) {
    let rendered = render_card(&cell, &theme, w, h);
    insta::assert_snapshot!(name, compact_snapshot(rendered));
}

#[test]
fn tool_call_status_pending() {
    let theme = Theme::dark();
    let cell = sample_cell(ToolStatus::Pending, ToolKind::Read, "", false, false);
    assert_compact("tool_call_pending_56x10_dark", cell, theme, 56, 10);
}

#[test]
fn tool_call_status_in_progress_spinner() {
    let theme = Theme::dark();
    let cell = sample_cell(ToolStatus::InProgress, ToolKind::Execute, "", false, false);
    assert_compact("tool_call_in_progress_56x10_dark", cell, theme, 56, 10);
}

#[test]
fn tool_call_status_completed() {
    let theme = Theme::dark();
    let cell = sample_cell(
        ToolStatus::Completed,
        ToolKind::Edit,
        "ok\nsaved",
        false,
        false,
    );
    assert_compact("tool_call_completed_56x12_dark", cell, theme, 56, 12);
}

#[test]
fn tool_call_status_failed() {
    let mut theme = Theme::light();
    theme.with_glyphs(ratakit_core::GlyphSet::Ascii);
    let cell = sample_cell(ToolStatus::Failed, ToolKind::Fetch, "timeout", false, false);
    assert_compact("tool_call_failed_56x10_light_ascii", cell, theme, 56, 10);
}

#[test]
fn tool_call_status_cancelled() {
    let theme = Theme::dark();
    let cell = sample_cell(ToolStatus::Cancelled, ToolKind::Search, "", false, false);
    assert_compact("tool_call_cancelled_56x8_dark", cell, theme, 56, 8);
}

#[test]
fn tool_call_kinds() {
    let theme = Theme::dark();
    for (kind, name) in [
        (ToolKind::Read, "read"),
        (ToolKind::Edit, "edit"),
        (ToolKind::Execute, "execute"),
        (ToolKind::Search, "search"),
        (ToolKind::Fetch, "fetch"),
        (ToolKind::Other, "other"),
    ] {
        let cell = sample_cell(ToolStatus::Completed, kind, "done", false, false);
        assert_compact(
            &format!("tool_call_kind_{name}_56x8_dark"),
            cell,
            theme.clone(),
            56,
            8,
        );
    }
}

#[test]
fn tool_call_truncated_output() {
    let theme = Theme::dark();
    let cell = sample_cell(
        ToolStatus::Completed,
        ToolKind::Execute,
        &long_output(),
        false,
        false,
    );
    assert_compact("tool_call_truncated_output_56x14_dark", cell, theme, 56, 14);
}

#[test]
fn tool_call_output_folded() {
    let theme = Theme::dark();
    let cell = sample_cell(
        ToolStatus::Completed,
        ToolKind::Execute,
        &long_output(),
        true,
        false,
    );
    assert_compact("tool_call_output_folded_56x10_dark", cell, theme, 56, 10);
}

#[test]
fn tool_call_output_unfolded() {
    let theme = Theme::dark();
    let cell = sample_cell(
        ToolStatus::Completed,
        ToolKind::Execute,
        &long_output(),
        false,
        false,
    );
    assert_compact("tool_call_output_unfolded_56x14_dark", cell, theme, 56, 14);
}

#[test]
fn tool_call_with_diff() {
    let theme = Theme::dark();
    let cell = sample_cell(ToolStatus::Completed, ToolKind::Edit, "", false, true);
    assert_compact("tool_call_with_diff_56x16_dark", cell, theme, 56, 16);
}

#[test]
fn tool_call_spinner_reduced_motion() {
    let mut theme = Theme::dark();
    theme.with_motion(MotionMode::Reduced);
    let cell = sample_cell(ToolStatus::InProgress, ToolKind::Read, "", false, false);
    assert_compact("tool_call_reduced_motion_56x8_dark", cell, theme, 56, 8);
}
