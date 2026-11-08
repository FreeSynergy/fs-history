//! Shared helpers of the integration tests: a real store, activity builders, a scripted source.

#![allow(dead_code)] // each test file uses a different subset

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use async_trait::async_trait;
use fs_history_core::{ActivitySource, Batch, Error, HistoryRepo, Result, SourceActivity};
use fs_objects::{ObjectId, Timestamp, TursoStore};

/// A history over a fresh in-memory store, and the store itself (to look at what is really kept).
pub async fn setup() -> (HistoryRepo, Arc<TursoStore>) {
    let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
    (HistoryRepo::new(store.clone(), store.clone()), store)
}

pub async fn repo() -> HistoryRepo {
    setup().await.0
}

/// A `UUIDv7` that sorts by `n`.
pub fn id(n: u64) -> ObjectId {
    format!("00000000-0000-7000-8000-{n:012x}").parse().unwrap()
}

/// A point in time, `s` seconds after the epoch.
pub fn at(s: i64) -> Timestamp {
    Timestamp::from_timestamp(s, 0).unwrap()
}

/// An activity: number `n` (its id), at `s` seconds, verb `kind`, about object number `obj`.
pub fn act(n: u64, s: i64, kind: &str, obj: u64) -> SourceActivity {
    SourceActivity {
        id: id(n),
        at: at(s),
        program: "calendar".into(),
        kind: kind.into(),
        object: id(1000 + obj),
        object_type: "Event".into(),
        name: format!("Meeting {obj}"),
        attributed_to: "https://example.org/people/ada".into(),
        context: "work".into(),
        actor: String::new(),
        at_is_received: true,
    }
}

/// The id numbers of a list of entries.
pub fn numbers(entries: &[fs_history_core::Entry]) -> Vec<u64> {
    entries
        .iter()
        .map(|e| u64::from_str_radix(&e.id.to_string()[24..], 16).unwrap())
        .collect()
}

/// A source with a fixed list. `ack` moves the position; `fail_ack` makes it fail first.
#[derive(Default)]
pub struct Scripted {
    pub items: Mutex<Vec<SourceActivity>>,
    pub acked: AtomicUsize,
    pub fail_ack: AtomicBool,
}

impl Scripted {
    pub fn with(items: Vec<SourceActivity>) -> Arc<Self> {
        Arc::new(Self {
            items: Mutex::new(items),
            ..Self::default()
        })
    }
    /// The hub forgets the acknowledgement: everything is delivered again.
    pub fn rewind(&self) {
        self.acked.store(0, Ordering::SeqCst);
    }
    pub fn push(&self, a: SourceActivity) {
        self.items.lock().unwrap().push(a);
    }
}

#[async_trait]
impl ActivitySource for Scripted {
    async fn pull(&self, limit: u32) -> Result<Batch> {
        let items = self.items.lock().unwrap();
        let from = self.acked.load(Ordering::SeqCst);
        let to = (from + limit as usize).min(items.len());
        Ok(Batch {
            items: items[from..to].to_vec(),
            last: items[from..to].last().map(|a| a.id),
            more: to < items.len(),
        })
    }
    async fn ack(&self, last: ObjectId) -> Result<()> {
        if self.fail_ack.load(Ordering::SeqCst) {
            return Err(Error::Source("ack failed on purpose".into()));
        }
        let items = self.items.lock().unwrap();
        let pos = items.iter().position(|a| a.id == last).unwrap();
        self.acked.store(pos + 1, Ordering::SeqCst);
        Ok(())
    }
}
