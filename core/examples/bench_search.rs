//! Times searches against the real on-disk index cache (read-only) and prints every
//! result's id and snippet, so two builds' output can be diffed for accuracy.
//!
//! `cargo run --release -p logbook-core --example bench_search [query ...]`

use logbook_core::{fts_query, index::SessionIndex, paths};
use std::time::Instant;

const DEFAULT_QUERIES: &[&str] = &[
    "a", "the", "rust", "dashboard env", "what gets scheduled", "\"what gets\"", "error handling",
    "tauri", "getspec", "pipeline yaml", "fix", "x", "release notes", "sql", "zzzzqqq",
];

fn main() -> anyhow::Result<()> {
    let t = Instant::now();
    let index = SessionIndex::open(&paths::index_cache_path())?;
    eprintln!("open: {:?} ({} sessions, {} with messages)", t.elapsed(), index.len(), index.listed_count());

    let args: Vec<String> = std::env::args().skip(1).collect();
    let queries: Vec<&str> = if args.is_empty() { DEFAULT_QUERIES.to_vec() } else { args.iter().map(String::as_str).collect() };

    for q in queries {
        let q = q.trim();
        let phrase_only = q.len() > 1 && q.starts_with('"') && q.ends_with('"');
        let tokens = fts_query::build(q);
        let mut results = index.search(&tokens, phrase_only, usize::MAX);
        let mut times = Vec::new();
        for _ in 0..5 {
            let t = Instant::now();
            results = index.search(&tokens, phrase_only, usize::MAX);
            times.push(t.elapsed());
        }
        times.sort();
        eprintln!("{q:>24}: {:>5} hits, median {:?}", results.len(), times[2]);
        println!("## {q}");
        for r in &results {
            println!("{} {:?}", r.session.id, r.snippet);
        }
    }
    Ok(())
}
