//! The manifest history registers with: default reads, no writes, never `Relationship`.

use fs_history_core::{DEFAULT_READS, Reads};
use fs_history_hub::{PROGRAM, manifest, manifest_with};

#[test]
fn default_reads_and_no_writes() {
    let m = manifest();
    assert_eq!(m.program, PROGRAM);
    assert_eq!(m.reads, DEFAULT_READS);
    assert!(m.writes.is_empty(), "history writes nothing to the hub");
    assert_eq!(m.content_types, [] as [std::string::String; 0]);
    assert!(m.capabilities.contains(&"history.forget".to_string()));
}

#[test]
fn relationship_and_survey_answers_are_not_in_the_manifest() {
    let m = manifest();
    for never in ["Relationship", "SurveyResponse"] {
        assert!(!m.reads.iter().any(|t| t == never), "{never}");
        assert!(!m.writes.iter().any(|t| t == never), "{never}");
    }
}

#[test]
fn a_chosen_list_is_taken_over_and_a_refused_one_cannot_be_built() {
    let reads = Reads::new(["Event"]).unwrap();
    assert_eq!(manifest_with(&reads).reads, ["Event"]);
    assert!(Reads::new(["Event", "Relationship"]).is_err());
}
