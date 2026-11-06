//! Deleted objects stay blank without entries; types are indexed normalised; the cap bounds reads.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use common::{act, at, id, repo};
use fs_history_core::{EntryFilter, HistoryRepo, Outcome};
use fs_store::traits::ObjectQuery;
use fs_store::{Query, StoredObject, TursoStore};

const FUTURE: i64 = 4_000_000_000;

fn blank(e: &fs_history_core::Entry) -> bool {
    e.name.is_empty() && e.context.is_empty()
}

#[tokio::test]
async fn a_hidden_delete_keeps_the_object_blank_until_a_create() {
    let repo = repo().await;
    repo.record(vec![act(1, 100, "Create", 1)]).await.unwrap();
    repo.forget(&EntryFilter {
        context: "work".into(),
        ..Default::default()
    })
    .await
    .unwrap();
    // The delete itself is forgotten, not stored.
    assert_eq!(
        repo.record(vec![act(2, 200, "Delete", 1)]).await.unwrap(),
        [Outcome::Forgotten]
    );
    let mut invite = act(4, FUTURE + 1, "Invite", 1);
    (
        invite.object_type,
        invite.name,
        invite.attributed_to,
        invite.context,
    ) = Default::default();
    repo.record_filtered(vec![act(3, FUTURE, "Update", 1), invite])
        .await
        .unwrap();
    assert!(
        blank(&repo.get(id(3)).await.unwrap().unwrap()),
        "update stays blank"
    );
    assert!(
        blank(&repo.get(id(4)).await.unwrap().unwrap()),
        "invite inherits blank"
    );
    repo.record(vec![
        act(5, FUTURE + 2, "Create", 1),
        act(6, FUTURE + 3, "Update", 1),
    ])
    .await
    .unwrap();
    assert_eq!(repo.get(id(6)).await.unwrap().unwrap().name, "Meeting 1");
}

#[tokio::test]
async fn a_forgotten_range_that_removes_the_stored_delete_changes_nothing() {
    let repo = repo().await;
    repo.record(vec![act(1, 100, "Create", 1), act(2, 200, "Delete", 1)])
        .await
        .unwrap();
    let range = EntryFilter {
        from: Some(at(150)),
        to: Some(at(250)),
        ..Default::default()
    };
    assert_eq!(repo.forget(&range).await.unwrap(), 1);
    assert!(repo.get(id(2)).await.unwrap().is_none());
    repo.record(vec![act(3, FUTURE, "Update", 1)])
        .await
        .unwrap();
    assert!(blank(&repo.get(id(3)).await.unwrap().unwrap()));
}

#[tokio::test]
async fn type_filters_match_whatever_the_spelling() {
    let repo = repo().await;
    repo.record(vec![act(1, 100, "Create", 1)]).await.unwrap();
    for spelling in ["event", "as:Event", "EVENT"] {
        let f = EntryFilter {
            object_type: spelling.into(),
            ..Default::default()
        };
        let page = repo.timeline(&f, None).await.unwrap();
        assert_eq!(page.items.len(), 1, "{spelling}");
        assert_eq!(page.items[0].object_type, "Event", "the original is shown");
    }
    let f = EntryFilter {
        object_type: "event".into(),
        ..Default::default()
    };
    assert_eq!(repo.forget(&f).await.unwrap(), 1);
    assert!(repo.get(id(1)).await.unwrap().is_none());
}

/// Counts the rows a query returns.
struct Counting(Arc<TursoStore>, Arc<AtomicUsize>);

#[async_trait]
impl ObjectQuery for Counting {
    async fn query(&self, q: &Query) -> fs_store::Result<Vec<StoredObject>> {
        let rows = self.0.query(q).await?;
        self.1.fetch_add(rows.len(), Ordering::SeqCst);
        Ok(rows)
    }
    async fn count(&self, q: &Query) -> fs_store::Result<usize> {
        self.0.count(q).await
    }
}

#[tokio::test]
async fn the_cap_bounds_what_a_page_reads() {
    let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let rows = Arc::new(AtomicUsize::new(0));
    let writer = HistoryRepo::new(store.clone(), store.clone());
    writer
        .record(
            (1..=500)
                .map(|n| act(n, n as i64 * 60, "Update", 1))
                .collect(),
        )
        .await
        .unwrap();
    let repo = HistoryRepo::new(store.clone(), Arc::new(Counting(store, rows.clone())))
        .with_page_size(10)
        .with_digest_cap(50);
    let page = repo
        .digest(&EntryFilter::default(), u32::MAX, None)
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(page.items[0].count <= 50);
    assert!(page.items[0].continues, "the run goes on");
    assert!(page.next.is_some());
    let read = rows.load(Ordering::SeqCst);
    assert!(read <= 50 + 10, "read {read} rows");
}

#[tokio::test]
async fn a_forgotten_or_older_create_does_not_bring_a_deleted_object_back() {
    // Forgotten: a later update stays blank.
    let repo = repo().await;
    repo.record(vec![act(1, 100, "Create", 1), act(2, 200, "Delete", 1)])
        .await
        .unwrap();
    repo.forget(&EntryFilter {
        object: Some(id(1001)),
        to: Some(at(150)),
        ..Default::default()
    })
    .await
    .unwrap();
    assert_eq!(
        repo.record(vec![act(1, 100, "Create", 1)]).await.unwrap(),
        [Outcome::Forgotten]
    );
    repo.record(vec![act(3, FUTURE, "Update", 1)])
        .await
        .unwrap();
    let page = repo.timeline(&EntryFilter::default(), None).await.unwrap();
    assert!(page.items.iter().all(blank), "{:?}", page.items);

    // Older than the delete, delivered late: the same.
    let repo = common::repo().await;
    repo.record(vec![act(2, 200, "Delete", 1), act(1, 100, "Create", 1)])
        .await
        .unwrap();
    repo.record(vec![act(3, 300, "Update", 1)]).await.unwrap();
    let page = repo.timeline(&EntryFilter::default(), None).await.unwrap();
    assert!(page.items.iter().filter(|e| e.kind == "Update").all(blank));

    // A newer create does bring it back.
    let repo = common::repo().await;
    repo.record(vec![
        act(1, 100, "Create", 1),
        act(2, 200, "Delete", 1),
        act(3, 300, "Create", 1),
        act(4, 400, "Update", 1),
    ])
    .await
    .unwrap();
    let page = repo.timeline(&EntryFilter::default(), None).await.unwrap();
    assert_eq!(page.items[0].name, "Meeting 1");
}
