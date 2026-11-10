//! The `Backend` of the generated frontend over the history core.
//!
//! Pattern: Adapter + Repository — [`HistoryBackend`] implements the frontend's `Backend` for one data type (`Entry` or
//! `Digest`) over the trait `History`; it knows the core only through that trait. The commands come from the one table in
//! `fs-history-core` (`commands.rs`): a command the table marks unsupported is refused with a typed
//! [`RunError::Unsupported`] (`run_checked`); `Backend::run` can only answer `NotFound` for it, because `BackendError`
//! has no variant for it (reported to thread A).
//!
//! The core is async, the frontend's `Backend` is called synchronously from the window thread: a runtime handle bridges
//! them (`block_on`), as the other programs' adapters do.

use std::collections::BTreeMap;
use std::sync::Arc;

use fs_admin_frontend::{Backend, BackendError, Outcome, Record, Values};
use fs_history_core::commands::{self, DIGEST, ENTRY};
use fs_history_core::{EntryFilter, History};
use fs_interfaces::Unsupported;
use fs_types::{Edit, RecordIssue, RecordProblem};
use tokio::runtime::Handle;

mod convert;
use convert::{
    digest_matches, digest_record, entry_matches, entry_record, filter_of, first, one_id,
};

/// Most records one `list` reads (pages of the core are followed up to this many), so a window never holds the whole
/// history. The list says nothing when it is cut; the filter is how a person narrows it.
pub const MAX_LIST: usize = 2000;

/// The window, when no minutes are given: entries of one object and program within this many minutes are one line.
const DEFAULT_WINDOW: u32 = 60;

/// Why a command did not run, in more detail than `BackendError` can say.
#[derive(Debug)]
pub enum RunError {
    /// The command is in the description but the core cannot do it (the flag in the table is false).
    Unsupported(Unsupported),
    /// Any other answer of `Backend`.
    Backend(BackendError),
}

/// The records of one data type of history and its commands.
pub struct HistoryBackend {
    history: Arc<dyn History>,
    handle: Handle,
    data_type: &'static str,
}

impl HistoryBackend {
    /// The entries of the history.
    pub fn entries(history: Arc<dyn History>, handle: Handle) -> Self {
        Self {
            history,
            handle,
            data_type: ENTRY,
        }
    }

    /// The digest lines of the history.
    pub fn digests(history: Arc<dyn History>, handle: Handle) -> Self {
        Self {
            history,
            handle,
            data_type: DIGEST,
        }
    }

    /// As `Backend::run`, with the refusal of a missing command typed.
    ///
    /// # Errors
    /// [`RunError::Unsupported`] for a command whose flag in the table is false; otherwise as `Backend::run`.
    pub fn run_checked(
        &self,
        name: &str,
        record: Option<&str>,
        input: &Values,
    ) -> Result<Outcome, RunError> {
        let Some(row) = commands::find(self.data_type, name) else {
            return Err(RunError::Backend(BackendError::NotFound));
        };
        if !row.supported {
            return Err(RunError::Unsupported(Unsupported::new(
                if row.capability.is_empty() {
                    name
                } else {
                    row.capability
                },
                row.reason,
            )));
        }
        self.carry_out(name, record, input)
            .map_err(RunError::Backend)
    }

