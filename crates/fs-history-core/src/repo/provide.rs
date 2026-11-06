//! `HistoryRepo` as a provider of the [`History`] trait.
//!
//! Pattern: Adapter — each trait method calls the inherent method of the same concern (inherent
//! methods win in paths like `HistoryRepo::timeline`), so callers with and without the trait agree.

use async_trait::async_trait;
use fs_interfaces::Capabilities;
use fs_store::ObjectId;

use super::handle::HistoryRepo;
use crate::error::Result;
use crate::feed::Subscription;
use crate::model::{Cursor, DigestEntry, Entry, EntryFilter, Page};
use crate::traits::History;

/// Capability ids that `HistoryRepo` provides: `observe` is the ingest side (see `Observer`).
const SUPPORTED: [&str; 5] = [
    "history.observe",
    "history.read",
    "history.digest",
    "history.forget",
    "history.subscribe",
];

#[async_trait]
impl History for HistoryRepo {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new("fs-history-core");
        for id in SUPPORTED {
            c.supported(id);
        }
        c
    }
    async fn timeline(&self, filter: &EntryFilter, cursor: Option<&Cursor>) -> Result<Page<Entry>> {
        HistoryRepo::timeline(self, filter, cursor).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<Entry>> {
        HistoryRepo::get(self, id).await
    }
    async fn digest(
        &self,
        filter: &EntryFilter,
        window_minutes: u32,
        cursor: Option<&Cursor>,
    ) -> Result<Page<DigestEntry>> {
        HistoryRepo::digest(self, filter, window_minutes, cursor).await
    }
    async fn forget(&self, filter: &EntryFilter) -> Result<u32> {
        HistoryRepo::forget(self, filter).await
    }
    async fn subscribe(&self, filter: &EntryFilter) -> Result<Subscription> {
        Ok(self.watch(filter.clone()))
    }
}
