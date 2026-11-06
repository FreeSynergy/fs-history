//! What history tells the hub when it registers.
//!
//! Pattern: Builder (over the hub's `Manifest`) — the types to read come from the core's validated
//! `Reads`, so `Relationship` and `SurveyResponse` cannot get in. History writes nothing.

use fs_history_core::Reads;
use fs_hub_client::Manifest;

/// The program name history registers with.
pub const PROGRAM: &str = "history";

const CAPABILITIES: [&str; 5] = [
    "history.observe",
    "history.read",
    "history.digest",
    "history.forget",
    "history.subscribe",
];

/// The manifest with the default `reads` ([`fs_history_core::DEFAULT_READS`]) and no `writes`.
pub fn manifest() -> Manifest {
    manifest_with(&Reads::default())
}

/// The manifest for a chosen list of types to read.
pub fn manifest_with(reads: &Reads) -> Manifest {
    let types: Vec<&str> = reads.types().iter().map(String::as_str).collect();
    Manifest::new(PROGRAM, env!("CARGO_PKG_VERSION"))
        .read(&types)
        .write(&[])
        .capabilities(&CAPABILITIES)
}
