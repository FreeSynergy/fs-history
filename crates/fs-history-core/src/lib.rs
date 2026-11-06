//! `fs-history-core`: program core of FreeSynergy history (no GUI, no capnp server yet).
//!
//! History observes what happens across programs (ADR 0020) and keeps a derived, local index: one
//! [`Entry`] per delivered activity, a summary and never the content. The index is written without
//! the activity log (`UnloggedObjects`), so it does not double every activity. What a person forgets
//! stays forgotten, also when the same activity is delivered again.
//! Start with [`History`] and [`HistoryRepo`]; feed it with an [`Observer`] over an [`ActivitySource`].
//! See `DESIGN.md` for the decisions.
//!
//! Pattern: Facade — this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// Pedantic lints that would only add noise to this crate's small, documented API: every fallible fn
// returns the crate `Result` (documented once on `Error`), and results of pure getters are used anyway.
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions
)]

pub mod digest;
mod error;
mod feed;
pub mod model;
mod observer;
mod reads;
mod repo;
mod source;
pub mod traits;
mod unsupported;

pub use error::{Error, Result};
pub use feed::{Feed, FeedEvent, Subscription};
pub use model::{
    Cursor, DigestEntry, Entry, EntryFilter, LOG_ONLY_KINDS, Page, SourceActivity, Timestamp,
};
pub use observer::{Observer, RunReport};
pub use reads::{DEFAULT_READS, NEVER_READ, Reads, is_never_read, normalize_type, strip_namespace};
pub use repo::{ENTRY_TYPE, HistoryRepo, Outcome, TOMBSTONE_TYPE};
pub use source::{ActivitySource, Batch};
pub use traits::History;
pub use unsupported::not_possible;
