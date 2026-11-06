//! The [`History`] contract: read the timeline, forget, subscribe.
//!
//! Pattern: Repository (interface) + Null Object — callers use `dyn History`. Like a parent class
//! (ADR 0045): every provider has every method; one it does not override answers an explicit
//! `Unsupported`. The Rust form of `History` in `history.capnp`. Writing entries is not part of it:
//! only an `Observer` feeds the index (`history.observe`).

use async_trait::async_trait;
use fs_interfaces::Capabilities;
use fs_store::ObjectId;

use crate::error::Result;
use crate::feed::Subscription;
use crate::model::{Cursor, DigestEntry, Entry, EntryFilter, Page};
use crate::unsupported::not_possible;

/// What happened, newest first. Interface status: draft (ADR 0045).
#[async_trait]
pub trait History: Send + Sync {
    /// What this provider can do, per capability id.
    fn capabilities(&self) -> Capabilities;
    /// The provider rule's report (`supportReport`): every capability with a reason where not offered.
    async fn support_report(&self) -> Result<Capabilities> {
        Ok(self.capabilities())
    }
    /// `history.read`: entries that pass `filter`, newest first, one page.
    async fn timeline(&self, filter: &EntryFilter, cursor: Option<&Cursor>) -> Result<Page<Entry>>;
    /// `history.read`: one entry by activity id.
    async fn get(&self, id: ObjectId) -> Result<Option<Entry>>;

    /// `history.digest`: entries of one object and program, no more than `window_minutes` apart,
    /// as one line; newest first. A page ends only between groups.
    async fn digest(
        &self,
        _filter: &EntryFilter,
        _window_minutes: u32,
        _cursor: Option<&Cursor>,
    ) -> Result<Page<DigestEntry>> {
        not_possible("history.digest")
    }
    /// `history.forget`: remove the matching entries and keep them out if delivered again. An
    /// empty filter is refused. Only this index is touched, never the hub's log. Returns the count.
    async fn forget(&self, _filter: &EntryFilter) -> Result<u32> {
        not_possible("history.forget")
    }
    /// `history.subscribe`: live view of what the filter lets through.
    async fn subscribe(&self, _filter: &EntryFilter) -> Result<Subscription> {
        not_possible("history.subscribe")
    }
}
