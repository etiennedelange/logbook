# gpui-proto

Experiment, not shipped: Logbook's search-and-list screen in GPUI ([gpui-kit](https://crates.io/crates/gpui-kit)),
over `logbook-core` with no IPC. See "Experiment: a GPUI front end" in `docs/PLAN.md` for the question and the criteria.

```bash
cargo run -p logbook-gpui-proto     # not a default workspace member
cargo test -p logbook-gpui-proto
```

Linux needs `libxkbcommon-dev libxkbcommon-x11-dev` (and a Vulkan driver such as `mesa-vulkan-drivers` to open a window).

- `src/rows.rs`: what the list shows, as plain data (titles, meta line, snippet ranges). Unit-tested.
- `src/main.rs`: window, search box, list. Keys: Up/Down move, Enter copies the resume command.
