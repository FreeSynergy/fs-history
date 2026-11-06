//! Which AS2 types history asks the hub for.
//!
//! Pattern: Newtype (validated value) — a `Reads` list can only be built without the types that are
//! never read: `Relationship` (role privacy, sprint rule) and `SurveyResponse` (anonymous answers).

use crate::error::{Error, Result};

/// Types read by default: what people recognise as their own work. No `Relationship`, no
/// `SurveyResponse`, no invitations or delegations.
pub const DEFAULT_READS: [&str; 9] = [
    "Profile",
    "Event",
    "Task",
    "Note",
    "Article",
    "Document",
    "Place",
    "Group",
    "Collection",
];

/// Types that must never be read, by any list and by any source.
pub const NEVER_READ: [&str; 2] = ["Relationship", "SurveyResponse"];

/// A list of types to read that does not name a type from [`NEVER_READ`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reads(Vec<String>);

impl Reads {
    /// The list `types`; a refused type makes the whole list fail.
    pub fn new<I, S>(types: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let list: Vec<String> = types.into_iter().map(Into::into).collect();
        if let Some(bad) = list.iter().find(|t| NEVER_READ.contains(&t.as_str())) {
            return Err(Error::Refused(format!("{bad} is never read by history")));
        }
        Ok(Self(list))
    }

    /// The types, in order.
    pub fn types(&self) -> &[String] {
        &self.0
    }
}

impl Default for Reads {
    fn default() -> Self {
        Self(DEFAULT_READS.iter().map(|t| (*t).to_string()).collect())
    }
}
