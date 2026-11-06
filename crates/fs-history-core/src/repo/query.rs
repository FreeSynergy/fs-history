//! Filter and cursor -> store query.
//!
//! Pattern: Builder (over the store's `Query`) — the same `EntryFilter` that works in memory is
//! translated here; the cursor is a keyset (time, id) so equal timestamps page stably.

use fs_store::{Cond, ObjectId, Op, Order, Query, TimeField, Timestamp};

use super::consts::ENTRY_TYPE;
use crate::error::{Error, Result};
use crate::model::{Cursor, Entry, EntryFilter};

/// Entries that pass `filter`, newest first, ties by id (descending).
pub(super) fn for_filter(filter: &EntryFilter) -> Query {
    let f = filter.clone().normalized();
    let mut q = Query::new()
        .of_type(ENTRY_TYPE)
        .ordered(Order::PublishedDesc);
    if let Some(t) = f.from {
        q = q.time(TimeField::Published, Op::Ge, t);
    }
    if let Some(t) = f.to {
        q = q.time(TimeField::Published, Op::Lt, t);
    }
    for (key, value) in [
        ("program", &f.program),
        ("person", &f.person),
        ("objectType", &f.object_type),
    ] {
        if !value.is_empty() {
            q = q.with_index(key, value.as_str());
        }
    }
    if !f.context.is_empty() {
        q = q.in_context(f.context);
    }
    if let Some(o) = f.object {
        q = q.with_index("object", o.to_string());
    }
    q
}

/// All entries of one object.
pub(super) fn for_object(object: ObjectId) -> Query {
    Query::new()
        .of_type(ENTRY_TYPE)
        .with_index("object", object.to_string())
        .ordered(Order::PublishedDesc)
}

/// The cursor that continues after `e`.
pub(super) fn cursor_of(e: &Entry) -> Cursor {
    Cursor(format!("{}:{}", e.at.timestamp_micros(), e.id))
}

/// Only what comes after the cursor in the order of `for_filter`.
pub(super) fn after(q: Query, cursor: &Cursor) -> Result<Query> {
    let bad = || Error::Invalid(format!("cursor {:?} is not one of this provider", cursor.0));
    let (micros, id) = cursor.0.split_once(':').ok_or_else(bad)?;
    let at =
        Timestamp::from_timestamp_micros(micros.parse().map_err(|_| bad())?).ok_or_else(bad)?;
    let id: ObjectId = id.parse().map_err(|_| bad())?;
    // Not newer than the cursor, and if equal in time then a smaller id (the OR-group does that).
    Ok(q.time(TimeField::Published, Op::Le, at)
        .or(Cond::Time {
            field: TimeField::Published,
            op: Op::Lt,
            value: at,
        })
        .or(Cond::Index {
            key: "activity".into(),
            op: Op::Lt,
            value: id.to_string(),
        }))
}
