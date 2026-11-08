//! A forgotten filter whose texts are salted hashes.
//!
//! Pattern: Specification (sealed) — the same predicate as `EntryFilter`, but the texts of the
//! filter (a project name, a person) are kept only as SHA-256 of salt, field and value, so a
//! forgotten name is not readable from the tombstone. A guessed value can still be tested; that is
//! the stated limit. Times and the object id are not texts and stay as they are.

use fs_objects::{ObjectId, Timestamp};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::model::{Entry, EntryFilter};
use crate::reads::normalize_type;

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|b| [DIGITS[usize::from(b >> 4)], DIGITS[usize::from(b & 15)]])
        .map(char::from)
        .collect()
}

fn hash(salt: &str, field: &str, value: &str) -> String {
    let mut h = Sha256::new();
    for part in [salt, field, value] {
        h.update(part.as_bytes());
        h.update([0]);
    }
    hex(&h.finalize())
}

/// A fresh random salt as hex.
pub(super) fn new_salt() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the system has a random source");
    hex(&bytes)
}

pub(super) struct Sealed {
    salt: String,
    from: Option<Timestamp>,
    to: Option<Timestamp>,
    object: Option<ObjectId>,
    texts: [(&'static str, Option<String>); 4],
}

fn text_of(e: &Entry, field: &str) -> String {
    match field {
        "program" => e.program.clone(),
        "person" => e.person().to_string(),
        "objectType" => normalize_type(&e.object_type),
        _ => e.context.clone(),
    }
}

impl Sealed {
    pub(super) fn new(salt: String, f: &EntryFilter) -> Self {
        let seal = |field: &'static str, v: &str, norm: bool| {
            let v = if norm {
                normalize_type(v)
            } else {
                v.to_string()
            };
            let sealed = (!v.is_empty()).then(|| hash(&salt, field, &v));
            (field, sealed)
        };
        Self {
            texts: [
                seal("program", &f.program, false),
                seal("person", &f.person, false),
                seal("objectType", &f.object_type, true),
                seal("context", &f.context, false),
            ],
            from: f.from,
            to: f.to,
            object: f.object,
            salt,
        }
    }

    pub(super) fn matches(&self, e: &Entry) -> bool {
        self.from.is_none_or(|t| e.at >= t)
            && self.to.is_none_or(|t| e.at < t)
            && self.object.is_none_or(|o| o == e.object)
            && self.texts.iter().all(|(field, sealed)| {
                sealed
                    .as_ref()
                    .is_none_or(|h| *h == hash(&self.salt, field, &text_of(e, field)))
            })
    }

    pub(super) fn to_json(&self) -> Value {
        let t = |i: usize| self.texts[i].1.clone();
        json!({
            "salt": self.salt,
            "from": self.from.map(|t| t.timestamp_micros()),
            "to": self.to.map(|t| t.timestamp_micros()),
            "object": self.object.map(|o| o.to_string()),
            "program": t(0), "person": t(1), "objectType": t(2), "context": t(3),
        })
    }

    pub(super) fn from_json(v: &Value) -> Self {
        let time = |k: &str| v[k].as_i64().and_then(Timestamp::from_timestamp_micros);
        let text = |k: &'static str| (k, v[k].as_str().map(str::to_string));
        Self {
            salt: v["salt"].as_str().unwrap_or_default().to_string(),
            from: time("from"),
            to: time("to"),
            object: v["object"].as_str().and_then(|s| s.parse().ok()),
            texts: [
                text("program"),
                text("person"),
                text("objectType"),
                text("context"),
            ],
        }
    }
}
