//! Tombstones: what was forgotten, kept so it stays forgotten.
//!
//! Pattern: Specification (stored) — a tombstone holds the removed activity ids (as index values,
//! so "was this id removed" is a query) and the filter that was forgotten (evaluated in memory
//! against every later entry). Written unlogged, like the entries.

use fs_store::{Envelope, ObjectId, Query, StoredObject, Timestamp};
use serde_json::{Value, json};

use super::consts::{TOMBSTONE_CONTENT, TOMBSTONE_TYPE};
use super::handle::HistoryRepo;
use crate::error::Result;
use crate::model::{Entry, EntryFilter};

fn micros(t: Option<Timestamp>) -> Value {
    t.map_or(Value::Null, |t| json!(t.timestamp_micros()))
}

fn filter_json(f: &EntryFilter) -> Value {
    json!({
        "from": micros(f.from), "to": micros(f.to), "program": f.program, "person": f.person,
        "objectType": f.object_type, "context": f.context,
        "object": f.object.map(|o| o.to_string()),
    })
}

fn filter_from(v: &Value) -> EntryFilter {
    let text = |k: &str| v[k].as_str().unwrap_or_default().to_string();
    let time = |k: &str| v[k].as_i64().and_then(Timestamp::from_timestamp_micros);
    EntryFilter {
        from: time("from"),
        to: time("to"),
        program: text("program"),
        person: text("person"),
        object_type: text("objectType"),
        context: text("context"),
        object: v["object"].as_str().and_then(|s| s.parse().ok()),
    }
}

impl HistoryRepo {
    /// Remember a forgotten `filter` and the ids of the entries removed with it.
    pub(super) async fn write_tombstone(
        &self,
        filter: &EntryFilter,
        removed: &[Entry],
    ) -> Result<()> {
        let env = Envelope::new(TOMBSTONE_TYPE, "forgotten");
        let content = json!({ "filter": filter_json(filter) });
        let mut o = StoredObject::new(env, TOMBSTONE_CONTENT, content);
        for e in removed {
            o = o.with_index("removed", e.id.to_string());
        }
        Ok(self.objects.put_unlogged(o).await?)
    }

    /// The filters forgotten so far.
    pub(super) async fn forgotten_filters(&self) -> Result<Vec<EntryFilter>> {
        let all = self
            .reads
            .query(&Query::new().of_type(TOMBSTONE_TYPE))
            .await?;
        Ok(all
            .iter()
            .map(|o| filter_from(&o.content["filter"]))
            .collect())
    }

    /// Whether an activity id was removed by a forget.
    pub(super) async fn was_removed(&self, id: ObjectId) -> Result<bool> {
        let q = Query::new()
            .of_type(TOMBSTONE_TYPE)
            .with_index("removed", id.to_string());
        Ok(self.reads.count(&q).await? > 0)
    }
}
