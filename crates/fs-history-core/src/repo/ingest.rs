//! Recording activities delivered by a source.
//!
//! Pattern: Template method (the fixed order of checks) — normalise and refuse never-read types,
//! skip what is stored, fill log-only verbs from the object's latest entry, decide on forgetting
//! while nothing is cleared yet, then blank (delete, or a change after a delete) and write.
//! Idempotent by activity id, so a redelivery changes nothing.

use super::codec::to_object;
use super::handle::HistoryRepo;
use super::tombstone::Tombstone;
use crate::error::Result;
use crate::model::{Entry, SourceActivity};
use crate::reads::{is_never_read, strip_namespace};

/// What happened to one delivered activity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Written to the index.
    Stored,
    /// Already there (a redelivery).
    Duplicate,
    /// Its id, its object or its filter was forgotten.
    Forgotten,
    /// Of a type that is never read, or of an unknown type from a source that does not filter.
    Refused,
}

impl HistoryRepo {
    /// Record `items` in order from a source that does not filter types: a log-only verb whose
    /// type stays unknown is refused.
    pub async fn record(&self, items: Vec<SourceActivity>) -> Result<Vec<Outcome>> {
        self.record_from(items, false).await
    }

    /// Record `items` from a source that delivers only the types asked for (`filters_types`).
    pub async fn record_filtered(&self, items: Vec<SourceActivity>) -> Result<Vec<Outcome>> {
        self.record_from(items, true).await
    }

    async fn record_from(
        &self,
        items: Vec<SourceActivity>,
        filtered: bool,
    ) -> Result<Vec<Outcome>> {
        let _guard = self.lock.lock().await;
        let tombs = self.tombstones().await?;
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(self.record_one(item, &tombs, filtered).await?);
        }
        Ok(out)
    }

    async fn record_one(
        &self,
        item: SourceActivity,
        tombs: &[Tombstone],
        filtered: bool,
    ) -> Result<Outcome> {
        if is_never_read(&item.object_type) {
            return Ok(Outcome::Refused);
        }
        if self.objects.read_unlogged(item.id).await?.is_some() {
            return Ok(Outcome::Duplicate);
        }
        let log_only = item.is_log_only();
        let mut entry = item.into_entry();
        entry.object_type = strip_namespace(&entry.object_type).to_string();
        if log_only {
            self.fill_from_latest(&mut entry).await?;
        }
        if is_never_read(&entry.object_type)
            || (log_only && !filtered && entry.object_type.is_empty())
        {
            return Ok(Outcome::Refused);
        }
        // Forgetting is decided before anything is cleared: a delete must not hide its context.
        let forgotten = tombs.iter().any(|t| t.hides(&entry)) || self.was_removed(entry.id).await?;
        if entry.kind == "Delete" {
            entry.name.clear();
            entry.context.clear();
            self.mark_deleted(entry.object).await?;
            self.blank_object(entry.object).await?;
        } else if entry.kind == "Create" {
            self.mark_alive(entry.object).await?;
        } else if self.is_deleted(entry.object).await? {
            entry.name.clear();
            entry.context.clear();
        }
        if forgotten {
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
