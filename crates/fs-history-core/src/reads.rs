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

/// The type without the `as:` prefix or the `ActivityStreams` namespace; the case is kept.
pub fn strip_namespace(t: &str) -> &str {
    let t = t.trim();
    t.strip_prefix("https://www.w3.org/ns/activitystreams#")
        .or_else(|| t.strip_prefix("as:"))
        .unwrap_or(t)
}

/// A type name as compared: lower case, without the `as:` prefix or the `ActivityStreams` namespace.
pub fn normalize_type(t: &str) -> String {
    strip_namespace(t).to_lowercase()
}

/// Whether `t` names a type from [`NEVER_READ`], however it is written.
pub fn is_never_read(t: &str) -> bool {
    let t = normalize_type(t);
    NEVER_READ.iter().any(|n| normalize_type(n) == t)
}

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
        if let Some(bad) = list.iter().find(|t| is_never_read(t)) {
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
