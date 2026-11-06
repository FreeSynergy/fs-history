//! [`DigestEntry`]: several close changes of one object by one program, as one line.
//!
//! Pattern: Value object — the Rust form of `DigestEntry` in `history.capnp`.

use super::entry::{Entry, Timestamp};

/// A group of entries of one object and program, no more than the window apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DigestEntry {
    /// The earliest entry of the group.
    pub first: Entry,
    /// The time of the latest entry of the group.
    pub last_at: Timestamp,
    /// How many entries the group holds.
    pub count: u32,
    /// Each verb once, in order of first appearance (earliest first).
    pub kinds: Vec<String>,
    /// Cut off here because the run is very long; the next page goes on with it.
    pub continues: bool,
}
