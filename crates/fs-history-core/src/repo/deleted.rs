//! Which objects are deleted, independent of any entry.
//!
//! Pattern: Registry (stored) — one small unlogged marker per deleted object, with the object's own
//! id as its id. It outlives the entries (a hidden or forgotten `Delete`), so a later change stays
//! blank until a new `Create` removes the marker. It holds no name and no context.

use fs_store::{Envelope, ObjectId, StoredObject};
use serde_json::json;

use super::consts::{DELETED_TYPE, ENTRY_CONTENT};
use super::handle::HistoryRepo;
use crate::error::Result;

impl HistoryRepo {
    pub(super) async fn mark_deleted(&self, object: ObjectId) -> Result<()> {
        let mut env = Envelope::new(DELETED_TYPE, "deleted");
        env.id = object;
        let o = StoredObject::new(env, ENTRY_CONTENT, json!({}));
        Ok(self.objects.put_unlogged(o).await?)
    }

    pub(super) async fn mark_alive(&self, object: ObjectId) -> Result<()> {
        if self.is_deleted(object).await? {
            self.objects.delete_unlogged(object).await?;
        }
        Ok(())
    }

    pub(super) async fn is_deleted(&self, object: ObjectId) -> Result<bool> {
        let found = self.objects.read_unlogged(object).await?;
        Ok(found.is_some_and(|o| o.envelope.object_type == DELETED_TYPE))
    }
}
