//! Values, filters and records between the frontend's `Values` and the core's types.
//!
//! Pattern: Adapter — pure functions, no state.

use chrono::{DateTime, SecondsFormat, Utc};
use fs_admin_frontend::{BackendError, Record, Values};
use fs_history_core::{DigestEntry, Entry, EntryFilter};
use fs_objects::ObjectId;
use fs_types::{RecordIssue, RecordProblem, TypeProblem};

pub(super) fn first<'a>(v: &'a Values, key: &str) -> &'a str {
    v.get(key)
        .and_then(|l| l.iter().map(|s| s.trim()).find(|s| !s.is_empty()))
        .unwrap_or("")
}

pub(super) fn bad(field: &str) -> BackendError {
    BackendError::Rejected(vec![RecordIssue {
        field: field.to_owned(),
        index: 0,
        problem: RecordProblem::Value(TypeProblem::BadDateTime),
    }])
}

pub(super) fn time(v: &Values, key: &str) -> Result<Option<DateTime<Utc>>, BackendError> {
    match first(v, key) {
        "" => Ok(None),
        t => DateTime::parse_from_rfc3339(t)
            .map(|d| Some(d.with_timezone(&Utc)))
            .map_err(|_| bad(key)),
    }
}

/// The record a command works on: the id the frontend passes, else the `entry` input (a caller without a record).
pub(super) fn one_id(record: Option<&str>, input: &Values) -> Result<ObjectId, BackendError> {
    record
        .unwrap_or_else(|| first(input, "entry"))
        .parse()
        .map_err(|_| BackendError::NotFound)
}

/// The filter members of a command input (`from`, `to`, `program`, `person`, `objectType`, `context`, `object`).
pub(super) fn filter_of(v: &Values) -> Result<EntryFilter, BackendError> {
    let object = match first(v, "object") {
        "" => None,
        o => Some(o.parse().map_err(|_| BackendError::NotFound)?),
    };
    Ok(EntryFilter {
        from: time(v, "from")?,
        to: time(v, "to")?,
        program: first(v, "program").to_owned(),
        person: first(v, "person").to_owned(),
        object_type: first(v, "objectType").to_owned(),
        context: first(v, "context").to_owned(),
        object,
    })
}

pub(super) fn entry_matches(e: &Entry, term: &str) -> bool {
    term.is_empty()
        || [&e.name, &e.program, &e.kind, &e.object_type, &e.context]
            .iter()
            .any(|f| f.to_lowercase().contains(term))
}

pub(super) fn digest_matches(d: &DigestEntry, term: &str) -> bool {
    entry_matches(&d.first, term) || d.kinds.iter().any(|k| k.to_lowercase().contains(term))
}

pub(super) fn stamp(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub(super) fn put(values: &mut Values, key: &str, value: &str) {
    if !value.is_empty() {
        values.insert(key.to_owned(), vec![value.to_owned()]);
    }
}

pub(super) fn entry_values(e: &Entry) -> Values {
    let mut v = Values::new();
    put(&mut v, "id", &e.id.to_string());
    put(&mut v, "at", &stamp(e.at));
    put(&mut v, "program", &e.program);
    put(&mut v, "kind", &e.kind);
    put(&mut v, "object", &e.object.to_string());
    put(&mut v, "objectType", &e.object_type);
    put(&mut v, "name", &e.name);
    put(&mut v, "attributedTo", &e.attributed_to);
    put(&mut v, "context", &e.context);
    put(&mut v, "actor", &e.actor);
    v
}

pub(super) fn entry_record(e: &Entry) -> Record {
    Record {
        id: e.id.to_string(),
        values: entry_values(e),
    }
}

pub(super) fn digest_record(d: &DigestEntry) -> Record {
    let mut v = Values::new();
    put(&mut v, "first", &d.first.id.to_string());
    put(&mut v, "lastAt", &stamp(d.last_at));
    put(&mut v, "count", &d.count.to_string());
    if !d.kinds.is_empty() {
        v.insert("kinds".to_owned(), d.kinds.clone());
    }
    Record {
        id: d.first.id.to_string(),
        values: v,
    }
}
