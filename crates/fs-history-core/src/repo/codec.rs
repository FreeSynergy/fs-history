//! Entry <-> stored object.
//!
//! Pattern: Adapter — maps the summary onto an AS2 envelope of type `Activity` (name, context,
//! attributedTo, published) plus index fields the store can query. The content holds the rest of
//! the summary. No payload and no object content is ever written: the fields are the whole record.

use fs_objects::{Envelope, StoredObject};
use serde_json::{Value, json};

use super::consts::{ENTRY_CONTENT, ENTRY_TYPE};
use crate::error::{Error, Result};
use crate::model::Entry;
use crate::reads::normalize_type;

/// The store refuses empty index values, so an empty field is simply not indexed.
pub(super) fn with_nonempty(o: StoredObject, key: &str, value: &str) -> StoredObject {
    if value.is_empty() {
        o
    } else {
        o.with_index(key, value)
    }
}

fn text(content: &Value, key: &str) -> String {
    content[key].as_str().unwrap_or_default().to_string()
}

pub(super) fn to_object(e: &Entry) -> StoredObject {
    // The envelope name is a label of the entry, never the object's name (which can be forgotten).
    let mut env = Envelope::new(ENTRY_TYPE, format!("{} in {}", e.kind, e.program));
    env.id = e.id;
    env.published = e.at;
    env.updated = e.at;
    env.context = (!e.context.is_empty()).then(|| e.context.clone());
    env.attributed_to = (!e.attributed_to.is_empty()).then(|| e.attributed_to.clone());
    let content = json!({
        "program": e.program,
        "kind": e.kind,
        "object": e.object.to_string(),
        "objectType": e.object_type,
        "name": e.name,
        "actor": e.actor,
        "atIsReceived": e.at_is_received,
    });
    let o = StoredObject::new(env, ENTRY_CONTENT, content)
        .with_index("activity", e.id.to_string())
        .with_index("object", e.object.to_string());
    let o = with_nonempty(o, "program", &e.program);
    let o = with_nonempty(o, "kind", &e.kind);
    let o = with_nonempty(o, "objectType", &normalize_type(&e.object_type));
    with_nonempty(o, "person", e.person())
}

pub(super) fn from_object(o: &StoredObject) -> Result<Entry> {
    let c = &o.content;
    let object = text(c, "object")
        .parse()
        .map_err(|e: fs_objects::Error| Error::Invalid(e.to_string()))?;
    Ok(Entry {
        id: o.envelope.id,
        at: o.envelope.published,
        program: text(c, "program"),
        kind: text(c, "kind"),
        object,
        object_type: text(c, "objectType"),
        name: text(c, "name"),
        attributed_to: o.envelope.attributed_to.clone().unwrap_or_default(),
        context: o.envelope.context.clone().unwrap_or_default(),
        actor: text(c, "actor"),
        at_is_received: c["atIsReceived"].as_bool().unwrap_or(false),
    })
}

/// The entry of a deleted object: no name, no context; the type stays.
pub(super) fn blanked(e: &Entry) -> Entry {
    Entry {
        name: String::new(),
        context: String::new(),
        ..e.clone()
    }
}
