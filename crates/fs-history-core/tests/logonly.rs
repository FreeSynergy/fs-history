//! Log-only verbs take what their envelope lacks from the object's latest entry.

mod common;

use common::{act, id, repo};

fn bare_invite(n: u64, s: i64, obj: u64) -> fs_history_core::SourceActivity {
    let mut a = act(n, s, "Invite", obj);
    (a.object_type, a.name, a.attributed_to, a.context) = Default::default();
    a
}

#[tokio::test]
async fn filled_from_the_latest_entry() {
    let repo = repo().await;
    let mut renamed = act(2, 200, "Update", 1);
    renamed.name = "Renamed".into();
    repo.record(vec![
        act(1, 100, "Create", 1),
        renamed,
        bare_invite(3, 300, 1),
    ])
    .await
    .unwrap();
    let e = repo.get(id(3)).await.unwrap().unwrap();
    assert_eq!(e.kind, "Invite");
    assert_eq!(e.name, "Renamed", "the latest, not the first");
    assert_eq!(
        (e.object_type.as_str(), e.context.as_str()),
        ("Event", "work")
    );
    assert_eq!(e.attributed_to, "https://example.org/people/ada");
}

#[tokio::test]
async fn none_known_leaves_the_fields_empty() {
    let repo = repo().await;
    repo.record(vec![bare_invite(1, 100, 9)]).await.unwrap();
    let e = repo.get(id(1)).await.unwrap().unwrap();
    assert_eq!(e.kind, "Invite");
    assert!(
        [&e.object_type, &e.name, &e.attributed_to, &e.context]
            .iter()
            .all(|f| f.is_empty())
    );
}

#[tokio::test]
async fn a_state_changing_verb_is_not_filled() {
    let repo = repo().await;
    let mut moved_out = act(2, 200, "Update", 1);
    moved_out.context = String::new();
    repo.record(vec![act(1, 100, "Create", 1), moved_out])
        .await
        .unwrap();
    assert_eq!(repo.get(id(2)).await.unwrap().unwrap().context, "");
}
