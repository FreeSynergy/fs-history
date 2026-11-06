//! Digest pages: bounded by the page size, keyset reads, the cap and `continues`.

mod common;

use common::{act, repo};
use fs_history_core::{EntryFilter, History};

#[tokio::test]
async fn distinct_objects_do_not_pull_the_whole_index_into_one_page() {
    let repo = repo().await.with_page_size(10);
    let items = (1..=1000)
        .map(|n| act(n, n as i64 * 60, "Update", n))
        .collect();
    repo.record(items).await.unwrap();
    let page = repo.digest(&EntryFilter::default(), 5, None).await.unwrap();
    assert!(page.items.len() <= 10 + 5, "{} lines", page.items.len());
    assert!(page.next.is_some());
}

#[tokio::test]
async fn pages_cover_every_entry_exactly_once() {
    let repo = repo().await.with_page_size(3);
    let mut items: Vec<_> = (1..=10)
        .map(|n| act(n, 10_000 + n as i64 * 30, "Update", 1))
        .collect();
    items.extend((11..=14).map(|n| act(n, n as i64 * 3600, "Update", n)));
    repo.record(items).await.unwrap();
    let (mut total, mut cursor) = (0, None);
    loop {
        let page = History::digest(&repo, &EntryFilter::default(), 5, cursor.as_ref())
            .await
            .unwrap();
        total += page.items.iter().map(|d| d.count).sum::<u32>();
        cursor = page.next;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(total, 14);
}

#[tokio::test]
async fn a_very_long_run_is_cut_and_flagged() {
    let repo = repo().await.with_page_size(3).with_digest_cap(5);
    let items = (1..=12)
        .map(|n| act(n, n as i64 * 60, "Update", 1))
        .collect();
    repo.record(items).await.unwrap();
    let first = repo.digest(&EntryFilter::default(), 5, None).await.unwrap();
    assert_eq!((first.items.len(), first.items[0].count), (1, 5));
    assert!(first.items[0].continues);
    let (mut total, mut page) = (0, first);
    loop {
        total += page.items.iter().map(|d| d.count).sum::<u32>();
        let Some(next) = page.next.clone() else { break };
        assert!(page.items.iter().all(|d| d.count <= 5));
        page = repo
            .digest(&EntryFilter::default(), 5, Some(&next))
            .await
            .unwrap();
    }
    assert_eq!(total, 12);
    assert!(
        !page.items.last().unwrap().continues,
        "the last piece ends the run"
    );
}
