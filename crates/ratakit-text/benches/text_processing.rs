use std::fmt::Write as _;
use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ratakit_core::Theme;
use ratakit_text::{DiffModel, MarkdownOptions, MarkdownStream, SyntectHighlighter};

fn benchmark_streaming_markdown(criterion: &mut Criterion) {
    let theme = Theme::dark();
    SyntectHighlighter::apply_theme(&theme);
    let highlighter = SyntectHighlighter;
    let mut history = String::new();
    for index in 0..256 {
        writeln!(
            history,
            "## Result {index}\n\nThe agent updated **module {index}** and added a regression test.\n"
        )
        .expect("write markdown fixture");
    }
    let mut group = criterion.benchmark_group("markdown_stream");
    group.bench_function("append_and_layout_after_256_committed_blocks", |bencher| {
        bencher.iter_batched(
            || {
                let mut stream = MarkdownStream::new(MarkdownOptions::default());
                stream.push(&history);
                black_box(stream.lines(80, &theme, &highlighter).len());
                stream
            },
            |mut stream| {
                stream.push("A streamed continuation with **new detail**.\n");
                black_box(stream.lines(80, &theme, &highlighter).len());
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn benchmark_large_diff(criterion: &mut Criterion) {
    let mut old = String::new();
    let mut new = String::new();
    for index in 0..4_000 {
        writeln!(old, "line {index}: original value").expect("write old diff fixture");
        let value = if index % 20 == 0 {
            "revised"
        } else {
            "original"
        };
        writeln!(new, "line {index}: {value} value").expect("write new diff fixture");
    }
    criterion.bench_function("large_diff/4000_lines", |bencher| {
        bencher.iter(|| black_box(DiffModel::from_texts("large.rs", &old, &new)));
    });
}

fn benchmark_text_processing(criterion: &mut Criterion) {
    benchmark_streaming_markdown(criterion);
    benchmark_large_diff(criterion);
}

criterion_group!(benches, benchmark_text_processing);
criterion_main!(benches);
