//! Which objects are deleted, independent of any entry.
//!
//! Pattern: Registry (stored) — one small unlogged marker per deleted object, with the object's own
//! id as its id. It outlives the entries (a hidden or forgotten `Delete`), so a later change stays
//! blank until a new `Create` removes the marker. It holds no name and no context.

use fs_store::{Envelope, ObjectId, StoredObject, Timestamp};
use serde_json::json;

use super::consts::{DELETED_TYPE, ENTRY_CONTENT};
use super::handle::HistoryRepo;
use crate::error::Result;

impl HistoryRepo {
    /// Remember the delete with its time; a late delivery of an older one does not move it back.
    pub(super) async fn mark_deleted(&self, object: ObjectId, at: Timestamp) -> Result<()> {
        if let Some(old) = self.objects.read_unlogged(object).await? {
            if old.envelope.object_type == DELETED_TYPE && old.envelope.published >= at {
                return Ok(());
            }
        }
        let mut env = Envelope::new(DELETED_TYPE, "deleted");
        env.id = object;
        env.published = at;
        env.updated = at;
        let o = StoredObject::new(env, ENTRY_CONTENT, json!({}));
        Ok(self.objects.put_unlogged(o).await?)
    }

    /// Only a `Create` newer than the delete brings an object back: a late redelivery does not.
    pub(super) async fn mark_alive(&self, object: ObjectId, at: Timestamp) -> Result<()> {
        if let Some(old) = self.objects.read_unlogged(object).await? {
            if old.envelope.object_type == DELETED_TYPE && old.envelope.published < at {
                self.objects.delete_unlogged(object).await?;
            }
        }
        Ok(())
    }

    pub(super) async fn is_deleted(&self, object: ObjectId) -> Result<bool> {
        let found = self.objects.read_unlogged(object).await?;
        Ok(found.is_some_and(|o| o.envelope.object_type == DELETED_TYPE))
    }
}
