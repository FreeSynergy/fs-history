//! [`SourceActivity`]: what a source hands over, as plain values.
//!
//! Pattern: Adapter (target shape) — adapters fill it from their own types, so the core never sees
//! the hub's. It carries the summary fields only; a payload has no place in it.

use fs_store::ObjectId;

use super::entry::{Entry, Timestamp};

/// Verbs that only log that something happened to an object; their envelope names little more than
/// the object id. The core fills the rest from the object's latest entry.
pub const LOG_ONLY_KINDS: [&str; 12] = [
    "Add",
    "Remove",
    "Offer",
    "Accept",
    "Reject",
    "Read",
    "Invite",
    "TentativeAccept",
    "Join",
    "Leave",
    "Like",
    "Move",
];

/// The summary of one activity as a source delivers it. Fields as in [`Entry`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceActivity {
    /// The activity id.
    pub id: ObjectId,
    /// When it happened or was received.
    pub at: Timestamp,
    /// The program it happened in.
    pub program: String,
    /// The verb.
    pub kind: String,
    /// The object.
    pub object: ObjectId,
    /// AS2 type of the object; empty if the activity does not say.
    pub object_type: String,
    /// Display name; empty if the activity does not say.
    pub name: String,
    /// Who the object belongs to; empty if unknown.
    pub attributed_to: String,
    /// The object's context; empty if none or unknown.
    pub context: String,
    /// Who acted; empty if the source does not know.
    pub actor: String,
    /// `at` is the time of receiving.
    pub at_is_received: bool,
}

impl SourceActivity {
    /// Whether the verb is one of [`LOG_ONLY_KINDS`].
    pub fn is_log_only(&self) -> bool {
        LOG_ONLY_KINDS.contains(&self.kind.as_str())
    }

    /// The entry with the same fields.
    pub fn into_entry(self) -> Entry {
        Entry {
            id: self.id,
            at: self.at,
            program: self.program,
            kind: self.kind,
            object: self.object,
            object_type: self.object_type,
            name: self.name,
            attributed_to: self.attributed_to,
            context: self.context,
            actor: self.actor,
            at_is_received: self.at_is_received,
        }
    }
}
