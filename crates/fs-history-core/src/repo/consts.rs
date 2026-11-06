//! Constants of the stored objects.
//!
//! Pattern: Registry (constants) — types and media types used by this program in one place.

/// AS2 type of the envelope of an entry.
pub const ENTRY_TYPE: &str = "Activity";
/// Type of the object that remembers what was forgotten.
pub const TOMBSTONE_TYPE: &str = "Tombstone";
pub(super) const ENTRY_CONTENT: &str = "application/vnd.freesynergy.history-entry+json";
pub(super) const TOMBSTONE_CONTENT: &str = "application/vnd.freesynergy.history-tombstone+json";
