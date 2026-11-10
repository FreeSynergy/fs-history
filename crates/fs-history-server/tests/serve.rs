//! The contract over a real Unix socket, in front of the real history over a real store.

use std::sync::Arc;

use fs_history_core::{HistoryRepo, SourceActivity};
use fs_history_server::HistoryServer;
use fs_interfaces::history_capnp::history;
use fs_interfaces::rpc::{Problem, bind_socket, connect, serve};
use fs_interfaces::types_capnp::type_catalog;
use fs_objects::{ObjectId, Timestamp, TursoStore};
use fs_types::{CommandKind, TypeRegistry};

fn id(n: u64) -> ObjectId {
    format!("00000000-0000-7000-8000-{n:012x}").parse().unwrap()
}

fn act(n: u64, s: i64, kind: &str, obj: u64) -> SourceActivity {
    SourceActivity {
        id: id(n),
        at: Timestamp::from_timestamp(s, 0).unwrap(),
        program: "calendar".into(),
        kind: kind.into(),
        object: id(1000 + obj),
        object_type: "Event".into(),
        name: format!("Meeting {obj}"),
        attributed_to: "https://example.org/people/ada".into(),
        context: "work".into(),
        actor: String::new(),
        at_is_received: true,
    }
}

async fn with_server<F, Fut>(test: F)
where
    F: FnOnce(history::Client) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
            let repo = HistoryRepo::new(store);
            repo.record(vec![
                act(1, 100, "Create", 1),
                act(2, 160, "Update", 1),
                act(3, 900, "Create", 2),
            ])
            .await
            .unwrap();
            let root = HistoryServer::new(Arc::new(repo)).into_client();
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("history.sock");
            let bound = bind_socket(&path).await.unwrap();
            let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
            let serving = tokio::task::spawn_local(serve(bound, root.client, async {
                let _ = stopped.await;
            }));
            let connection = connect::<history::Client>(&path).await.unwrap();
            test(connection.client.clone()).await;
            drop(connection);
            let _ = stop.send(());
            serving.await.unwrap().unwrap();
        })
        .await;
}

#[tokio::test(flavor = "current_thread")]
async fn types_serves_the_catalog_with_every_command() {
    with_server(|root| async move {
        let answer = root.types_request().send().promise.await.unwrap();
        let catalog: type_catalog::Client = answer.get().unwrap().get_catalog().unwrap();
        let described = catalog.describe_request().send().promise.await.unwrap();
        let back = TypeRegistry::read(described.get().unwrap().get_types().unwrap()).unwrap();
        assert_eq!(back.len(), 2);
        let entry = back.get("Entry").unwrap();
        assert_eq!(entry.program, "fs-history");
        for name in ["find", "get", "forget", "forgetMatching", "subscribe", "contactMerge", "onThisDay"] {
            assert!(entry.find_command(name).is_some(), "{name}");
        }
        // the flag travels on the wire: locked ones are listed, not hidden
        assert!(entry.find_command("find").unwrap().supported);
        assert!(!entry.find_command("subscribe").unwrap().supported);
        assert!(!entry.find_command("contactMerge").unwrap().supported);
        assert_eq!(entry.find_command("forget").unwrap().kind, CommandKind::Delete);
        assert!(back.get("Digest").unwrap().find_command("find").is_some());
        assert!(back.unresolved_targets().is_empty());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn timeline_get_digest_and_forget_answer() {
    with_server(|root| async move {
        let page = root.timeline_request().send().promise.await.unwrap();
        let items = page.get().unwrap().get_page().unwrap().get_items().unwrap();
        // newest first
        let names: Vec<_> = items.iter().map(|e| e.get_name().unwrap().to_string().unwrap()).collect();
        assert_eq!(names, ["Meeting 2", "Meeting 1", "Meeting 1"]);
        let first_id = items.get(0).get_id().unwrap();
        let mut get = root.get_request();
        get.get().set_id(first_id).unwrap();
        let got = get.send().promise.await.unwrap();
        assert!(got.get().unwrap().get_found());
        assert_eq!(got.get().unwrap().get_entry().unwrap().get_program().unwrap(), "calendar");

        let mut digest = root.digest_request();
        digest.get().set_window_minutes(10);
        let d = digest.send().promise.await.unwrap();
        let lines = d.get().unwrap().get_page().unwrap().get_items().unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines.iter().map(|l| l.get_count()).sum::<u32>(), 3);

        let mut forget = root.forget_request();
        forget.get().get_filter().unwrap().set_program("calendar");
        let f = forget.send().promise.await.unwrap();
        assert_eq!(f.get().unwrap().get_removed(), 3);
        let again = root.timeline_request().send().promise.await.unwrap();
        assert_eq!(again.get().unwrap().get_page().unwrap().get_items().unwrap().len(), 0);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_empty_forget_is_refused_and_subscribe_says_unsupported() {
    with_server(|root| async move {
        let err = root.forget_request().send().promise.await.err().unwrap();
        let problem = Problem::from_error(&err).unwrap();
        assert_eq!(problem.status, 422);
        let err = root
            .subscribe_request()
            .send()
            .promise
            .await
            .err()
            .expect("subscribe has no RPC listener yet");
        assert_eq!(Problem::from_error(&err).unwrap().status, 501);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_support_report_names_what_is_offered() {
    with_server(|root| async move {
        let r = root.support_report_request().send().promise.await.unwrap();
        let report = r.get().unwrap().get_report().unwrap();
        assert!(report.get_entries().unwrap().len() >= 4);
        let c = root.capabilities_request().send().promise.await.unwrap();
        let ids = c.get().unwrap().get_capabilities().unwrap().get_ids().unwrap();
        assert!(ids.iter().any(|i| i.unwrap().to_str().unwrap() == "history.read"));
    })
    .await;
}
