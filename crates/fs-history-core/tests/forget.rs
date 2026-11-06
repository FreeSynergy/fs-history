//! Forget: removed stays removed, also on redelivery and for later activities in the range.

mod common;

use common::{act, at, id, repo, setup};
use fs_history_core::{EntryFilter, Error, FeedEvent, History};
use fs_store::Query;
use fs_store::traits::ObjectQuery;

fn program(p: &str) -> EntryFilter {
    EntryFilter {
        program: p.into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn removed_entries_stay_out_after_redelivery_of_the_same_ids() {
    let repo = repo().await;
    let items = vec![act(1, 100, "Create", 1), act(2, 200, "Update", 2)];
    repo.record(items.clone()).await.unwrap();
    assert_eq!(
        repo.forget(&EntryFilter {
            object: Some(id(1001)),
            ..Default::default()
        })
        .await
        .unwrap(),
        1
    );

    repo.record(items).await.unwrap();
    assert!(
        repo.get(id(1)).await.unwrap().is_none(),
        "forgotten id stays out"
    );
    assert!(repo.get(id(2)).await.unwrap().is_some());
}

#[tokio::test]
async fn a_later_activity_inside_the_forgotten_filter_stays_out() {
    let repo = repo().await;
    repo.record(vec![act(1, 100, "Create", 1)]).await.unwrap();
    let range = EntryFilter {
        from: Some(at(0)),
        to: Some(at(1000)),
        ..Default::default()
    };
    repo.forget(&range).await.unwrap();

    let late_inside = act(9, 500, "Update", 5);
    let outside = act(10, 5000, "Update", 5);
    let outcomes = repo.record(vec![late_inside, outside]).await.unwrap();
    assert_eq!(outcomes[0], fs_history_core::Outcome::Forgotten);
    assert_eq!(outcomes[1], fs_history_core::Outcome::Stored);
    assert!(repo.get(id(9)).await.unwrap().is_none());
    assert!(repo.get(id(10)).await.unwrap().is_some());
}

#[tokio::test]
async fn an_empty_filter_and_an_all_zero_filter_are_refused() {
    let (repo, store) = setup().await;
    repo.record(vec![act(1, 100, "Create", 1)]).await.unwrap();
    let before = store.count(&Query::new()).await.unwrap();

    let empty = repo.forget(&EntryFilter::default()).await;
    assert!(matches!(empty, Err(Error::Refused(_))));
    let zero = EntryFilter {
        from: Some(at(0)),
        to: Some(at(0)),
        ..Default::default()
    };
    assert!(matches!(repo.forget(&zero).await, Err(Error::Refused(_))));
    let via_trait = History::forget(&repo, &zero).await;
    assert!(matches!(via_trait, Err(Error::Refused(_))));
    assert_eq!(
        store.count(&Query::new()).await.unwrap(),
        before,
        "nothing was written"
    );
    assert!(repo.get(id(1)).await.unwrap().is_some());
}

#[tokio::test]
async fn the_feed_reports_a_forget_to_subscribers_it_concerns() {
    let repo = repo().await;
    repo.record(vec![act(1, 100, "Create", 1)]).await.unwrap();
    let mut mine = repo.subscribe(&program("calendar")).await.unwrap();
    let mut other = repo.subscribe(&program("wiki")).await.unwrap();

    repo.forget(&program("calendar")).await.unwrap();
    assert_eq!(mine.recv().await, Some(FeedEvent::Forgotten));
    assert_eq!(
        other.try_recv(),
        None,
        "a filter that matches nothing hears nothing"
    );
}
