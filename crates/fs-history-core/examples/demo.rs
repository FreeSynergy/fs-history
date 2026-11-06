//! History in one go: record activities, read the timeline and a digest, then forget.
//!
//! Run with `cargo run -p fs-history-core --example demo`.

use std::sync::Arc;

use fs_history_core::{EntryFilter, Error, HistoryRepo, SourceActivity, Timestamp};
use fs_store::{ObjectId, TursoStore};

fn activity(kind: &str, object: ObjectId, secs: i64, name: &str) -> SourceActivity {
    SourceActivity {
        id: ObjectId::new(),
        at: Timestamp::from_timestamp(1_800_000_000 + secs, 0).unwrap(),
        program: "calendar".into(),
        kind: kind.into(),
        object,
        object_type: "Event".into(),
        name: name.into(),
        attributed_to: "https://example.org/people/ada".into(),
        context: "solar-roofs".into(),
        actor: String::new(),
        at_is_received: true,
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let store = Arc::new(TursoStore::open_in_memory().await?);
    let history = HistoryRepo::new(store.clone(), store);

    let meeting = ObjectId::new();
    history
        .record(vec![
            activity("Create", meeting, 0, "Kick-off"),
            activity("Update", meeting, 120, "Kick-off"),
            activity("Update", meeting, 4_000, "Kick-off (moved)"),
        ])
        .await?;

    let all = EntryFilter::default();
    for e in history.timeline(&all, None).await?.items {
        println!("{} {} {:?}", e.at, e.kind, e.name);
    }
    for line in history.digest(&all, 10, None).await?.items {
        println!("digest: {} change(s) {:?}", line.count, line.kinds);
    }

    let project = EntryFilter {
        context: "solar-roofs".into(),
        ..EntryFilter::default()
    };
    println!("forgotten: {}", history.forget(&project).await?);
    println!("left: {}", history.timeline(&all, None).await?.items.len());
    Ok(())
}
