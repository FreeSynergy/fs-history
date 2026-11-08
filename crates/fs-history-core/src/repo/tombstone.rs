//! Tombstones: what was forgotten, kept so it stays forgotten.
//!
//! Pattern: Specification (stored) — a tombstone holds the forget time, the removed activity ids
//! (index values, so "was this id removed" is a query), the removed object ids, and the sealed
//! filter. Forgetting covers the past: an activity is forgotten if its id was removed, or if its
//! `at` is at or before the forget time and it names a removed object or matches the filter.
//! Written unlogged, like the entries.

use std::collections::HashSet;

use fs_objects::{Envelope, ObjectId, Query, StoredObject, Timestamp};
use serde_json::json;

use super::consts::{TOMBSTONE_CONTENT, TOMBSTONE_TYPE};
use super::handle::HistoryRepo;
use super::sealed::{Sealed, new_salt};
use crate::error::Result;
use crate::model::{Entry, EntryFilter};

/// One loaded tombstone.
pub(super) struct Tombstone {
    forgotten_at: Timestamp,
    objects: HashSet<ObjectId>,
    filter: Sealed,
}

impl Tombstone {
    /// Whether this tombstone keeps `e` out (not counting removed activity ids).
    pub(super) fn hides(&self, e: &Entry) -> bool {
        e.at <= self.forgotten_at && (self.objects.contains(&e.object) || self.filter.matches(e))
    }
}

impl HistoryRepo {
    /// Remember a forgotten `filter` and the entries removed with it.
    pub(super) async fn write_tombstone(
        &self,
        filter: &EntryFilter,
        removed: &[Entry],
    ) -> Result<()> {
        let sealed = Sealed::new(new_salt(), filter);
        let objects: HashSet<String> = removed.iter().map(|e| e.object.to_string()).collect();
        let content = json!({
            "forgottenAt": chrono::Utc::now().timestamp_micros(),
            "objects": objects,
            "filter": sealed.to_json(),
        });
        let mut o = StoredObject::new(
            Envelope::new(TOMBSTONE_TYPE, "forgotten"),
            TOMBSTONE_CONTENT,
            content,
        );
        for e in removed {
            o = o.with_index("removed", e.id.to_string());
        }
        Ok(self.objects.put_unlogged(o).await?)
    }

    /// All tombstones.
    pub(super) async fn tombstones(&self) -> Result<Vec<Tombstone>> {
        let all = self
            .objects
            .query_unlogged(&Query::new().of_type(TOMBSTONE_TYPE), &[TOMBSTONE_TYPE])
            .await?;
        Ok(all
            .iter()
            .map(|o| Tombstone {
                forgotten_at: o.content["forgottenAt"]
                    .as_i64()
                    .and_then(Timestamp::from_timestamp_micros)
                    .unwrap_or_default(),
                objects: o.content["objects"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str()?.parse().ok())
                    .collect(),
                filter: Sealed::from_json(&o.content["filter"]),
            })
            .collect())
    }

    /// Whether an activity id was removed by a forget.
    pub(super) async fn was_removed(&self, id: ObjectId) -> Result<bool> {
        let q = Query::new()
            .of_type(TOMBSTONE_TYPE)
            .with_index("removed", id.to_string())
            .limit(1);
        Ok(!self
            .objects
            .query_unlogged(&q, &[TOMBSTONE_TYPE])
            .await?
            .is_empty())
    }
}
