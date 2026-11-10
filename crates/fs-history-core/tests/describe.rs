//! The description against the table, the table against `requirements.md` and the contract.

use fs_history_core::commands::{self, DIGEST, ENTRY, table, unsupported_commands};
use fs_history_core::{DigestEntry, Entry, History, HistoryRepo, registry};
use fs_objects::TursoStore;
use fs_types::DescribesType;
use std::sync::Arc;

/// The snapshot of the requirements file, or the live one when `FS_HISTORY_REQUIREMENTS` names it.
fn requirements() -> String {
    match std::env::var("FS_HISTORY_REQUIREMENTS") {
        Ok(path) => std::fs::read_to_string(path).unwrap(),
        Err(_) => include_str!("data/requirements.md").to_owned(),
    }
}

/// First cell of every data row of every table.
fn requirement_rows(md: &str) -> Vec<String> {
    md.lines()
        .filter(|l| l.starts_with('|'))
        .filter_map(|l| l.split('|').nth(1).map(|c| c.trim().to_owned()))
        .filter(|c| !c.is_empty() && c != "Capability" && !c.starts_with("---"))
        .collect()
}

/// Every `contact.xxx` named in backticks.
fn named_commands(md: &str) -> Vec<String> {
    md.split('`')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, s)| s.to_owned())
        .filter(|s| s.starts_with("contact."))
        .collect()
}

#[test]
fn every_requirement_row_has_a_command_in_the_table() {
    let md = requirements();
    let rows = requirement_rows(&md);
    assert!(
        rows.len() >= 12,
        "the parser found only {} rows",
        rows.len()
    );
    let table = table();
    for row in &rows {
        assert!(
            table.iter().any(|r| r.requirement == row),
            "requirements.md row has no command in commands.rs: {row}"
        );
    }
}

#[test]
fn every_named_requirement_command_is_in_the_table() {
    let md = requirements();
    let named = named_commands(&md);
    assert!(named.len() >= 7);
    let table = table();
    for n in &named {
        assert!(
            table.iter().any(|r| r.requirement_name == n),
            "requirements.md names {n}, the table has no row for it"
        );
    }
}

#[test]
fn the_table_has_no_row_the_requirements_do_not_know() {
    let rows = requirement_rows(&requirements());
    for r in table().iter().filter(|r| !r.requirement.is_empty()) {
        assert!(
            rows.iter().any(|x| x == r.requirement),
            "stale row {}",
            r.name
        );
        assert!(
            !r.supported,
            "{} is asked for and cannot be supported without the core having it",
            r.name
        );
        assert!(!r.reason.is_empty());
    }
}

#[test]
fn the_description_holds_every_command_of_the_table() {
    for (name, d) in [
        (ENTRY, Entry::describe()),
        (DIGEST, DigestEntry::describe()),
    ] {
        let want: Vec<&str> = table()
            .iter()
            .filter(|r| r.data_type == name)
            .map(|r| r.name)
            .collect();
        let have: Vec<&str> = d.commands.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(have, want, "{name}");
        d.validate().unwrap();
    }
}

#[test]
fn the_description_carries_the_flag_of_the_table() {
    for (name, d) in [
        (ENTRY, Entry::describe()),
        (DIGEST, DigestEntry::describe()),
    ] {
        for r in table().iter().filter(|r| r.data_type == name) {
            assert_eq!(
                d.find_command(r.name).unwrap().supported,
                r.supported,
                "{}",
                r.name
            );
        }
    }
}

#[test]
fn the_unsupported_commands_are_listed_and_the_supported_are_not() {
    let u = unsupported_commands(ENTRY);
    assert!(u.contains(&"subscribe") && u.contains(&"contactMerge") && u.contains(&"onThisDay"));
    assert!(!u.contains(&"find") && !u.contains(&"forget"));
    assert_eq!(unsupported_commands(DIGEST), vec!["weeklySummary"]);
}

#[test]
fn the_registry_is_sound_and_its_references_resolve() {
    let r = registry().unwrap();
    assert_eq!(r.len(), 2);
    assert!(r.unresolved_targets().is_empty());
}

/// Tests against the trait: a supported command is a capability the provider reports; an unsupported one with a
/// capability would be a lie.
#[tokio::test]
async fn supported_commands_are_capabilities_of_the_provider() {
    let store = TursoStore::open_in_memory().await.unwrap();
    let provider: Arc<dyn History> = Arc::new(HistoryRepo::new(Arc::new(store)));
    let caps = provider.capabilities();
    for r in table() {
        if r.supported {
            assert!(
                caps.is_supported(r.capability),
                "{}: {}",
                r.name,
                r.capability
            );
        } else if !r.capability.is_empty() {
            // `subscribe` is a stream: the core offers it, the command form cannot (see the reason).
            assert!(r.reason.contains("stream"), "{}", r.name);
        }
    }
    assert!(commands::find(ENTRY, "find").is_some());
}

#[test]
fn field_names_follow_the_contract() {
    let e = Entry::describe();
    for n in [
        "id",
        "at",
        "program",
        "kind",
        "object",
        "objectType",
        "name",
        "attributedTo",
        "context",
        "actor",
    ] {
        assert!(e.find_field(n).is_some(), "{n}");
    }
}
