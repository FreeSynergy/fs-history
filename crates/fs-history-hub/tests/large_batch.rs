//! A batch too large for one hub message is pulled in smaller pieces.

use std::sync::Arc;

use fs_history_core::{EntryFilter, HistoryRepo, Observer};
use fs_history_hub::{HubSource, manifest};
use fs_hub::{Config, Daemon};
use fs_hub_client::{HubStatus, Manifest, connect_at};
use fs_objects_hub::{Envelope, OpenOptions, Store, StoredObject, TursoStore as HubStore};

const NOTES: usize = 80;
const MIB: usize = 1 << 20;

#[tokio::test]
async fn eighty_one_mib_notes_do_not_stick_the_ingest() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("run/freesynergy/hub.sock");
    let _hub = Daemon::spawn(Config::new(socket.clone(), dir.path().join("hub-data"))).unwrap();

    let wiki = Manifest::new("wiki", "0.1.0").write(&["Note"]);
    let session = match connect_at(&socket, wiki).await.unwrap() {
        HubStatus::Connected(s) => s,
        HubStatus::Absent => panic!("hub should be there"),
    };
    let store = HubStore::open_with(dir.path().join("wiki.db"), OpenOptions::default())
        .await
        .unwrap();
    // Pushed in pieces: only the way back is meant to be too large.
    for round in 0..NOTES / 8 {
        for n in 0..8 {
            let name = format!("Note {}", round * 8 + n);
            let content = serde_json::json!({ "text": "x".repeat(MIB) });
            let o = StoredObject::new(Envelope::new("Note", name), "application/json", content);
            store.put(o, "wiki").await.unwrap();
        }
        session.sync_once(&store).await.unwrap();
    }

    let h_session = connect(&socket).await;
    let own = Arc::new(fs_objects::TursoStore::open_in_memory().await.unwrap());
    let repo = HistoryRepo::new(own.clone());

    // Asking for 100 at a time is too large for one hub message: since fs-hub 0.3 the hub cuts the
    // batch itself and says there is more, instead of refusing it.
    let greedy = HubSource::new(h_session).with_max_batch(100);
    let observer = Observer::new(repo.clone(), Arc::new(greedy));
    let cut = observer.run_once().await.unwrap();
    assert!(cut.more && cut.stored < NOTES, "{cut:?}");

    // The default stays below the limit, with the default observer batch of 100.
    let h_session = connect(&socket).await;
    let observer = Observer::new(repo.clone(), Arc::new(HubSource::new(h_session)));
    let report = observer.catch_up().await.unwrap();
    assert_eq!(report.stored + cut.stored, NOTES);
    let page = repo.timeline(&EntryFilter::default(), None).await.unwrap();
    assert_eq!(page.items.len(), NOTES);
}

async fn connect(socket: &std::path::Path) -> fs_hub_client::HubSession {
    match connect_at(socket, manifest()).await.unwrap() {
        HubStatus::Connected(s) => s,
        HubStatus::Absent => panic!("hub should be there"),
    }
}
