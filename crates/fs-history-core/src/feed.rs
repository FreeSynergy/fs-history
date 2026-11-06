//! The live feed of the index: what was added, and that something was removed or blanked.
//!
//! Pattern: Observer (`tokio::sync::broadcast`) + RAII guard — every [`Subscription`] has its own
//! filter and receiver; dropping it unsubscribes. In-process only; the RPC listener comes later.

use std::sync::Arc;

use tokio::sync::broadcast::{
    self,
    error::{RecvError, TryRecvError},
};

use crate::model::{Entry, EntryFilter};

/// What a subscription reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeedEvent {
    /// A new entry that passes the filter.
    Added(Box<Entry>),
    /// Entries that pass the filter were removed or blanked; read again.
    Forgotten,
}

#[derive(Clone, Debug)]
enum Message {
    Added(Box<Entry>),
    Forgotten(Arc<[Entry]>),
}

/// The sending side, owned by the repository. Cheap to clone.
#[derive(Clone, Debug)]
pub struct Feed {
    tx: broadcast::Sender<Message>,
}

impl Default for Feed {
    fn default() -> Self {
        Self {
            tx: broadcast::channel(256).0,
        }
    }
}

impl Feed {
    /// Listen to entries that pass `filter`. Dropping the result unsubscribes.
    pub fn subscribe(&self, filter: EntryFilter) -> Subscription {
        Subscription {
            filter: filter.normalized(),
            rx: self.tx.subscribe(),
        }
    }

    pub(crate) fn added(&self, entry: &Entry) {
        // No subscriber is not an error.
        let _ = self.tx.send(Message::Added(Box::new(entry.clone())));
    }

    /// `affected` are the entries as they were before they were removed or blanked.
    pub(crate) fn forgotten(&self, affected: Vec<Entry>) {
        if !affected.is_empty() {
            let _ = self.tx.send(Message::Forgotten(affected.into()));
        }
    }
}

/// A subscription to the feed. Dropping it ends it.
#[derive(Debug)]
pub struct Subscription {
    filter: EntryFilter,
    rx: broadcast::Receiver<Message>,
}

impl Subscription {
    fn pass(&self, m: &Message) -> Option<FeedEvent> {
        match m {
            Message::Added(e) if self.filter.matches(e) => Some(FeedEvent::Added(e.clone())),
            Message::Forgotten(list) if list.iter().any(|e| self.filter.matches(e)) => {
                Some(FeedEvent::Forgotten)
            }
            _ => None,
        }
    }

    /// The next event for this filter, or `None` when the feed is gone. A subscriber that fell
    /// behind gets `Forgotten` ("read again").
    pub async fn recv(&mut self) -> Option<FeedEvent> {
        loop {
            match self.rx.recv().await {
                Ok(m) => {
                    if let Some(event) = self.pass(&m) {
                        return Some(event);
                    }
                }
                Err(RecvError::Lagged(_)) => return Some(FeedEvent::Forgotten),
                Err(RecvError::Closed) => return None,
            }
        }
    }

    /// The next event if one is already waiting.
    pub fn try_recv(&mut self) -> Option<FeedEvent> {
        loop {
            match self.rx.try_recv() {
                Ok(m) => {
                    if let Some(event) = self.pass(&m) {
                        return Some(event);
                    }
                }
                Err(TryRecvError::Lagged(_)) => return Some(FeedEvent::Forgotten),
                Err(_) => return None,
            }
        }
    }
}
