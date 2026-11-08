//! [`Entry`]: the summary of one activity, as the timeline shows it.
//!
//! Pattern: Value object — the Rust form of `Entry` in `history.capnp`. It never holds a payload.

use fs_objects::ObjectId;
pub use fs_objects::Timestamp;

/// One line of the history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The activity id in the hub log.
    pub id: ObjectId,
    /// When it happened if the hub knows that, else when the hub received it.
    pub at: Timestamp,
    /// Which program it happened in.
    pub program: String,
    /// AS2 verb as the hub logs it: `Create`, `Update`, `Delete`, `Invite`, ...
    pub kind: String,
    /// The object the activity is about.
    pub object: ObjectId,
    /// AS2 type of the object.
    pub object_type: String,
    /// Display name at that moment; for log-only verbs the last known. Empty after a delete.
    pub name: String,
    /// Who the object belongs to; not always who acted.
    pub attributed_to: String,
    /// The object's context; for log-only verbs the last known. Empty after a delete.
    pub context: String,
    /// Who acted; empty until the hub delivers it.
    pub actor: String,
    /// `at` is only when the hub received it, so it can be much later.
    pub at_is_received: bool,
}

impl Entry {
    /// The person this entry counts for in a `person` filter: `actor`, else `attributed_to`.
    pub fn person(&self) -> &str {
        if self.actor.is_empty() {
            &self.attributed_to
        } else {
            &self.actor
        }
    }
}