    fn carry_out(
        &self,
        name: &str,
        record: Option<&str>,
        input: &Values,
    ) -> Result<Outcome, BackendError> {
        match (self.data_type, name) {
            (_, "find") => {
                let records = self.list(input)?;
                Ok(Outcome {
                    id: None,
                    values: BTreeMap::from([(
                        "items".to_owned(),
                        records.into_iter().map(|r| r.id).collect(),
                    )]),
                })
            }
            (ENTRY, "get") => {
                let id = one_id(record, input)?;
                let found = self.block(self.history.get(id))?;
                let entry = found.ok_or(BackendError::NotFound)?;
                Ok(Outcome {
                    id: Some(entry.id.to_string()),
                    values: BTreeMap::from([("found".to_owned(), vec![entry.id.to_string()])]),
                })
            }
            (ENTRY, "forget") => {
                let id = one_id(record, input)?;
                let entry = self
                    .block(self.history.get(id))?
                    .ok_or(BackendError::NotFound)?;
                // The core removes by filter: the object of the entry in its program.
                let filter = EntryFilter {
                    program: entry.program.clone(),
                    object: Some(entry.object),
                    ..EntryFilter::default()
                };
                self.forget(&filter)
            }
            (ENTRY, "forgetMatching") => {
                let filter = filter_of(input)?;
                if filter.clone().normalized().is_empty() {
                    // The core refuses an empty filter; the form marks the first member.
                    return Err(BackendError::Rejected(vec![RecordIssue {
                        field: "program".into(),
                        index: 0,
                        problem: RecordProblem::Missing,
                    }]));
                }
                self.forget(&filter)
            }
            _ => Err(BackendError::NotFound),
        }
    }

    fn forget(&self, filter: &EntryFilter) -> Result<Outcome, BackendError> {
        let removed = self.block(self.history.forget(filter))?;
        Ok(Outcome {
            id: None,
            values: BTreeMap::from([("removed".to_owned(), vec![removed.to_string()])]),
        })
    }

    fn block<T>(
        &self,
        work: impl std::future::Future<Output = fs_history_core::Result<T>>,
    ) -> Result<T, BackendError> {
        self.handle.block_on(work).map_err(|e| match e {
            fs_history_core::Error::Refused(why) => {
                BackendError::Unavailable(format!("refused: {why}"))
            }
            other => BackendError::Unavailable(other.to_string()),
        })
    }
}

impl Backend for HistoryBackend {
    fn list(&self, query: &Values) -> Result<Vec<Record>, BackendError> {
        let filter = filter_of(query)?;
        let term = first(query, "term").to_lowercase();
        if self.data_type == DIGEST {
            let minutes = match first(query, "windowMinutes") {
                "" => DEFAULT_WINDOW,
                m => m.trim().parse().map_err(|_| bad("windowMinutes"))?,
            };
            let mut out = Vec::new();
            let mut cursor = None;
            while out.len() < MAX_LIST {
                let page = self.block(self.history.digest(&filter, minutes, cursor.as_ref()))?;
                out.extend(
                    page.items
                        .iter()
                        .filter(|d| digest_matches(d, &term))
                        .map(digest_record),
                );
                cursor = page.next;
                if cursor.is_none() {
                    break;
                }
            }
            return Ok(out);
        }
        let mut out = Vec::new();
        let mut cursor = None;
        while out.len() < MAX_LIST {
            let page = self.block(self.history.timeline(&filter, cursor.as_ref()))?;
            out.extend(
                page.items
                    .iter()
                    .filter(|e| entry_matches(e, &term))
                    .map(entry_record),
            );
            cursor = page.next;
            if cursor.is_none() {
                break;
            }
        }
        Ok(out)
    }

    fn supported(&self, command: &str) -> bool {
        commands::find(self.data_type, command).is_some_and(|r| r.supported)
    }

    fn change(&self, _record: &str, _edit: &Edit) -> Result<Outcome, BackendError> {
        // An entry is a derived summary: the table lists `change` as unsupported (`supported` says so too).
        Err(BackendError::NotFound)
    }

    fn run(
        &self,
        name: &str,
        record: Option<&str>,
        input: &Values,
    ) -> Result<Outcome, BackendError> {
        match self.run_checked(name, record, input) {
            Ok(o) => Ok(o),
            Err(RunError::Backend(e)) => Err(e),
            // `BackendError` has no variant for "not possible": the closest is "no such command".
            Err(RunError::Unsupported(_)) => Err(BackendError::NotFound),
        }
    }
}
