//! [`HubSource`]: the hub as an activity source.
//!
//! Pattern: Adapter — `HubSession::pull` / `ack` behind the core's `ActivitySource` port. The hub
//! keeps the pull cursor and moves it only on `ack`, which is what lets the observer acknowledge
//! after writing. The hub's reader refuses an answer of more than about 8 million words, and that
//! error ends the whole RPC connection (checked against a real hub), so a smaller retry on the same
//! session cannot work. Instead a pull asks for at most [`DEFAULT_MAX_BATCH`] activities, and a
//! too-large answer is reported as such.

use async_trait::async_trait;
use fs_history_core::{ActivitySource, Batch, Error, Result};
use fs_hub_client::HubSession;
use fs_objects::ObjectId;

use crate::map::{summarise, to_core_id};

/// Most activities one pull asks for by default: 16 notes of 1 MiB stay far below the hub's limit.
pub const DEFAULT_MAX_BATCH: u32 = 16;

/// Pulls other programs' activities from a registered hub session.
#[derive(Clone)]
pub struct HubSource {
    session: HubSession,
    max_batch: u32,
}

impl HubSource {
    /// A source over a session registered with [`manifest`](crate::manifest).
    pub fn new(session: HubSession) -> Self {
        Self {
            session,
            max_batch: DEFAULT_MAX_BATCH,
        }
    }

    /// Ask for at most `n` activities per pull (at least 1).
    #[must_use]
    pub fn with_max_batch(mut self, n: u32) -> Self {
        self.max_batch = n.max(1);
        self
    }
}

fn hub_error(e: impl std::fmt::Display) -> Error {
    Error::Source(e.to_string())
}

#[async_trait]
impl ActivitySource for HubSource {
    async fn pull(&self, limit: u32) -> Result<Batch> {
        let asked = limit.min(self.max_batch).max(1);
        let batch = self.session.pull(None, asked).await.map_err(|e| {
            let text = e.to_string();
            if text.contains("too large") {
                Error::Source(format!(
                    "{text} (asked for {asked}; the connection is closed by this error: \
                     connect again with a smaller `with_max_batch`)"
                ))
            } else {
                Error::Source(text)
            }
        })?;
        Ok(Batch {
            items: batch
                .activities
                .iter()
                .map(summarise)
                .collect::<Result<_>>()?,
            last: batch.last.map(to_core_id).transpose()?,
            more: batch.more,
        })
    }

    fn filters_types(&self) -> bool {
        true
    }

    async fn ack(&self, last: ObjectId) -> Result<()> {
        let id = last.to_string().parse().map_err(hub_error)?;
        self.session.ack(id).await.map_err(hub_error)
    }
}
