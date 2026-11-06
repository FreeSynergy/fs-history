//! The digest over the index.
//!
//! Pattern: Iterator (cursor paging) + pure function — the page is a run of entries that ends only
//! where the gap to the next entry exceeds the window (`digest::cut`), grouped by `digest::digest`.
//! One group is never split; a very busy object can make a page longer than the page size.

use super::handle::HistoryRepo;
use super::query::cursor_of;
use crate::digest::{cut, digest_page};
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
        let page = loop {
            // Keyset per chunk: a concurrent insert cannot shift what was already read.
            let after = got.last().map(cursor_of).or_else(|| cursor.cloned());
            let chunk = self
                .page_rows(filter, after.as_ref(), self.page_size, 0)
                .await?;
            let exhausted = chunk.len() < self.page_size;
            got.extend(chunk);
            if let Some(c) = cut(
                &got,
                self.page_size,
                window_minutes,
                exhausted,
                self.max_page,
            ) {
                break c;
            }
        };
        let next = (page.end < got.len()).then(|| cursor_of(&got[page.end - 1]));
        got.truncate(page.end);
        Ok(Page {
            items: digest_page(&got, window_minutes, &page.continued),
            next,
        })
    }
}
