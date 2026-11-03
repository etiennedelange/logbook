//! Times search plus JSON serialization (what the desktop app sends over IPC) against the
//! real on-disk index cache, read-only.
//!
//! `cargo run --release -p logbook-core --example bench_payload`

use logbook_core::service::SessionService;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let s = SessionService::open()?;
    for q in ["a", "the", "fix", "dashboard env"] {
        let t = Instant::now();
        let hits = s.search(q, usize::MAX);
        let search = t.elapsed();
        let t = Instant::now();
        let json = serde_json::to_string(&hits)?;
        eprintln!(
            "{q:>14}: {:>4} hits, search {search:?}, json {:?}, {} KB",
            hits.len(),
            t.elapsed(),
            json.len() / 1024
        );
    }
    let t = Instant::now();
    let all = serde_json::to_string(&s.recent(usize::MAX))?;
    eprintln!("recent: json {:?}, {} KB", t.elapsed(), all.len() / 1024);
    Ok(())
}
