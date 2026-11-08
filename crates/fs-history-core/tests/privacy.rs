//! No payload, no content, no activity log: the index keeps the summary and nothing else.

mod common;

use common::{act, setup};
use fs_objects::ActivityFilter;
use fs_objects::Query;
use fs_objects::traits::{ActivityLog, UnloggedObjects};

#[tokio::test]
async fn raw_stored_json_holds_only_the_summary_and_the_log_stays_empty() {
    let (repo, store) = setup().await;
    let items = vec![act(1, 100, "Create", 1), act(2, 200, "Update", 1)];
    repo.record(items).await.unwrap();

    let stored = store
        .query_unlogged(&Query::new(), &[fs_history_core::ENTRY_TYPE])
        .await
        .unwrap();
    assert_eq!(stored.len(), 2);
    for o in &stored {
        let raw = serde_json::to_value(o).unwrap();
        let mut keys: Vec<&str> = raw["content"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "actor",
                "atIsReceived",
                "kind",
                "name",
                "object",
                "objectType",
                "program"
            ]
        );
        assert!(raw.get("original").is_none(), "no original is kept");
        assert_eq!(raw["envelope"]["type"], "Activity");
        // The envelope name is a label of the entry, not the object's name.
        assert!(
            !raw["envelope"]["name"]
                .as_str()
                .unwrap()
                .contains("Meeting")
        );
    }
    let log = store.activities(&ActivityFilter::default()).await.unwrap();
    assert!(log.is_empty(), "unlogged writes leave no activity: {log:?}");
}

#[tokio::test]
async fn forget_and_delete_leave_no_activity_either() {
    let (repo, store) = setup().await;
    repo.record(vec![act(1, 100, "Create", 1), act(2, 200, "Delete", 1)])
        .await
        .unwrap();
    let filter = fs_history_core::EntryFilter {
        program: "calendar".into(),
        ..Default::default()
    };
    repo.forget(&filter).await.unwrap();
    let log = store.activities(&ActivityFilter::default()).await.unwrap();
    assert!(log.is_empty());
}
