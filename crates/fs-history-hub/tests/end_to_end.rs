//! End to end with a real hub daemon: two programs write, history observes, the timeline shows it.

use std::sync::Arc;

use fs_history_core::{EntryFilter, HistoryRepo, Observer};
use fs_history_hub::{HubSource, manifest};
use fs_hub::{Config, Daemon};
use fs_hub_client::{HubSession, HubStatus, Manifest, connect_at};
use fs_objects::Query;
use fs_objects::traits::UnloggedObjects;
use fs_objects::{Envelope, OpenOptions, Store, StoredObject, TursoStore as HubStore};

const SECRET: &str = "ada-secret@example.org";

fn socket(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().join("run/freesynergy/hub.sock")
}

async fn program(dir: &tempfile::TempDir, m: Manifest) -> (HubSession, HubStore) {
    let opts = OpenOptions::default();
    let store = HubStore::open_with(dir.path().join(format!("{}.db", m.program)), opts)
        .await
        .unwrap();
    match connect_at(&socket(dir), m).await.unwrap() {
        HubStatus::Connected(s) => (s, store),
        HubStatus::Absent => panic!("hub should be there"),
    }
}

fn object(kind: &str, name: &str, content: serde_json::Value) -> StoredObject {
    let mut env = Envelope::new(kind, name);
    env.context = Some("solar-roofs".into());
    env.attributed_to = Some("https://example.org/people/ada".into());
    StoredObject::new(env, "application/json", content)
}

#[tokio::test]
async fn two_programs_write_and_the_timeline_shows_them_without_content() {
    let dir = tempfile::tempdir().unwrap();
    let _hub = Daemon::spawn(Config::new(socket(&dir), dir.path().join("hub-data"))).unwrap();

    let contacts = Manifest::new("contacts", "0.1.0")
        .read(&["Event"])
        .write(&["Profile"]);
    let calendar = Manifest::new("calendar", "0.1.0")
        .read(&["Profile"])
        .write(&["Event"]);
    let (c_session, c_store) = program(&dir, contacts).await;
    let (k_session, k_store) = program(&dir, calendar).await;

    let ada = object(
        "Profile",
        "Ada Lovelace",
        serde_json::json!({"email": SECRET}),
    );
    c_store.put(ada.clone(), "contacts").await.unwrap();
    c_session.sync_once(&c_store).await.unwrap();
    let meeting = object(
        "Event",
        "Kick-off",
        serde_json::json!({"notes": "the plan is secret"}),
    );
    k_store.put(meeting, "calendar").await.unwrap();
    k_session.sync_once(&k_store).await.unwrap();
    // An update and a delete go through the hub as well.
    let mut renamed = ada.clone();
    renamed.envelope.name = "Ada King".into();
    c_store.put(renamed, "contacts").await.unwrap();
    c_session.sync_once(&c_store).await.unwrap();

    // History: its own store, registered with its own manifest, observing through the hub.
    let (h_session, _own) = program(&dir, manifest()).await;
    let store = Arc::new(fs_objects::TursoStore::open_in_memory().await.unwrap());
    let repo = HistoryRepo::new(store.clone());
    let observer = Observer::new(repo.clone(), Arc::new(HubSource::new(h_session)));
    let report = observer.catch_up().await.unwrap();
    assert_eq!(report.stored, 3);

    let page = repo.timeline(&EntryFilter::default(), None).await.unwrap();
    let seen: Vec<(&str, &str, &str)> = page
        .items
        .iter()
        .map(|e| (e.program.as_str(), e.kind.as_str(), e.name.as_str()))
        .collect();
    assert_eq!(seen.len(), 3);
    for want in [
        ("contacts", "Create", "Ada Lovelace"),
        ("calendar", "Create", "Kick-off"),
        ("contacts", "Update", "Ada King"),
    ] {
        assert!(seen.contains(&want), "{want:?} in {seen:?}");
    }
    assert!(
        page.items
            .iter()
            .all(|e| e.actor.is_empty() && e.at_is_received)
    );
    let ada_only = EntryFilter {
        program: "contacts".into(),
        ..EntryFilter::default()
    };
    assert_eq!(repo.timeline(&ada_only, None).await.unwrap().items.len(), 2);

    // The hub keeps the cursor: a second run finds nothing new, and nothing is stored twice.
    let again = observer.catch_up().await.unwrap();
    assert_eq!(again.stored + again.duplicates, 0);

    // Privacy, from the raw stored JSON: no content of any object, and no activity log.
    let raw = serde_json::to_string(
        &store
            .query_unlogged(&Query::new(), &[fs_history_core::ENTRY_TYPE])
            .await
            .unwrap(),
    )
    .unwrap();
    for leaked in [SECRET, "the plan is secret", "email", "notes"] {
        assert!(!raw.contains(leaked), "{leaked} must not be stored");
    }
    let log = store
        .activities(&fs_objects::ActivityFilter::default())
        .await
        .unwrap();
    assert_eq!(log, [] as [fs_objects::Activity; 0]);
}
