//! [`EntryFilter`]: which entries a view wants.
//!
//! Pattern: Specification — a plain value with a predicate; the repository translates the same
//! value into a store query, and tombstones and subscriptions evaluate it in memory.

use fs_objects::ObjectId;

use crate::reads::normalize_type;

use super::entry::{Entry, Timestamp};

/// All members optional; unset (`None`, empty text) means no restriction; combined with AND.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryFilter {
    /// At or after.
    pub from: Option<Timestamp>,
    /// Before (half-open range).
    pub to: Option<Timestamp>,
    /// Program.
    pub program: String,
    /// Matches `actor`, or `attributed_to` where `actor` is empty.
    pub person: String,
    /// AS2 type of the object.
    pub object_type: String,
    /// Context.
    pub context: String,
    /// One object.
    pub object: Option<ObjectId>,
}

impl EntryFilter {
    /// The filter with all-zero times counted as unset, as the contract says.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        let unset = |t: &Option<Timestamp>| t.is_some_and(|t| t.timestamp_micros() == 0);
        if unset(&self.from) {
            self.from = None;
        }
        if unset(&self.to) {
            self.to = None;
        }
        self
    }

    /// No restriction at all (after [`normalized`](Self::normalized)).
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Whether `e` passes every set member.
    pub fn matches(&self, e: &Entry) -> bool {
        let f = self.clone().normalized();
        let text = |want: &str, have: &str| want.is_empty() || want == have;
        f.from.is_none_or(|t| e.at >= t)
            && f.to.is_none_or(|t| e.at < t)
            && text(&f.program, &e.program)
            && (f.person.is_empty() || f.person == e.person())
            && (f.object_type.is_empty()
                || normalize_type(&f.object_type) == normalize_type(&e.object_type))
            && text(&f.context, &e.context)
            && f.object.is_none_or(|o| o == e.object)
    }
}
