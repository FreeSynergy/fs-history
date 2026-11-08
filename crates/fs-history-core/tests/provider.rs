//! The provider rule (ADR 0045): every provider has every method; what it lacks says `Unsupported`.

mod common;

use std::sync::Arc;

use async_trait::async_trait;
use common::{act, id, repo};
use fs_history_core::{Cursor, Entry, EntryFilter, Error, FeedEvent, History, Page, Result};
use fs_interfaces::Capabilities;
use fs_objects::ObjectId;

/// A provider that can only read.
struct ReadOnly;

#[async_trait]
impl History for ReadOnly {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new("read-only");
        c.supported("history.read");
        c
    }
    async fn timeline(&self, _: &EntryFilter, _: Option<&Cursor>) -> Result<Page<Entry>> {
        Ok(Page {
            items: vec![],
            next: None,
        })
    }
    async fn get(&self, _: ObjectId) -> Result<Option<Entry>> {
        Ok(None)
    }
}

#[tokio::test]
async fn what_a_provider_does_not_override_is_unsupported() {
    let p: Arc<dyn History> = Arc::new(ReadOnly);
    let any = EntryFilter::default();
    assert!(matches!(
        p.digest(&any, 5, None).await,
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(p.forget(&any).await, Err(Error::Unsupported(_))));
    assert!(matches!(
        p.subscribe(&any).await,
        Err(Error::Unsupported(_))
    ));
    assert!(
        p.capabilities().why_not("history.forget").is_none(),
        "not listed, not claimed"
    );
    assert!(!p.capabilities().is_supported("history.forget"));
}

#[tokio::test]
async fn the_repo_reports_every_capability_of_the_contract() {
    let repo = repo().await;
    let caps = History::capabilities(&repo);
    for id in [
        "history.observe",
        "history.read",
        "history.digest",
        "history.forget",
        "history.subscribe",
    ] {
        assert!(caps.is_supported(id), "{id}");
    }
}

#[tokio::test]
async fn a_subscription_hears_only_its_filter_and_dropping_it_unsubscribes() {
    let repo = repo().await;
    let mut tasks = repo
        .subscribe(&EntryFilter {
            program: "tasks".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let mut everything = repo.subscribe(&EntryFilter::default()).await.unwrap();
    let mut task = act(2, 200, "Create", 2);
    task.program = "tasks".into();
    repo.record(vec![act(1, 100, "Create", 1), task])
        .await
        .unwrap();

    match everything.recv().await {
        Some(FeedEvent::Added(e)) => assert_eq!(e.id, id(1)),
        other => panic!("{other:?}"),
    }
    match tasks.recv().await {
        Some(FeedEvent::Added(e)) => assert_eq!(e.program, "tasks"),
        other => panic!("{other:?}"),
    }
    assert_eq!(tasks.try_recv(), None);
    drop(tasks);
    drop(everything);
    repo.record(vec![act(3, 300, "Update", 1)]).await.unwrap(); // nobody listening: no error
}

#[tokio::test]
async fn the_support_report_lists_the_capabilities() {
    let repo = repo().await;
    let report = History::support_report(&repo).await.unwrap();
    assert_eq!(report.entries().len(), 5);
    assert!(report.is_supported("history.forget"));
    // A provider that only reads still has the method.
    let p: Arc<dyn History> = Arc::new(ReadOnly);
    assert!(
        p.support_report()
            .await
            .unwrap()
            .is_supported("history.read")
    );
}
