//! The digest: what a log has too much of, as one line.
//!
//! Pattern: pure functions — no store, no clock. Entries newest first are grouped when object and
//! program are equal and the gap to the neighbouring entry of the same object and program is at
//! most the window (a chain). [`cut`] says where a page may end so that no run crosses the cut;
//! a run longer than the cap is cut and flagged `continues`.

use std::collections::{HashMap, HashSet};

use chrono::Duration;
use fs_objects::ObjectId;

use crate::model::{DigestEntry, Entry};

/// A page never holds more entries than this; a longer run is cut and flagged.
pub const MAX_PAGE_ENTRIES: usize = 10_000;

/// One object in one program: the unit a digest line is about.
pub type Pair = (ObjectId, String);

fn window(minutes: u32) -> Duration {
    Duration::minutes(i64::from(minutes))
}

/// Group `entries` (newest first). The groups come out newest first by their latest entry; inside a
/// group `first` is the earliest entry and `kinds` runs from the earliest verb on.
pub fn digest(entries: &[Entry], window_minutes: u32) -> Vec<DigestEntry> {
    digest_page(entries, window_minutes, &[])
}

/// Like [`digest`]; the group at the end of each run in `continued` is flagged `continues`.
pub fn digest_page(entries: &[Entry], window_minutes: u32, continued: &[Pair]) -> Vec<DigestEntry> {
    let w = window(window_minutes);
    let mut groups: Vec<Vec<usize>> = Vec::new();
    // The open group of a pair: the one its oldest member seen so far belongs to.
    let mut open: HashMap<(ObjectId, &str), usize> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        let key = (e.object, e.program.as_str());
        let joined = open
            .get(&key)
            .copied()
            .filter(|&g| entries[*groups[g].last().unwrap_or(&i)].at - e.at <= w);
        if let Some(g) = joined {
            groups[g].push(i);
        } else {
            open.insert(key, groups.len());
            groups.push(vec![i]);
        }
    }
    let flagged: HashSet<usize> = continued
        .iter()
        .filter_map(|(o, p)| open.get(&(*o, p.as_str())).copied())
        .collect();
    groups
        .iter()
        .enumerate()
        .map(|(g, members)| line(entries, members, flagged.contains(&g)))
        .collect()
}

fn line(entries: &[Entry], members: &[usize], continues: bool) -> DigestEntry {
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
        continues,
    }
}

/// Where a page ends.
#[derive(Debug, PartialEq, Eq)]
pub struct Cut {
    /// The page is `entries[..end]`.
    pub end: usize,
    /// Runs that go on in the next page because of the cap.
    pub continued: Vec<Pair>,
    /// Entries are left for a next page (some may not have been read).
    pub more: bool,
}

/// Where a page of at least `target` entries may end so that no run of one object and program
/// crosses: entries after the end that lie within the window of a run on the page are taken in.
/// Nothing is read beyond the window after the page. `None`: not decidable yet, read more (unless
/// `exhausted`). A page stops at `max` entries; runs that would go on are listed in `continued`.
pub fn cut(
    entries: &[Entry],
    target: usize,
    window_minutes: u32,
    exhausted: bool,
    max: usize,
) -> Option<Cut> {
    let w = window(window_minutes);
    let want = target.clamp(1, max.max(1));
    if entries.len() < want && !exhausted {
        return None;
    }
    let mut end = want.min(entries.len());
    let mut last: HashMap<(ObjectId, &str), _> = HashMap::new();
    for e in &entries[..end] {
        last.insert((e.object, e.program.as_str()), e.at);
    }
    let (mut j, mut forced, mut continued) = (end, false, Vec::<Pair>::new());
    loop {
        let Some(e) = entries.get(j) else {
            if exhausted || end == 0 {
                let more = end < entries.len();
                return Some(Cut {
                    end,
                    continued,
                    more,
                });
            }
            if entries.len() < max {
                return None;
            }
            // Read as much as the cap allows. A run on the page that the unread rows could still
            // continue is flagged; this can flag a run that in fact ends here (a needless flag).
            let read_to = entries[entries.len() - 1].at;
            for (&(object, program), &l) in &last {
                let pair = (object, program.to_string());
                if l - read_to <= w && !continued.contains(&pair) {
                    continued.push(pair);
                }
            }
            return Some(Cut {
                end,
                continued,
                more: true,
            });
        };
        if end == 0 || e.at < entries[end - 1].at - w {
            return Some(Cut {
                end,
                continued,
                more: true,
            });
        }
        let key = (e.object, e.program.as_str());
        if last.get(&key).is_some_and(|&l| l - e.at <= w) {
            if forced || j + 1 > max {
                forced = true;
                let pair = (e.object, e.program.clone());
                if !continued.contains(&pair) {
                    continued.push(pair);
                }
            } else {
                for x in &entries[end..=j] {
                    last.insert((x.object, x.program.as_str()), x.at);
                }
                end = j + 1;
                j = end;
                continue;
            }
        }
        j += 1;
    }
}
