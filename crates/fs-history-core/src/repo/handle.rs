//! [`HistoryRepo`]: the store-backed implementation of [`History`](crate::History).
//!
//! Pattern: Repository + Facade — one small type hides the object store and the entry rules. Its
//! collaborators are injected as the narrow store traits `UnloggedObjects` (writes, no activity
//! log) and `ObjectQuery` (reads), so each can be swapped.

use std::sync::Arc;

use fs_store::traits::{ObjectQuery, UnloggedObjects};
use tokio::sync::Mutex;

use crate::feed::{Feed, Subscription};
use crate::model::EntryFilter;

/// History over an object store. Cheap to clone; clones share one write lock and one feed.
#[derive(Clone)]
pub struct HistoryRepo {
    pub(super) objects: Arc<dyn UnloggedObjects>,
    pub(super) reads: Arc<dyn ObjectQuery>,
    pub(super) page_size: usize,
    pub(super) max_page: usize,
    pub(super) feed: Feed,
    /// Serialises the writes that read first (ingest, blanking, forget).
    pub(super) lock: Arc<Mutex<()>>,
}

impl HistoryRepo {
    /// History over the two narrow store traits. Give it a store of its own: the unlogged writes
    /// refuse ids that have logged history. A `TursoStore` serves as both.
    pub fn new(objects: Arc<dyn UnloggedObjects>, reads: Arc<dyn ObjectQuery>) -> Self {
        Self {
            objects,
            reads,
            page_size: 100,
            max_page: crate::digest::MAX_PAGE_ENTRIES,
            feed: Feed::default(),
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Items per page of `timeline` and `digest` (default 100, at least 1).
    #[must_use]
    pub fn with_page_size(mut self, n: usize) -> Self {
        self.page_size = n.max(1);
        self
    }

    /// Most entries one digest page may hold (default 10 000); a longer run is cut and flagged.
    #[must_use]
    pub fn with_digest_cap(mut self, n: usize) -> Self {
        self.max_page = n.max(1);
        self
    }

    /// Listen to the feed (the same as `History::subscribe`, without the trait).
    pub fn watch(&self, filter: EntryFilter) -> Subscription {
        self.feed.subscribe(filter)
    }
}
