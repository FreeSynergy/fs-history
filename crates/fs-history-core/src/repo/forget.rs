//! Forgetting: remove entries and keep them out.
//!
//! Pattern: Command with a stored record — the tombstone is written before the entries go, so a
//! crash in between leaves a repeatable forget, never a leak. Only this index is touched.

use super::handle::HistoryRepo;
use super::query::for_filter;
use crate::error::{Error, Result};
use crate::model::EntryFilter;

impl HistoryRepo {
    /// Remove the entries that pass `filter` and remember it. An empty filter is refused; all-zero
    /// times count as unset. Returns how many entries were removed.
    pub async fn forget(&self, filter: &EntryFilter) -> Result<u32> {
        let filter = filter.clone().normalized();
        if filter.is_empty() {
            return Err(Error::Refused(
                "an empty filter would forget everything; name what to forget".into(),
            ));
        }
        let _guard = self.lock.lock().await;
        let doomed = self.rows(&for_filter(&filter)).await?;
        self.write_tombstone(&filter, &doomed).await?;
        for e in &doomed {
            self.objects.delete_unlogged(e.id).await?;
        }
        let count = u32::try_from(doomed.len()).unwrap_or(u32::MAX);
        self.feed.forgotten(doomed);
        Ok(count)
    }
}
