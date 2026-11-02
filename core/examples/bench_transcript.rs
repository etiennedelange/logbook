//! Times reading the transcripts of the largest sessions (what a click on a row costs).
//!
//! `cargo run --release -p logbook-core --example bench_transcript`

use logbook_core::{events_reader, paths};
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let mut files: Vec<_> = std::fs::read_dir(paths::session_state_root())?
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("events.jsonl"))
        .filter_map(|p| std::fs::metadata(&p).ok().map(|m| (m.len(), p)))
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    for (size, path) in files.iter().take(5) {
        let t = Instant::now();
        let scan = events_reader::scan(path);
        let text: usize = scan.timeline.iter().map(|e| e.text.len()).sum();
        eprintln!(
            "{:>6.1} MB -> {:>5} messages, {:>6} KB text, {:?}",
            *size as f64 / 1e6,
            scan.timeline.len(),
            text / 1024,
            t.elapsed()
        );
    }
    Ok(())
}
