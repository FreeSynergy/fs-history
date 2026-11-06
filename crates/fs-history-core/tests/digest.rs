//! Digest: exact window, programs kept apart, pages that never split a group.

mod common;

use common::{act, numbers, repo};
use fs_history_core::{EntryFilter, History, digest};

fn minutes(m: i64) -> i64 {
    m * 60
}

#[test]
fn the_window_boundary_is_exact() {
    let at_edge = [
        act(2, minutes(10), "Update", 1).into_entry(),
        act(1, 0, "Create", 1).into_entry(),
    ];
    let one = digest::digest(&at_edge, 10);
    assert_eq!(one.len(), 1, "a gap of exactly the window groups");
    assert_eq!(
        (one[0].count, one[0].kinds.clone()),
        (2, vec!["Create".to_string(), "Update".to_string()])
    );
    assert_eq!(one[0].first.kind, "Create");
    assert_eq!(one[0].last_at, at_edge[0].at);

    let past = [
        act(2, minutes(10) + 1, "Update", 1).into_entry(),
        act(1, 0, "Create", 1).into_entry(),
    ];
    assert_eq!(digest::digest(&past, 10).len(), 2, "one second more splits");
}

#[test]
fn two_programs_on_one_object_are_not_merged_and_chains_extend() {
    let mut other = act(3, minutes(5), "Update", 1);
    other.program = "tasks".into();
    let list = [
        act(4, minutes(16), "Update", 1).into_entry(),
        other.into_entry(),
        act(2, minutes(8), "Update", 1).into_entry(),
        act(1, 0, "Create", 1).into_entry(),
    ];
    let lines = digest::digest(&list, 10);
    assert_eq!(lines.len(), 2);
    assert_eq!(
        (lines[0].first.program.as_str(), lines[0].count),
        ("calendar", 3),
        "8 -> 16 -> 0 chain"
    );
    assert_eq!(lines[0].kinds, ["Create", "Update"], "each verb once");
    assert_eq!(
        (lines[1].first.program.as_str(), lines[1].count),
        ("tasks", 1)
    );
}

#[tokio::test]
async fn a_page_never_splits_a_group() {
    let repo = repo().await.with_page_size(2);
    // Object 1: four edits a minute apart (one group). Object 2: one edit a day earlier.
    let mut items: Vec<_> = (1..=4)
        .map(|n| act(n, 10_000 + minutes(n as i64), "Update", 1))
        .collect();
    items.push(act(5, 100, "Create", 2));
    repo.record(items).await.unwrap();

    let first = repo.digest(&EntryFilter::default(), 5, None).await.unwrap();
    assert_eq!(first.items.len(), 1);
    assert_eq!(
        first.items[0].count, 4,
        "a page of 2 grows to take the whole group"
    );
    let second = repo
        .digest(&EntryFilter::default(), 5, first.next.as_ref())
        .await
        .unwrap();
    assert_eq!((second.items.len(), second.items[0].count), (1, 1));
    assert!(second.next.is_none());
    assert_eq!(
        numbers(
            &repo
                .timeline(&EntryFilter::default(), None)
                .await
                .unwrap()
                .items
        )
        .len(),
        2
    );
}

#[tokio::test]
async fn digest_is_offered_through_the_trait() {
    let repo = repo().await;
    repo.record(vec![act(1, 0, "Create", 1), act(2, 60, "Update", 1)])
        .await
        .unwrap();
    let page = History::digest(&repo, &EntryFilter::default(), 5, None)
        .await
        .unwrap();
    assert_eq!(page.items[0].count, 2);
}
