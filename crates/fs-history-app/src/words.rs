//! The words of the program (ADR 0057): all English text of this frontend in one place.
//!
//! Pattern: Strategy (`Say` of `fs-components-fields`) — the frontend asks for keys, this answers. Translation
//! (ADR 0053) will replace this file's content, not its users. The keys of the description (`entry.object-type`,
//! `entry.forget-matching.confirm`, ...) are words made from the last part of the key.

use fs_components_fields::Say;

/// The title of the window.
pub const TITLE: &str = "FreeSynergy History";

/// The words the frontend itself asks for (`fs-admin-frontend/DESIGN.md`, "Translation keys").
const FRONTEND: &[(&str, &str)] = &[
    ("action.add", "Add"),
    ("action.back", "Back"),
    ("action.cancel", "Cancel"),
    ("action.close", "Close"),
    ("action.remove", "Remove"),
    ("action.save", "Save"),
    ("menu.program", "Program"),
    ("menu.view", "View"),
    ("menu.help", "Help"),
    ("menu.quit", "Quit"),
    ("menu.about", "About"),
    ("message.done", "Done."),
    ("message.rejected", "The history program refused this."),
    (
        "message.not-found",
        "There is no such record or command (or it is not available yet).",
    ),
    ("message.unavailable", "The history cannot be read."),
    (
        "message.unsaved",
        "There is text that was not sent. Send or cancel it first.",
    ),
    ("sort.ascending", "ascending"),
    ("sort.descending", "descending"),
    ("finding.slot", "Value {n}"),
    ("list.empty", "Nothing here yet."),
    ("list.separator", ", "),
    (
        "list.stale",
        "Old: the history could not be read the last time.",
    ),
    ("detail.not-loaded", "This record is not loaded."),
    ("confirm.sure", "Are you sure?"),
    ("value.none", "none"),
    ("about.version", "Version"),
    ("about.origin", "Origin"),
    ("about.licence", "Licence"),
    ("about.repository", "Repository"),
    ("about.unoffered", "Not offered here"),
    ("chart.entries-per-day", "Entries per day"),
    ("chart.digest-per-program", "Digest per program"),
];

/// The English words of the program.
#[derive(Debug, Clone, Copy, Default)]
pub struct English;

impl Say for English {
    fn say(&self, key: &str) -> String {
        if let Some((_, text)) = FRONTEND.iter().find(|(k, _)| *k == key) {
            return (*text).to_owned();
        }
        humanise(key)
    }
}

/// `entry.object-type` -> `Object type`: the last dotted part, hyphens as spaces, first letter in capitals.
fn humanise(key: &str) -> String {
    let last = key.rsplit('.').next().unwrap_or(key).replace('-', " ");
    let mut chars = last.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}
