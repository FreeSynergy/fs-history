//! [`HubSource`]: the hub as an activity source.
//!
//! Pattern: Adapter — `HubSession::pull` / `ack` behind the core's `ActivitySource` port. The hub
//! keeps the pull cursor and moves it only on `ack`, which is what lets the observer acknowledge
//! after writing.

use async_trait::async_trait;
use fs_history_core::{ActivitySource, Batch, Error, Result};
use fs_hub_client::HubSession;
use fs_store::ObjectId;

use crate::map::{summarise, to_core_id};

/// Pulls other programs' activities from a registered hub session.
#[derive(Clone)]
pub struct HubSource {
    session: HubSession,
}

impl HubSource {
    /// A source over a session registered with [`manifest`](crate::manifest).
    pub fn new(session: HubSession) -> Self {
        Self { session }
    }
}

fn hub_error(e: impl std::fmt::Display) -> Error {
    Error::Source(e.to_string())
}

#[async_trait]
impl ActivitySource for HubSource {
    async fn pull(&self, limit: u32) -> Result<Batch> {
        let batch = self.session.pull(None, limit).await.map_err(hub_error)?;
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

    async fn ack(&self, last: ObjectId) -> Result<()> {
        let id = last.to_string().parse().map_err(hub_error)?;
        self.session.ack(id).await.map_err(hub_error)
    }
}
