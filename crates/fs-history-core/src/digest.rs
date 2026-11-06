//! The digest: what a log has too much of, as one line.
//!
//! Pattern: pure functions — no store, no clock. Entries newest first are grouped when object and
//! program are equal and the gap to the neighbouring entry of the same object and program is at
//! most the window (a chain, so a long run of close edits is one group). [`cut`] says where a page
//! may end without splitting a group.

use std::collections::HashMap;

use chrono::Duration;
use fs_store::ObjectId;

use crate::model::{DigestEntry, Entry};

fn window(minutes: u32) -> Duration {
    Duration::minutes(i64::from(minutes))
}

/// Group `entries` (newest first). The groups come out newest first by their latest entry; inside a
/// group `first` is the earliest entry and `kinds` runs from the earliest verb on.
pub fn digest(entries: &[Entry], window_minutes: u32) -> Vec<DigestEntry> {
    let w = window(window_minutes);
    // Members per group, newest first, as indexes into `entries`.
    let mut groups: Vec<Vec<usize>> = Vec::new();
    // The open group of an (object, program) pair: the one its latest-seen member belongs to.
    let mut open: HashMap<(ObjectId, &str), usize> = HashMap::default();
    for (i, e) in entries.iter().enumerate() {
        let key = (e.object, e.program.as_str());
        let joined = open.get(&key).copied().filter(|&g| {
            let oldest = &entries[*groups[g].last().unwrap_or(&i)];
            oldest.at - e.at <= w
        });
        if let Some(g) = joined {
            groups[g].push(i);
        } else {
            open.insert(key, groups.len());
            groups.push(vec![i]);
        }
    }
    groups.iter().map(|g| line(entries, g)).collect()
}

fn line(entries: &[Entry], members: &[usize]) -> DigestEntry {
    let mut kinds: Vec<String> = Vec::new();
    for &i in members.iter().rev() {
        if !kinds.contains(&entries[i].kind) {
            kinds.push(entries[i].kind.clone());
        }
    }
    DigestEntry {
        first: entries[members[members.len() - 1]].clone(),
        last_at: entries[members[0]].at,
        count: u32::try_from(members.len()).unwrap_or(u32::MAX),
        kinds,
    }
}

/// Where a page of at least `target` entries may end: the first index `k >= target` at which the gap
/// between `entries[k - 1]` and `entries[k]` is larger than the window, since then no group runs
/// across. `None`: not decidable yet, read more (unless `exhausted`, then the page takes all).
pub fn cut(
    entries: &[Entry],
    target: usize,
    window_minutes: u32,
    exhausted: bool,
) -> Option<usize> {
    let w = window(window_minutes);
    for k in target.max(1)..entries.len() {
        if entries[k - 1].at - entries[k].at > w {
            return Some(k);
        }
    }
    exhausted.then_some(entries.len())
}
