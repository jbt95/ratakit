use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ratakit_agent::state::{AgentEvent, ContentPart, MessageId, TranscriptState};
use ratakit_agent::widgets::Transcript;
use ratakit_core::Theme;
use ratakit_text::SyntectHighlighter;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn transcript_with_turns(turns: usize) -> TranscriptState {
    let mut state = TranscriptState::new();
    for index in 0..turns {
        state.apply(AgentEvent::UserMessage {
            id: MessageId::new(format!("user-{index}")),
            content: vec![ContentPart::Text(format!("Review change {index}"))],
        });
        let id = MessageId::new(format!("assistant-{index}"));
        state.apply(AgentEvent::AssistantDelta {
            id: id.clone(),
            text: format!(
                "Change **{index}** is complete.\n\n- Updated the module.\n- Added a regression test."
            ),
        });
        state.apply(AgentEvent::MessageDone { id });
    }
    state
}

fn benchmark_long_transcript(criterion: &mut Criterion) {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let mut group = criterion.benchmark_group("transcript_render");
    group.bench_function("500_turns_1000_cells_80x24", |bencher| {
        bencher.iter_batched(
            || transcript_with_turns(500),
            |mut state| {
                let backend = TestBackend::new(80, 24);
                let mut terminal = Terminal::new(backend).expect("test terminal");
                terminal
                    .draw(|frame| {
                        frame.render_stateful_widget(
                            Transcript::new(&theme, &highlighter),
                            frame.area(),
                            &mut state,
                        );
                    })
                    .expect("transcript draw");
                black_box(terminal.backend());
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();

    let transcript = transcript_with_turns(500);
    let mut search_group = criterion.benchmark_group("transcript_search");
    search_group.bench_function("500_turns_query_module_limit_20", |bencher| {
        bencher.iter(|| black_box(transcript.search(black_box("module"), 20)));
    });
    search_group.finish();
}

criterion_group!(benches, benchmark_long_transcript);
criterion_main!(benches);
