//! Reading the timeline and single entries.
//!
//! Pattern: Iterator (cursor paging) — a page is one more than asked for, to know whether a next
//! page exists; the cursor is the position of the last item, not an offset.

use fs_store::ObjectId;

use super::codec::from_object;
use super::consts::ENTRY_TYPE;
use super::handle::HistoryRepo;
use super::query::cursor_of;
use crate::error::Result;
use crate::model::{Cursor, Entry, EntryFilter, Page};

impl HistoryRepo {
    /// Entries that pass `filter`, newest first.
    pub async fn timeline(
        &self,
        filter: &EntryFilter,
        cursor: Option<&Cursor>,
    ) -> Result<Page<Entry>> {
        let mut items = self
            .page_rows(filter, cursor, self.page_size + 1, 0)
            .await?;
        let more = items.len() > self.page_size;
        items.truncate(self.page_size);
        let next = more.then(|| items.last().map(cursor_of)).flatten();
        Ok(Page { items, next })
    }

    /// One entry by activity id.
    pub async fn get(&self, id: ObjectId) -> Result<Option<Entry>> {
        match self.objects.read_unlogged(id).await? {
            Some(o) if o.envelope.object_type == ENTRY_TYPE => Ok(Some(from_object(&o)?)),
            _ => Ok(None),
        }
    }
}
