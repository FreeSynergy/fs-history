//! Forgetting covers the past only; it is decided before anything is cleared; texts are sealed.

mod common;

use common::{act, at, id, repo, setup};
use fs_history_core::{EntryFilter, Outcome};
use fs_objects::Query;
use fs_objects::traits::UnloggedObjects;

/// Year 2096: after any forget made by a test run.
const FUTURE: i64 = 4_000_000_000;

fn only(f: impl FnOnce(&mut EntryFilter)) -> EntryFilter {
    let mut filter = EntryFilter::default();
    f(&mut filter);
    filter
}

#[tokio::test]
async fn later_activities_are_recorded_again() {
    for filter in [
        only(|f| f.from = Some(at(50))),
        only(|f| f.program = "calendar".into()),
        only(|f| f.context = "work".into()),
    ] {
        let repo = repo().await;
        repo.record(vec![act(1, 100, "Create", 1)]).await.unwrap();
        assert_eq!(repo.forget(&filter).await.unwrap(), 1);
        // The same object, after the forget: recorded.
        let out = repo
            .record(vec![act(2, FUTURE, "Update", 1)])
            .await
            .unwrap();
        assert_eq!(out, [Outcome::Stored], "{filter:?}");
        assert!(repo.get(id(2)).await.unwrap().is_some());
        // The past stays out, also the same object's older activity.
        let old = repo.record(vec![act(3, 90, "Update", 1)]).await.unwrap();
        assert_eq!(old, [Outcome::Forgotten], "{filter:?}");
    }
}

#[tokio::test]
async fn a_delete_in_a_forgotten_context_is_still_forgotten() {
    let repo = repo().await;
    repo.record(vec![act(1, 100, "Create", 1)]).await.unwrap();
    repo.forget(&only(|f| f.context = "work".into()))
        .await
        .unwrap();
    let out = repo.record(vec![act(2, 200, "Delete", 1)]).await.unwrap();
    assert_eq!(out, [Outcome::Forgotten]);
    assert!(repo.get(id(2)).await.unwrap().is_none());
}

#[tokio::test]
async fn a_bare_log_only_verb_of_a_removed_object_stays_out() {
    for filter in [
        only(|f| f.context = "work".into()),
        only(|f| f.object_type = "Event".into()),
        only(|f| f.person = "https://example.org/people/ada".into()),
    ] {
        let repo = repo().await;
        repo.record(vec![act(1, 100, "Create", 1)]).await.unwrap();
        repo.forget(&filter).await.unwrap();
        let mut invite = act(2, 50, "Invite", 1);
        (
            invite.object_type,
            invite.name,
            invite.attributed_to,
            invite.context,
        ) = Default::default();
        let out = repo.record_filtered(vec![invite]).await.unwrap();
        assert_eq!(out, [Outcome::Forgotten], "{filter:?}");
    }
}

#[tokio::test]
async fn filter_texts_are_not_readable_in_the_tombstone() {
    let (repo, store) = setup().await;
    repo.record(vec![act(1, 100, "Create", 1)]).await.unwrap();
    let filter = EntryFilter {
        context: "work".into(),
        program: "calendar".into(),
        person: "https://example.org/people/ada".into(),
        object_type: "Event".into(),
        ..Default::default()
    };
    repo.forget(&filter).await.unwrap();
    let tombstones = Query::new().of_type(fs_history_core::TOMBSTONE_TYPE);
    let raw = serde_json::to_string(
        &store
            .query_unlogged(&tombstones, &[fs_history_core::TOMBSTONE_TYPE])
            .await
            .unwrap(),
    )
    .unwrap();
    for text in ["work", "calendar", "ada", "Event", "event"] {
        assert!(!raw.contains(text), "{text} is readable in {raw}");
    }
    // Sealed, but it still works on the hash.
    let out = repo.record(vec![act(2, 90, "Update", 7)]).await.unwrap();
    assert_eq!(out, [Outcome::Forgotten]);
    let other = {
        let mut a = act(3, 90, "Update", 8);
        a.context = "home".into();
        a
    };
    assert_eq!(repo.record(vec![other]).await.unwrap(), [Outcome::Stored]);
}
