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

use chrono::{DateTime, SecondsFormat, Utc};
use fs_admin_frontend::{Backend, BackendError, Outcome, Record, Values};
use fs_history_core::commands::{self, DIGEST, ENTRY};
use fs_history_core::{DigestEntry, Entry, EntryFilter, History};
use fs_interfaces::Unsupported;
use fs_objects::ObjectId;
use fs_types::{Edit, RecordIssue, RecordProblem, TypeProblem};
use tokio::runtime::Handle;

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
                if row.capability.is_empty() { name } else { row.capability },
                row.reason,
            )));
        }
        self.carry_out(name, record, input).map_err(RunError::Backend)
    }

    fn carry_out(&self, name: &str, record: Option<&str>, input: &Values) -> Result<Outcome, BackendError> {
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
            fs_history_core::Error::Refused(why) => BackendError::Unavailable(format!("refused: {why}")),
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

fn first<'a>(v: &'a Values, key: &str) -> &'a str {
    v.get(key)
        .and_then(|l| l.iter().map(|s| s.trim()).find(|s| !s.is_empty()))
        .unwrap_or("")
}

fn bad(field: &str) -> BackendError {
    BackendError::Rejected(vec![RecordIssue {
        field: field.to_owned(),
        index: 0,
        problem: RecordProblem::Value(TypeProblem::BadDateTime),
    }])
}

fn time(v: &Values, key: &str) -> Result<Option<DateTime<Utc>>, BackendError> {
    match first(v, key) {
        "" => Ok(None),
        t => DateTime::parse_from_rfc3339(t)
            .map(|d| Some(d.with_timezone(&Utc)))
            .map_err(|_| bad(key)),
    }
}

/// The record a command works on: the id the frontend passes, else the `entry` input (a caller without a record).
fn one_id(record: Option<&str>, input: &Values) -> Result<ObjectId, BackendError> {
    record
        .unwrap_or_else(|| first(input, "entry"))
        .parse()
        .map_err(|_| BackendError::NotFound)
}

/// The filter members of a command input (`from`, `to`, `program`, `person`, `objectType`, `context`, `object`).
fn filter_of(v: &Values) -> Result<EntryFilter, BackendError> {
    let object = match first(v, "object") {
        "" => None,
        o => Some(o.parse().map_err(|_| BackendError::NotFound)?),
    };
    Ok(EntryFilter {
        from: time(v, "from")?,
        to: time(v, "to")?,
        program: first(v, "program").to_owned(),
        person: first(v, "person").to_owned(),
        object_type: first(v, "objectType").to_owned(),
        context: first(v, "context").to_owned(),
        object,
    })
}

fn entry_matches(e: &Entry, term: &str) -> bool {
    term.is_empty()
        || [&e.name, &e.program, &e.kind, &e.object_type, &e.context]
            .iter()
            .any(|f| f.to_lowercase().contains(term))
}

fn digest_matches(d: &DigestEntry, term: &str) -> bool {
    entry_matches(&d.first, term) || d.kinds.iter().any(|k| k.to_lowercase().contains(term))
}

fn stamp(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn put(values: &mut Values, key: &str, value: &str) {
    if !value.is_empty() {
        values.insert(key.to_owned(), vec![value.to_owned()]);
    }
}

fn entry_values(e: &Entry) -> Values {
    let mut v = Values::new();
    put(&mut v, "id", &e.id.to_string());
    put(&mut v, "at", &stamp(e.at));
    put(&mut v, "program", &e.program);
    put(&mut v, "kind", &e.kind);
    put(&mut v, "object", &e.object.to_string());
    put(&mut v, "objectType", &e.object_type);
    put(&mut v, "name", &e.name);
    put(&mut v, "attributedTo", &e.attributed_to);
    put(&mut v, "context", &e.context);
    put(&mut v, "actor", &e.actor);
    v
}

fn entry_record(e: &Entry) -> Record {
    Record {
        id: e.id.to_string(),
        values: entry_values(e),
    }
}

fn digest_record(d: &DigestEntry) -> Record {
    let mut v = Values::new();
    put(&mut v, "first", &d.first.id.to_string());
    put(&mut v, "lastAt", &stamp(d.last_at));
    put(&mut v, "count", &d.count.to_string());
    if !d.kinds.is_empty() {
        v.insert("kinds".to_owned(), d.kinds.clone());
    }
    Record {
        id: d.first.id.to_string(),
        values: v,
    }
}
