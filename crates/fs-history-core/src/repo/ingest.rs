//! Recording activities delivered by a source.
//!
//! Pattern: Template method (the fixed order of checks) — refuse never-read types, skip what is
//! stored, fill log-only verbs from the object's latest entry, blank on delete, skip what was
//! forgotten, then write. Idempotent by activity id, so a redelivery changes nothing.

use super::codec::to_object;
use super::handle::HistoryRepo;
use crate::error::Result;
use crate::model::{Entry, SourceActivity};
use crate::reads::NEVER_READ;

/// What happened to one delivered activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Written to the index.
    Stored,
    /// Already there (a redelivery).
    Duplicate,
    /// Its id or its filter was forgotten.
    Forgotten,
    /// Of a type that is never read.
    Refused,
}

impl HistoryRepo {
    /// Record `items` in order; returns one outcome per item.
    pub async fn record(&self, items: Vec<SourceActivity>) -> Result<Vec<Outcome>> {
        let _guard = self.lock.lock().await;
        let filters = self.forgotten_filters().await?;
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(self.record_one(item, &filters).await?);
        }
        Ok(out)
    }

    async fn record_one(
        &self,
        item: SourceActivity,
        filters: &[crate::model::EntryFilter],
    ) -> Result<Outcome> {
        if NEVER_READ.contains(&item.object_type.as_str()) {
            return Ok(Outcome::Refused);
        }
        if self.objects.read_unlogged(item.id).await?.is_some() {
            return Ok(Outcome::Duplicate);
        }
        let log_only = item.is_log_only();
        let mut entry = item.into_entry();
        if log_only {
            self.fill_from_latest(&mut entry).await?;
        }
        if entry.kind == "Delete" {
            entry.name.clear();
            entry.context.clear();
            self.blank_object(entry.object).await?;
        }
        if filters.iter().any(|f| f.matches(&entry)) || self.was_removed(entry.id).await? {
            return Ok(Outcome::Forgotten);
        }
        self.objects.put_unlogged(to_object(&entry)).await?;
        self.feed.added(&entry);
        Ok(Outcome::Stored)
    }

    /// Log-only verbs carry little: take what is empty from the latest entry of the object.
    async fn fill_from_latest(&self, e: &mut Entry) -> Result<()> {
        let Some(known) = self.latest_of(e.object).await? else {
            return Ok(());
        };
        for (mine, theirs) in [
            (&mut e.object_type, known.object_type),
            (&mut e.name, known.name),
            (&mut e.attributed_to, known.attributed_to),
            (&mut e.context, known.context),
        ] {
            if mine.is_empty() {
                *mine = theirs;
            }
        }
        Ok(())
    }
}
