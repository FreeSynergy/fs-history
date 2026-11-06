//! `Relationship` and `SurveyResponse` are never read.

mod common;

use common::{act, id, repo};
use fs_history_core::{DEFAULT_READS, Error, Outcome, Reads};

#[test]
fn relationship_is_refused_in_reads_and_absent_from_the_default() {
    assert!(matches!(
        Reads::new(["Profile", "Relationship"]),
        Err(Error::Refused(_))
    ));
    assert!(matches!(
        Reads::new(["SurveyResponse"]),
        Err(Error::Refused(_))
    ));
    assert!(Reads::new(["Profile", "Event"]).is_ok());
    assert!(!DEFAULT_READS.contains(&"Relationship"));
    assert!(!DEFAULT_READS.contains(&"SurveyResponse"));
    let default = Reads::default();
    assert_eq!(default.types().len(), DEFAULT_READS.len());
    assert!(
        default
            .types()
            .iter()
            .all(|t| t != "Relationship" && t != "SurveyResponse")
    );
}

#[tokio::test]
async fn a_source_that_delivers_one_anyway_is_not_recorded() {
    let repo = repo().await;
    let mut rel = act(1, 100, "Create", 1);
    rel.object_type = "Relationship".into();
    let out = repo.record(vec![rel]).await.unwrap();
    assert_eq!(out, [Outcome::Refused]);
    assert!(repo.get(id(1)).await.unwrap().is_none());
}
