//! The store-backed history: module map.
//!
//! Pattern: Facade (declarations) — `HistoryRepo` is the one entry point; each file is one concept
//! (codec, query building, reads, timeline, digest, ingest, tombstones, forget, provider report).

mod codec;
mod consts;
mod deleted;
mod digest;
mod forget;
mod handle;
mod ingest;
mod provide;
mod query;
mod rows;
mod sealed;
mod timeline;
mod tombstone;

pub use consts::{ENTRY_TYPE, TOMBSTONE_TYPE};
pub use handle::HistoryRepo;
pub use ingest::Outcome;
