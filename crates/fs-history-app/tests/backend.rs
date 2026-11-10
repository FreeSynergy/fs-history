//! The backend over the real history and a real store, through the trait the frontend uses.

use std::sync::Arc;

use fs_admin_frontend::{Backend, BackendError, Values};
use fs_history_app::gui::{app_widget, chart_surfaces};
use fs_history_app::{HistoryBackend, RunError};
use fs_history_core::commands::{DIGEST, ENTRY, table};
use fs_history_core::{History, HistoryRepo, SourceActivity};
use fs_objects::{ObjectId, Timestamp, TursoStore};

fn id(n: u64) -> ObjectId {
    format!("00000000-0000-7000-8000-{n:012x}").parse().unwrap()
}

fn act(n: u64, s: i64, kind: &str, obj: u64, program: &str) -> SourceActivity {
    SourceActivity {
        id: id(n),
        at: Timestamp::from_timestamp(s, 0).unwrap(),
        program: program.into(),
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

fn values(pairs: &[(&str, &str)]) -> Values {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), vec![(*v).to_owned()]))
        .collect()
}

struct Fixture {
    rt: tokio::runtime::Runtime,
    history: Arc<dyn History>,
}

fn fixture() -> Fixture {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = rt.block_on(async {
        let repo = HistoryRepo::new(Arc::new(TursoStore::open_in_memory().await.unwrap()));
        repo.record(vec![
            act(1, 86_400 * 3, "Create", 1, "calendar"),
            act(2, 86_400 * 3 + 60, "Update", 1, "calendar"),
            act(3, 86_400 * 4, "Create", 2, "tasks"),
        ])
        .await
        .unwrap();
        repo
    });
    Fixture {
        rt,
        history: Arc::new(repo),
    }
}

#[test]
fn list_reads_entries_newest_first_and_filters_by_term_and_member() {
    let f = fixture();
    let b = HistoryBackend::entries(f.history.clone(), f.rt.handle().clone());
    let all = b.list(&Values::new()).unwrap();
    let names: Vec<_> = all.iter().map(|r| r.values["name"][0].clone()).collect();
    assert_eq!(names, ["Meeting 2", "Meeting 1", "Meeting 1"]);
    assert_eq!(all[0].values["at"][0], "1970-01-05T00:00:00Z");
    assert_eq!(b.list(&values(&[("term", "TASK")])).unwrap().len(), 1);
    assert_eq!(
        b.list(&values(&[("program", "calendar")])).unwrap().len(),
        2
    );
    assert_eq!(
        b.list(&values(&[("from", "1970-01-05T00:00:00Z")]))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn digest_list_groups_close_changes() {
    let f = fixture();
    let b = HistoryBackend::digests(f.history.clone(), f.rt.handle().clone());
    let lines = b.list(&values(&[("windowMinutes", "10")])).unwrap();
    assert_eq!(lines.len(), 2);
    let counts: Vec<_> = lines
        .iter()
        .map(|r| r.values["count"][0].as_str())
        .collect();
    assert_eq!(counts, ["1", "2"]);
    assert!(b.list(&values(&[("windowMinutes", "soon")])).is_err());
}

#[test]
fn get_and_forget_work_on_the_record_the_frontend_names() {
    let f = fixture();
    let b = HistoryBackend::entries(f.history.clone(), f.rt.handle().clone());
    let one = b.list(&Values::new()).unwrap().remove(1);
    let got = b.run("get", Some(&one.id), &Values::new()).unwrap();
    assert_eq!(got.id.as_deref(), Some(one.id.as_str()));
    assert_eq!(
        b.run(
            "get",
            Some("00000000-0000-7000-8000-0000000000ff"),
            &Values::new()
        )
        .unwrap_err(),
        BackendError::NotFound
    );
    // forgets the object of the entry in its program: both entries of Meeting 1
    let out = b.run("forget", Some(&one.id), &Values::new()).unwrap();
    assert_eq!(out.values["removed"], ["2"]);
    assert_eq!(b.list(&Values::new()).unwrap().len(), 1);
}

#[test]
fn forget_matching_refuses_an_empty_filter_and_removes_by_filter() {
    let f = fixture();
    let b = HistoryBackend::entries(f.history.clone(), f.rt.handle().clone());
    let err = b
        .run("forgetMatching", None, &values(&[("confirm", "yes")]))
        .unwrap_err();
    assert!(matches!(err, BackendError::Rejected(ref issues) if !issues.is_empty()));
    assert_eq!(b.list(&Values::new()).unwrap().len(), 3);
    let out = b
        .run(
            "forgetMatching",
            None,
            &values(&[("program", "tasks"), ("confirm", "yes")]),
        )
        .unwrap();
    assert_eq!(out.values["removed"], ["1"]);
}

#[test]
fn every_unsupported_command_is_refused_typed_and_reported_not_supported() {
    let f = fixture();
    for (kind, name) in [(ENTRY, true), (DIGEST, false)] {
        let b = if name {
            HistoryBackend::entries(f.history.clone(), f.rt.handle().clone())
        } else {
            HistoryBackend::digests(f.history.clone(), f.rt.handle().clone())
        };
        let mut seen = 0;
        for row in table().iter().filter(|r| r.data_type == kind) {
            assert_eq!(b.supported(row.name), row.supported, "{}", row.name);
            if !row.supported {
                seen += 1;
                match b.run_checked(row.name, None, &Values::new()) {
                    Err(RunError::Unsupported(u)) => assert!(!u.reason().is_empty()),
                    other => panic!("{}: {other:?}", row.name),
                }
                assert_eq!(
                    b.run(row.name, None, &Values::new()).unwrap_err(),
                    BackendError::NotFound
                );
            }
        }
        assert!(seen >= 1, "{kind}");
    }
}

#[test]
fn the_window_has_a_screen_per_type_and_the_charts_read_the_history() {
    let f = fixture();
    let app = app_widget(&f.history, f.rt.handle()).unwrap();
    let ids: Vec<String> = app.with_shell(|s| s.screens().iter().map(|x| x.id.clone()).collect());
    assert_eq!(ids, ["Entry", "Digest"]);
    for (key, surface) in chart_surfaces(&f.history, f.rt.handle()) {
        let view = surface.view().unwrap_or_else(|e| panic!("{key}: {e}"));
        assert!(!view.text_alternative().table.rows.is_empty(), "{key}");
    }
}
