pub mod changes;
pub mod dto;
pub mod events_reader;
pub mod fts_query;
pub mod git;
pub mod index;
pub mod indexer;
pub mod models;
pub mod paths;
pub mod service;
pub mod source;
pub mod yaml_reader;

/// The version of Logbook this engine belongs to (bumped with the app by `tools/release`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
