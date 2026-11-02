//! Times a cold first index of every session (empty cache), in three ways:
//!   old       sequential, whole-file read, full timeline built (the pre-optimization scan)
//!   seq-light sequential, streaming read, no timeline
//!   new       `indexer::run` (streaming, no timeline, parallel)
//!
//! `cargo run --release -p logbook-core --example bench_index -- <old|seq-light|new>`
//! Read-only on the sessions; the new mode writes its cache to a temp file.

use logbook_core::index::SessionIndex;
use logbook_core::{events_reader, indexer, paths, yaml_reader};
use memchr::memmem;
use serde_json::Value;
use std::path::Path;
use std::time::Instant;

/// Verbatim logic of the old `events_reader::scan`.
fn legacy_scan(path: &Path) -> (usize, usize, usize) {
    let mut text_parts: Vec<String> = Vec::new();
    let mut timeline: Vec<(bool, String)> = Vec::new();
    let mut count = 0usize;
    let Ok(bytes) = std::fs::read(path) else { return (0, 0, 0) };
    let user = memmem::Finder::new(b"\"user.message\"");
    let assistant = memmem::Finder::new(b"\"assistant.message\"");
    for line in bytes.split(|&b| b == b'\n') {
        if user.find(line).is_none() && assistant.find(line).is_none() {
            continue;
        }
        let Ok(line) = std::str::from_utf8(line) else { continue };
        let Ok(value) = serde_json::from_str::<Value>(line) else { continue };
        let Some(t) = value.get("type").and_then(Value::as_str) else { continue };
        if t != "user.message" && t != "assistant.message" {
            continue;
        }
        count += 1;
        let Some(c) = value.get("data").and_then(|d| d.get("content")).and_then(Value::as_str) else { continue };
        if c.trim().is_empty() {
            continue;
        }
        text_parts.push(c.to_string());
        timeline.push((t == "user.message", c.to_string()));
    }
    (count, text_parts.join("\n").len(), timeline.len())
}

fn main() -> anyhow::Result<()> {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let dirs: Vec<_> = std::fs::read_dir(paths::session_state_root())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    let t = Instant::now();
    match mode.as_str() {
        "old" | "seq-light" => {
            let mut total = 0usize;
            for d in &dirs {
                if yaml_reader::try_read(d).is_none() {
                    continue;
                }
                let p = d.join("events.jsonl");
                total += if mode == "old" {
                    legacy_scan(&p).1
                } else {
                    events_reader::scan_for_index(&p).searchable_text.len()
                };
            }
            eprintln!("{mode}: {:?} ({} sessions, {} KB text)", t.elapsed(), dirs.len(), total / 1024);
        }
        "new" => {
            let cache = std::env::temp_dir().join(format!("bench-index-{}.bin", std::process::id()));
            let mut index = SessionIndex::open(&cache)?;
            let stats = indexer::run(&mut index)?;
            eprintln!("new: {:?} {:?}", t.elapsed(), stats);
            let _ = std::fs::remove_file(&cache);
        }
        _ => eprintln!("usage: bench_index <old|seq-light|new>"),
    }
    Ok(())
}
