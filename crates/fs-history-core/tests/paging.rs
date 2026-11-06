//! Paging and filters: stable under equal timestamps, half-open range, each member, `person`.

mod common;

use common::{act, at, id, numbers, repo};
use fs_history_core::{EntryFilter, History, HistoryRepo};

async fn ids(repo: &HistoryRepo, f: EntryFilter) -> Vec<u64> {
    numbers(&History::timeline(repo, &f, None).await.unwrap().items)
}

async fn all_pages(repo: &HistoryRepo, filter: &EntryFilter) -> Vec<Vec<u64>> {
    let (mut pages, mut cursor) = (vec![], None);
    loop {
        let page = repo.timeline(filter, cursor.as_ref()).await.unwrap();
        pages.push(numbers(&page.items));
        cursor = page.next;
        if cursor.is_none() {
            return pages;
        }
    }
}

#[tokio::test]
async fn paging_is_stable_under_equal_timestamps() {
    let repo = repo().await.with_page_size(2);
    // Five at the same second, one older, one newer; written in a shuffled order.
    let order = [3, 1, 5, 2, 4];
    let mut items: Vec<_> = order.iter().map(|&n| act(n, 500, "Update", 1)).collect();
    items.push(act(6, 100, "Create", 1));
    items.push(act(7, 900, "Update", 1));
    repo.record(items).await.unwrap();

    let pages = all_pages(&repo, &EntryFilter::default()).await;
    assert_eq!(pages, [vec![7, 5], vec![4, 3], vec![2, 1], vec![6]]);

    // An entry arriving between two calls changes neither the seen nor the unseen part.
    let first = repo.timeline(&EntryFilter::default(), None).await.unwrap();
    repo.record(vec![act(8, 950, "Update", 1)]).await.unwrap();
    let second = repo
        .timeline(&EntryFilter::default(), first.next.as_ref())
        .await
        .unwrap();
    assert_eq!(numbers(&second.items), [4, 3]);
}

#[tokio::test]
async fn the_range_is_half_open() {
    let repo = repo().await;
    repo.record(vec![
        act(1, 100, "Create", 1),
        act(2, 200, "Update", 1),
        act(3, 300, "Update", 1),
    ])
    .await
    .unwrap();
    let f = EntryFilter {
        from: Some(at(100)),
        to: Some(at(300)),
        ..Default::default()
    };
    let page = repo.timeline(&f, None).await.unwrap();
    assert_eq!(numbers(&page.items), [2, 1], "from is included, to is not");
    // An all-zero time is unset.
    let zero = EntryFilter {
        from: Some(at(0)),
        to: Some(at(0)),
        ..Default::default()
    };
    assert_eq!(repo.timeline(&zero, None).await.unwrap().items.len(), 3);
}

#[tokio::test]
async fn each_member_of_the_filter_narrows() {
    let repo = repo().await;
    let mut a = act(1, 100, "Create", 1);
    a.program = "tasks".into();
    a.object_type = "Task".into();
    a.context = "home".into();
    let b = act(2, 200, "Update", 2);
    repo.record(vec![a, b]).await.unwrap();
    let one = |f: EntryFilter| ids(&repo, f);
    assert_eq!(
        one(EntryFilter {
            program: "tasks".into(),
            ..Default::default()
        })
        .await,
        [1]
    );
    assert_eq!(
        one(EntryFilter {
            object_type: "Event".into(),
            ..Default::default()
        })
        .await,
        [2]
    );
    assert_eq!(
        one(EntryFilter {
            context: "home".into(),
            ..Default::default()
        })
        .await,
        [1]
    );
    assert_eq!(
        one(EntryFilter {
            object: Some(id(1002)),
            ..Default::default()
        })
        .await,
        [2]
    );
    let ada = "https://example.org/people/ada";
    assert_eq!(
        one(EntryFilter {
            person: ada.into(),
            ..Default::default()
        })
        .await,
        [2, 1]
    );
    let both = EntryFilter {
        program: "tasks".into(),
        object_type: "Event".into(),
        ..Default::default()
    };
    assert!(one(both).await.is_empty(), "members combine with AND");
}

#[tokio::test]
async fn person_matches_the_actor_or_the_owner_where_there_is_no_actor() {
    let repo = repo().await;
    let (ada, bo) = (
        "https://example.org/people/ada",
        "https://example.org/people/bo",
    );
    let owner_only = act(1, 100, "Create", 1); // attributed to ada, no actor
    let mut by_bo = act(2, 200, "Update", 1); // ada's object, bo acted
    by_bo.actor = bo.into();
    repo.record(vec![owner_only, by_bo]).await.unwrap();
    let who = |p: &str| EntryFilter {
        person: p.into(),
        ..Default::default()
    };
    assert_eq!(
        numbers(&repo.timeline(&who(ada), None).await.unwrap().items),
        [1]
    );
    assert_eq!(
        numbers(&repo.timeline(&who(bo), None).await.unwrap().items),
        [2]
    );
    for e in repo
        .timeline(&EntryFilter::default(), None)
        .await
        .unwrap()
        .items
    {
        assert_eq!(who(ada).matches(&e), e.actor.is_empty());
    }
}
