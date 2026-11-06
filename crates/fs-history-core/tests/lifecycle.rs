//! After a delete, and type names written in other ways.

mod common;

use common::{act, id, repo};
use fs_history_core::{Outcome, Reads};

#[tokio::test]
async fn a_change_after_a_delete_stays_blank_until_a_new_create() {
    let repo = repo().await;
    repo.record(vec![
        act(1, 100, "Create", 1),
        act(2, 200, "Delete", 1),
        act(3, 300, "Update", 1),
    ])
    .await
    .unwrap();
    let late = repo.get(id(3)).await.unwrap().unwrap();
    assert_eq!((late.name.as_str(), late.context.as_str()), ("", ""));
    assert_eq!(late.object_type, "Event");

    repo.record(vec![act(4, 400, "Create", 1), act(5, 500, "Update", 1)])
        .await
        .unwrap();
    assert_eq!(repo.get(id(4)).await.unwrap().unwrap().name, "Meeting 1");
    assert_eq!(repo.get(id(5)).await.unwrap().unwrap().name, "Meeting 1");
}

#[tokio::test]
async fn other_spellings_of_relationship_do_not_get_through() {
    let repo = repo().await;
    let spellings = [
        "relationship",
        "RELATIONSHIP",
        "as:Relationship",
        "https://www.w3.org/ns/activitystreams#Relationship",
        "surveyresponse",
    ];
    for (n, t) in spellings.iter().enumerate() {
        let mut a = act(n as u64 + 1, 100, "Create", 1);
        a.object_type = (*t).into();
        assert_eq!(
            repo.record(vec![a]).await.unwrap(),
            [Outcome::Refused],
            "{t}"
        );
        assert!(Reads::new([*t]).is_err(), "{t}");
    }
    let mut ok = act(20, 100, "Create", 2);
    ok.object_type = "as:Event".into();
    repo.record(vec![ok]).await.unwrap();
    assert_eq!(
        repo.get(id(20)).await.unwrap().unwrap().object_type,
        "Event"
    );
}

#[tokio::test]
async fn a_log_only_verb_of_unknown_type_needs_a_source_that_filters() {
    let repo = repo().await;
    let mut bare = act(1, 100, "Invite", 9);
    (
        bare.object_type,
        bare.name,
        bare.attributed_to,
        bare.context,
    ) = Default::default();
    assert_eq!(
        repo.record(vec![bare.clone()]).await.unwrap(),
        [Outcome::Refused]
    );
    assert_eq!(
        repo.record_filtered(vec![bare]).await.unwrap(),
        [Outcome::Stored]
    );
}
