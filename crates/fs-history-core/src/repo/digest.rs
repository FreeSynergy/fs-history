//! The digest over the index.
//!
//! Pattern: Iterator (cursor paging) + pure function — the page is a run of entries that ends only
//! where the gap to the next entry exceeds the window (`digest::cut`), grouped by `digest::digest`.
//! One group is never split; a very busy object can make a page longer than the page size.

use super::handle::HistoryRepo;
use super::query::cursor_of;
use crate::digest::{cut, digest};
use crate::error::Result;
use crate::model::{Cursor, DigestEntry, EntryFilter, Page};

impl HistoryRepo {
    /// Entries that pass `filter` as digest lines, newest first.
    pub async fn digest(
        &self,
        filter: &EntryFilter,
        window_minutes: u32,
        cursor: Option<&Cursor>,
    ) -> Result<Page<DigestEntry>> {
        let mut got = Vec::new();
        let end = loop {
            let chunk = self
                .page_rows(filter, cursor, self.page_size, got.len())
                .await?;
            let exhausted = chunk.len() < self.page_size;
            got.extend(chunk);
            if let Some(k) = cut(&got, self.page_size, window_minutes, exhausted) {
                break k;
            }
        };
        let next = (end < got.len()).then(|| cursor_of(&got[end - 1]));
        got.truncate(end);
        Ok(Page {
            items: digest(&got, window_minutes),
            next,
        })
    }
}
