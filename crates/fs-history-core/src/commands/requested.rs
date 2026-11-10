//! The commands `requirements.md` asks for and the core does not have (all `supported = false`).
//!
//! Pattern: Registry (table rows, data only); a child of `commands`, whose helpers it uses.

use fs_types::{BasicType, CommandKind};

use super::{CommandRow, DIGEST, ENTRY, FieldSpec, NOT_BUILT, OF_PEOPLE, missing};

/// The commands `requirements.md` asks for; the core has none of them.
pub(super) fn requested() -> Vec<CommandRow> {
    let mut rows = ideas();
    rows.extend(contact_commands());
    rows
}

/// A row with its fields.
fn with(mut r: CommandRow, input: Vec<FieldSpec>, result: Vec<FieldSpec>) -> CommandRow {
    r.input = input;
    r.result = result;
    r
}

// A data table: its length is its rows, not logic.
#[allow(clippy::too_many_lines)]
/// The ideas of the optional table of `requirements.md` that name no command.
fn ideas() -> Vec<CommandRow> {
    let mut rows = Vec::new();
    let mut add = |r: CommandRow, input: Vec<FieldSpec>, result: Vec<FieldSpec>| {
        rows.push(with(r, input, result));
    };
    // Optional table of requirements.md, in its order.
    add(
        // A `change` takes an `Edit` and no input list (rule of types.capnp): the place and weather would be two
        // more fields of the entry, edited with it.
        missing(
            ENTRY,
            "change",
            CommandKind::Change,
            "Place and weather recorded with an entry",
            "",
            "an entry is a derived summary and cannot be edited; place and weather are proposed fields",
        ),
        vec![],
        vec![],
    );
    add(
        missing(
            ENTRY,
            "onThisDay",
            CommandKind::Find,
            "\"On this day\": show events of the same date in earlier years",
            "",
            NOT_BUILT,
        ),
        vec![FieldSpec::new("date", BasicType::Date).required()],
        vec![
            FieldSpec::new("entries", BasicType::Reference)
                .to(ENTRY)
                .multiple(),
        ],
    );
    add(
        missing(
            ENTRY,
            "capture",
            CommandKind::Create,
            "Automatic capture of activity, raw events kept, summaries on demand",
            "",
            NOT_BUILT,
        ),
        vec![FieldSpec::new("bucket", BasicType::Text).required()],
        vec![],
    );
    add(
        missing(
            ENTRY,
            "categorise",
            CommandKind::Other,
            "Categorisation rules applied after capture",
            "",
            NOT_BUILT,
        ),
        vec![FieldSpec::new("rules", BasicType::Text).multiple()],
        vec![],
    );
    add(
        missing(
            ENTRY,
            "import",
            CommandKind::Create,
            "Import of events from other programs through one local interface",
            "",
            NOT_BUILT,
        ),
        vec![FieldSpec::new("source", BasicType::Link).required()],
        vec![FieldSpec::new("imported", BasicType::Text)],
    );
    add(
        missing(
            ENTRY,
            "correlate",
            CommandKind::Find,
            "Correlations between unrelated data series",
            "",
            NOT_BUILT,
        ),
        vec![
            FieldSpec::new("series", BasicType::Text)
                .required()
                .multiple(),
        ],
        vec![FieldSpec::new("correlations", BasicType::Text).multiple()],
    );
    add(
        missing(
            DIGEST,
            "weeklySummary",
            CommandKind::Find,
            "Weekly summary",
            "",
            NOT_BUILT,
        ),
        vec![FieldSpec::new("week", BasicType::Date)],
        vec![
            FieldSpec::new("digests", BasicType::Reference)
                .to(DIGEST)
                .multiple(),
        ],
    );
    // The rows of requirements.md that name contact commands (copied from the people analysis; they belong to
    // fs-people, but history lists them as asked, see DESIGN.md).
    rows
}

// A data table: its length is its rows, not logic.
#[allow(clippy::too_many_lines)]
/// The rows of `requirements.md` that name contact commands (they belong to fs-people, see `DESIGN.md`).
fn contact_commands() -> Vec<CommandRow> {
    let mut rows = Vec::new();
    let mut add = |r: CommandRow, input: Vec<FieldSpec>, result: Vec<FieldSpec>| {
        rows.push(with(r, input, result));
    };
    let person = FieldSpec::new("person", BasicType::Reference);
    let merge_req = "Suggest duplicates of one person and merge them one by one or all at once; merging can be undone (`contact.mergeSuggest`, `contact.merge`)";
    add(
        missing(
            ENTRY,
            "contactMergeSuggest",
            CommandKind::Suggest,
            merge_req,
            "contact.mergeSuggest",
            OF_PEOPLE,
        ),
        vec![person],
        vec![FieldSpec::new("duplicates", BasicType::Reference).multiple()],
    );
    add(
        missing(
            ENTRY,
            "contactMerge",
            CommandKind::Other,
            merge_req,
            "contact.merge",
            OF_PEOPLE,
        ),
        vec![
            person.required(),
            FieldSpec::new("with", BasicType::Reference)
                .required()
                .multiple(),
        ],
        vec![],
    );
    add(
        missing(
            ENTRY,
            "contactShareNearby",
            CommandKind::Other,
            "Exchange own contact details in person, with a choice of which fields go out; receive-only possible; never overwrites an existing contact silently (`contact.shareNearby`)",
            "contact.shareNearby",
            OF_PEOPLE,
        ),
        vec![FieldSpec::new("fields", BasicType::Keywords)],
        vec![],
    );
    add(
        missing(
            ENTRY,
            "contactPromote",
            CommandKind::Other,
            "Keep addresses of people only written to apart from the address book until promoted (`contact.promote`)",
            "contact.promote",
            OF_PEOPLE,
        ),
        vec![person.required()],
        vec![],
    );
    add(
        missing(
            ENTRY,
            "contactHistory",
            CommandKind::Find,
            "Change history of a contact, with undo (`contact.history`)",
            "contact.history",
            OF_PEOPLE,
        ),
        vec![person.required()],
        vec![
            FieldSpec::new("entries", BasicType::Reference)
                .to(ENTRY)
                .multiple(),
        ],
    );
    add(
        missing(
            ENTRY,
            "contactKeepInTouch",
            CommandKind::Other,
            "Remind to get back in touch; show when a person was last spoken to (`contact.keepInTouch`)",
            "contact.keepInTouch",
            OF_PEOPLE,
        ),
        vec![
            person.required(),
            FieldSpec::new("remindAt", BasicType::DateTime),
        ],
        vec![FieldSpec::new("lastSpoken", BasicType::DateTime)],
    );
    add(
        missing(
            ENTRY,
            "contactLifeEvent",
            CommandKind::Create,
            "Record life events and milestones of a person (`contact.lifeEvent`)",
            "contact.lifeEvent",
            OF_PEOPLE,
        ),
        vec![
            person.required(),
            FieldSpec::new("title", BasicType::Text).required(),
            FieldSpec::new("at", BasicType::Date),
        ],
        vec![],
    );
    rows
}
