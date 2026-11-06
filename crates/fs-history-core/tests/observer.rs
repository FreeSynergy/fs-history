//! Ingest: idempotent redelivery, acknowledging only after the write, catching up in batches.

mod common;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use common::{Scripted, act, id, numbers, setup};
use fs_history_core::{EntryFilter, Error, Observer};
use fs_store::Query;
use fs_store::traits::ObjectQuery;

fn all() -> EntryFilter {
    EntryFilter::default()
}

#[tokio::test]
async fn redelivery_changes_nothing() {
    let (repo, store) = setup().await;
    let source = Scripted::with(vec![act(1, 100, "Create", 1), act(2, 200, "Update", 1)]);
    let observer = Observer::new(repo.clone(), source.clone());
    assert_eq!(observer.run_once().await.unwrap().stored, 2);

    source.rewind();
    let again = observer.run_once().await.unwrap();
    assert_eq!((again.stored, again.duplicates), (0, 2));
    assert_eq!(store.count(&Query::new()).await.unwrap(), 2);
}

#[tokio::test]
async fn a_crash_before_the_ack_gives_no_duplicates() {
    let (repo, store) = setup().await;
    let source = Scripted::with(vec![act(1, 100, "Create", 1), act(2, 200, "Update", 1)]);
    let observer = Observer::new(repo.clone(), source.clone());

    source.fail_ack.store(true, Ordering::SeqCst);
    let failed = observer.run_once().await;
    assert!(matches!(failed, Err(Error::Source(_))));
    assert_eq!(
        source.acked.load(Ordering::SeqCst),
        0,
        "nothing acknowledged"
    );
    assert_eq!(store.count(&Query::new()).await.unwrap(), 2, "but written");

    source.fail_ack.store(false, Ordering::SeqCst);
    let report = observer.run_once().await.unwrap();
    assert_eq!((report.stored, report.duplicates), (0, 2));
    assert_eq!(store.count(&Query::new()).await.unwrap(), 2);
    assert_eq!(source.acked.load(Ordering::SeqCst), 2);
    assert_eq!(
        numbers(&repo.timeline(&all(), None).await.unwrap().items),
        [2, 1]
    );
}

#[tokio::test]
async fn catch_up_loops_while_there_is_more() {
    let repo = common::repo().await;
    let items = (1..=7)
        .map(|n| act(n, 100 + n as i64, "Update", 1))
        .collect();
    let source = Scripted::with(items);
    let observer = Observer::new(repo.clone(), source.clone()).with_batch(3);
    let total = observer.catch_up().await.unwrap();
    assert_eq!(total.stored, 7);
    assert!(!total.more);
    source.push(act(8, 300, "Update", 1));
    assert_eq!(observer.catch_up().await.unwrap().stored, 1);
    assert!(repo.get(id(8)).await.unwrap().is_some());
}

#[tokio::test]
async fn forgotten_activities_are_counted_and_kept_out() {
    let repo = common::repo().await;
    let source = Scripted::with(vec![act(1, 100, "Create", 1)]);
    let observer = Observer::new(repo.clone(), source.clone());
    observer.run_once().await.unwrap();
    repo.forget(&EntryFilter {
        object: Some(id(1001)),
        ..Default::default()
    })
    .await
    .unwrap();
    source.rewind();
    let report = observer.run_once().await.unwrap();
    assert_eq!((report.stored, report.forgotten), (0, 1));
}

/// A source that always says "more" and never moves.
struct Stuck;

#[async_trait::async_trait]
impl fs_history_core::ActivitySource for Stuck {
    async fn pull(&self, _: u32) -> fs_history_core::Result<fs_history_core::Batch> {
        Ok(fs_history_core::Batch {
            items: vec![],
            last: None,
            more: true,
        })
    }
    async fn ack(&self, _: fs_store::ObjectId) -> fs_history_core::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn catch_up_errors_out_when_there_is_no_progress() {
    let observer = Observer::new(common::repo().await, Arc::new(Stuck));
    assert!(matches!(observer.catch_up().await, Err(Error::Source(_))));
}
