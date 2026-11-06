//! A `Delete` blanks name and context of every earlier entry of the object; the type stays.

mod common;

use common::{act, id, repo};
use fs_history_core::{EntryFilter, FeedEvent};

#[tokio::test]
async fn delete_blanks_earlier_entries_and_a_later_log_only_verb_stays_blank() {
    let repo = repo().await;
    let mut feed = repo.watch(EntryFilter::default());
    repo.record(vec![
        act(1, 100, "Create", 1),
        act(2, 200, "Update", 1),
        act(3, 150, "Create", 2),
    ])
    .await
    .unwrap();
    let mut gone = act(4, 300, "Delete", 1);
    gone.name = "Meeting 1".into();
    repo.record(vec![gone]).await.unwrap();

    for n in [1, 2, 4] {
        let e = repo.get(id(n)).await.unwrap().unwrap();
        assert_eq!((e.name.as_str(), e.context.as_str()), ("", ""), "entry {n}");
        assert_eq!(e.object_type, "Event", "the type stays");
        assert_eq!(e.program, "calendar");
    }
    let other = repo.get(id(3)).await.unwrap().unwrap();
    assert_eq!(other.name, "Meeting 2", "other objects are untouched");

    // A log-only verb after the delete has nothing to fill from but blank.
    let mut invite = act(5, 400, "Invite", 1);
    (invite.name, invite.context, invite.object_type) = Default::default();
    repo.record(vec![invite]).await.unwrap();
    let e = repo.get(id(5)).await.unwrap().unwrap();
    assert_eq!((e.name.as_str(), e.context.as_str()), ("", ""));
    assert_eq!(e.object_type, "Event");

    let mut seen_forgotten = false;
    while let Some(ev) = feed.try_recv() {
        seen_forgotten |= ev == FeedEvent::Forgotten;
    }
    assert!(seen_forgotten, "the feed says entries were blanked");
}
