//! Reading entries back from the store.
//!
//! Pattern: Repository (read side) — small helpers that run a query and decode the entries.

use fs_objects::{ObjectId, Query};

use super::codec::{from_object, to_object};
use super::handle::HistoryRepo;
use super::query::{after, for_filter, for_object};
use crate::error::Result;
use crate::model::{Cursor, Entry, EntryFilter};

impl HistoryRepo {
    pub(super) async fn rows(&self, q: &Query) -> Result<Vec<Entry>> {
        self.reads.query(q).await?.iter().map(from_object).collect()
    }

    /// `limit` entries of the filter after the cursor, skipping `offset` of them.
    pub(super) async fn page_rows(
        &self,
        filter: &EntryFilter,
        cursor: Option<&Cursor>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Entry>> {
        let mut q = for_filter(filter);
        if let Some(c) = cursor {
            q = after(q, c)?;
        }
        self.rows(&q.limit(limit).offset(offset)).await
    }

    /// The newest entry of an object, if there is one.
    pub(super) async fn latest_of(&self, object: ObjectId) -> Result<Option<Entry>> {
        Ok(self.rows(&for_object(object).limit(1)).await?.pop())
    }

    /// Blank name and context of every entry of an object (it was deleted); the type stays.
    pub(super) async fn blank_object(&self, object: ObjectId) -> Result<()> {
        let affected: Vec<Entry> = self
            .rows(&for_object(object))
            .await?
            .into_iter()
            .filter(|e| !e.name.is_empty() || !e.context.is_empty())
            .collect();
        for e in &affected {
            self.objects
                .put_unlogged(to_object(&super::codec::blanked(e)))
                .await?;
        }
        self.feed.forgotten(affected);
        Ok(())
    }
}
