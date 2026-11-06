//! The port history is fed through.
//!
//! Pattern: Strategy (port) — the core knows no hub: anything that can hand over batches of
//! summarised activities and take an acknowledgement is a source. The hub is one adapter.

use async_trait::async_trait;
use fs_store::ObjectId;

use crate::error::Result;
use crate::model::SourceActivity;

/// One batch from a source.
#[derive(Clone, Debug, Default)]
pub struct Batch {
    /// The activities, oldest first.
    pub items: Vec<SourceActivity>,
    /// The source scanned up to here (it may be past the last item); hand it to `ack`.
    pub last: Option<ObjectId>,
    /// More are waiting.
    pub more: bool,
}

/// Where activities come from. Pulling twice without acknowledging gives the same again.
#[async_trait]
pub trait ActivitySource: Send + Sync {
    /// Up to `limit` activities after the acknowledged position.
    async fn pull(&self, limit: u32) -> Result<Batch>;
    /// Whether the source delivers only types history asked for (the hub does). A log-only verb
    /// of unknown type is kept only from such a source.
    fn filters_types(&self) -> bool {
        false
    }
    /// Everything up to `last` is stored; do not deliver it again.
    async fn ack(&self, last: ObjectId) -> Result<()>;
}
