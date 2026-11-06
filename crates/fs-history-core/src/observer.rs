//! The observer: pulls from a source and records.
//!
//! Pattern: Template method — `run_once` is one fixed sequence (pull, record, acknowledge only after
//! the write); `catch_up` repeats it while the source has more. Because recording is idempotent, a
//! crash before the acknowledgement only costs a redelivery.

use std::sync::Arc;

use crate::error::Result;
use crate::repo::{HistoryRepo, Outcome};
use crate::source::ActivitySource;

/// What one run did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunReport {
    /// Written to the index.
    pub stored: usize,
    /// Already there.
    pub duplicates: usize,
    /// Kept out by a forget.
    pub forgotten: usize,
    /// Of a type that is never read.
    pub refused: usize,
    /// The source has more waiting.
    pub more: bool,
}

impl RunReport {
    fn add(&mut self, o: Outcome) {
        match o {
            Outcome::Stored => self.stored += 1,
            Outcome::Duplicate => self.duplicates += 1,
            Outcome::Forgotten => self.forgotten += 1,
            Outcome::Refused => self.refused += 1,
        }
    }

    fn merge(&mut self, other: Self) {
        self.stored += other.stored;
        self.duplicates += other.duplicates;
        self.forgotten += other.forgotten;
        self.refused += other.refused;
        self.more = other.more;
    }
}

/// Feeds a [`HistoryRepo`] from an [`ActivitySource`].
pub struct Observer {
    repo: HistoryRepo,
    source: Arc<dyn ActivitySource>,
    batch: u32,
}

impl Observer {
    /// An observer pulling 100 activities at a time.
    pub fn new(repo: HistoryRepo, source: Arc<dyn ActivitySource>) -> Self {
        Self {
            repo,
            source,
            batch: 100,
        }
    }

    /// Activities per pull (at least 1).
    #[must_use]
    pub fn with_batch(mut self, n: u32) -> Self {
        self.batch = n.max(1);
        self
    }

    /// Pull one batch, record it, then acknowledge. If recording or acknowledging fails, nothing
    /// is lost: the source delivers the batch again and the second recording changes nothing.
    pub async fn run_once(&self) -> Result<RunReport> {
        let batch = self.source.pull(self.batch).await?;
        let mut report = RunReport {
            more: batch.more,
            ..RunReport::default()
        };
        for outcome in self.repo.record(batch.items).await? {
            report.add(outcome);
        }
        if let Some(last) = batch.last {
            self.source.ack(last).await?;
        }
        Ok(report)
    }

    /// Run until the source has nothing more; the reports are added up.
    pub async fn catch_up(&self) -> Result<RunReport> {
        let mut total = RunReport::default();
        loop {
            let one = self.run_once().await?;
            let more = one.more;
            total.merge(one);
            if !more {
                return Ok(total);
            }
        }
    }
}
