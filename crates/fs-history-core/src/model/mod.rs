//! Plain data types of the history core: module map.
//!
//! Pattern: Facade (declarations) — one file per concept: entry, activity from a source, filter,
//! digest line, paging.

mod activity;
mod digest_entry;
mod entry;
mod filter;
mod page;

pub use activity::{LOG_ONLY_KINDS, SourceActivity};
pub use digest_entry::DigestEntry;
pub use entry::{Entry, Timestamp};
pub use filter::EntryFilter;
pub use page::{Cursor, Page};
