//! The one table of every command history has or is asked to have.
//!
//! Pattern: Registry (table-driven) — one local table is the single source for the type description, for
//! [`unsupported_commands`], for the backend's refusal and for the test against `requirements.md`. A command that is
//! missing in the core stays in the table with `supported = false`; it is never left out (coordinator rule, thread R).
//!
//! The description carries the flag (`CommandDescription::supported`, fs-types v0.0.11): every command is listed, a
//! missing one locked. The backend refuses it with a typed `Unsupported` all the same.


use fs_types::{BasicType, CommandKind};

/// A field of a command, as plain data (turned into a `FieldDescription` by `describe.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldSpec {
    /// Machine name, lower camel case.
    pub name: &'static str,
    /// Basic type.
    pub basic: BasicType,
    /// Must be given.
    pub required: bool,
    /// A list of values.
    pub multiple: bool,
    /// For `Reference`: the data type referred to.
    pub target: &'static str,
    /// For `Choice`: the allowed keys.
    pub choices: &'static [&'static str],
}

impl FieldSpec {
    const fn new(name: &'static str, basic: BasicType) -> Self {
        Self {
            name,
            basic,
            required: false,
            multiple: false,
            target: "",
            choices: &[],
        }
    }
    const fn required(mut self) -> Self {
        self.required = true;
        self
    }
    const fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }
    const fn to(mut self, target: &'static str) -> Self {
        self.target = target;
        self
    }
    const fn choose(mut self, choices: &'static [&'static str]) -> Self {
        self.choices = choices;
        self
    }
}

/// One command of one data type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRow {
    /// The data type it belongs to (`Entry`, `Digest`).
    pub data_type: &'static str,
    /// Machine name, lower camel case, unique in its type.
    pub name: &'static str,
    /// What it does to the data.
    pub kind: CommandKind,
    /// Whether the core can do it today.
    pub supported: bool,
    /// The capability id of the contract (`history.read` ...) or, where the core lacks it, the capability that would
    /// carry it.
    pub capability: &'static str,
    /// The first cell of its row in `requirements.md` (empty for commands of the contract itself).
    pub requirement: &'static str,
    /// The name `requirements.md` gives it (`contact.merge`), empty where it names none.
    pub requirement_name: &'static str,
    /// Why it is not supported (empty where it is).
    pub reason: &'static str,
    /// Input fields.
    pub input: Vec<FieldSpec>,
    /// Result fields.
    pub result: Vec<FieldSpec>,
}

/// The type names.
pub const ENTRY: &str = "Entry";
/// The type name of the grouped line.
pub const DIGEST: &str = "Digest";

const YES: &[&str] = &["yes"];

fn filter() -> Vec<FieldSpec> {
    vec![
        FieldSpec::new("from", BasicType::DateTime),
        FieldSpec::new("to", BasicType::DateTime),
        FieldSpec::new("program", BasicType::Text),
        FieldSpec::new("person", BasicType::Text),
        FieldSpec::new("objectType", BasicType::Text),
        FieldSpec::new("context", BasicType::Text),
        FieldSpec::new("object", BasicType::Reference),
    ]
}

fn row(
    data_type: &'static str,
    name: &'static str,
    kind: CommandKind,
    capability: &'static str,
) -> CommandRow {
    CommandRow {
        data_type,
        name,
        kind,
        supported: true,
        capability,
        requirement: "",
        requirement_name: "",
        reason: "",
        input: Vec::new(),
        result: Vec::new(),
    }
}

/// A command `requirements.md` asks for and the core cannot do.
fn missing(
    data_type: &'static str,
    name: &'static str,
    kind: CommandKind,
    requirement: &'static str,
    requirement_name: &'static str,
    reason: &'static str,
) -> CommandRow {
    CommandRow {
        supported: false,
        requirement,
        requirement_name,
        reason,
        ..row(data_type, name, kind, "")
    }
}

const NOT_BUILT: &str = "proposed in requirements.md, not built";
const OF_PEOPLE: &str = "a command on contacts, listed here by requirements.md; it belongs to fs-people";

/// Every command, supported or not.
#[must_use]
pub fn table() -> Vec<CommandRow> {
    let entry = FieldSpec::new("entry", BasicType::Reference).to(ENTRY);
    let mut find = row(ENTRY, "find", CommandKind::Find, "history.read");
    // The standard `find` (rule of types.capnp): `term` (empty = all) and `items`; the filter members are further
    // inputs. Paging with a cursor is part of the call, not a field.
    find.input = vec![FieldSpec::new("term", BasicType::Text)];
    find.input.extend(filter());
    find.result = vec![FieldSpec::new("items", BasicType::Reference).to(ENTRY).multiple()];
    let mut get = row(ENTRY, "get", CommandKind::Find, "history.read");
    get.input = vec![entry.required()];
    get.result = vec![FieldSpec::new("found", BasicType::Reference).to(ENTRY)];
    // One entry cannot be named to the core: it removes by filter. So the record command forgets the object of the
    // entry in its program (the filter `object` + `program`).
    let mut forget = row(ENTRY, "forget", CommandKind::Delete, "history.forget");
    forget.input = vec![entry.required()];
    forget.result = vec![FieldSpec::new("removed", BasicType::Text)];
    let mut forget_matching = row(ENTRY, "forgetMatching", CommandKind::Other, "history.forget");
    forget_matching.input = filter();
    forget_matching.input.push(FieldSpec::new("confirm", BasicType::Choice).required().choose(YES));
    forget_matching.result = vec![FieldSpec::new("removed", BasicType::Text)];
    let mut subscribe = row(ENTRY, "subscribe", CommandKind::Other, "history.subscribe");
    subscribe.input = filter();
    subscribe.supported = false;
    subscribe.reason = "a stream of entries; the core has it, but a command answers once (no request/response form yet)";
    let mut digest = row(DIGEST, "find", CommandKind::Find, "history.digest");
    digest.input = vec![FieldSpec::new("term", BasicType::Text)];
    digest.input.extend(filter());
    digest.input.push(FieldSpec::new("windowMinutes", BasicType::Text).required());
    digest.result = vec![FieldSpec::new("items", BasicType::Reference).to(DIGEST).multiple()];

    let mut rows = vec![find, get, forget, forget_matching, subscribe, digest];
    let mut add = |mut r: CommandRow, input: Vec<FieldSpec>, result: Vec<FieldSpec>| {
        r.input = input;
        r.result = result;
        rows.push(r);
    };
    let person = FieldSpec::new("person", BasicType::Reference);
    // Optional table of requirements.md, in its order.
    add(
        // A `change` takes an `Edit` and no input list (rule of types.capnp): the place and weather would be two
        // more fields of the entry, edited with it.
        missing(ENTRY, "change", CommandKind::Change,
            "Place and weather recorded with an entry", "", "an entry is a derived summary and cannot be edited; place and weather are proposed fields"),
        vec![],
        vec![],
    );
    add(
        missing(ENTRY, "onThisDay", CommandKind::Find,
            "\"On this day\": show events of the same date in earlier years", "", NOT_BUILT),
        vec![FieldSpec::new("date", BasicType::Date).required()],
        vec![FieldSpec::new("entries", BasicType::Reference).to(ENTRY).multiple()],
    );
    add(
        missing(ENTRY, "capture", CommandKind::Create,
            "Automatic capture of activity, raw events kept, summaries on demand", "", NOT_BUILT),
        vec![FieldSpec::new("bucket", BasicType::Text).required()],
        vec![],
    );
    add(
        missing(ENTRY, "categorise", CommandKind::Other,
            "Categorisation rules applied after capture", "", NOT_BUILT),
        vec![FieldSpec::new("rules", BasicType::Text).multiple()],
        vec![],
    );
    add(
        missing(ENTRY, "import", CommandKind::Create,
            "Import of events from other programs through one local interface", "", NOT_BUILT),
        vec![FieldSpec::new("source", BasicType::Link).required()],
        vec![FieldSpec::new("imported", BasicType::Text)],
    );
    add(
        missing(ENTRY, "correlate", CommandKind::Find,
            "Correlations between unrelated data series", "", NOT_BUILT),
        vec![FieldSpec::new("series", BasicType::Text).required().multiple()],
        vec![FieldSpec::new("correlations", BasicType::Text).multiple()],
    );
    add(
        missing(DIGEST, "weeklySummary", CommandKind::Find,
            "Weekly summary", "", NOT_BUILT),
        vec![FieldSpec::new("week", BasicType::Date)],
        vec![FieldSpec::new("digests", BasicType::Reference).to(DIGEST).multiple()],
    );
    // The rows of requirements.md that name contact commands (copied from the people analysis; they belong to
    // fs-people, but history lists them as asked, see DESIGN.md).
    let merge_req = "Suggest duplicates of one person and merge them one by one or all at once; merging can be undone (`contact.mergeSuggest`, `contact.merge`)";
    add(
        missing(ENTRY, "contactMergeSuggest", CommandKind::Suggest, merge_req, "contact.mergeSuggest", OF_PEOPLE),
        vec![person],
        vec![FieldSpec::new("duplicates", BasicType::Reference).multiple()],
    );
    add(
        missing(ENTRY, "contactMerge", CommandKind::Other, merge_req, "contact.merge", OF_PEOPLE),
        vec![person.required(), FieldSpec::new("with", BasicType::Reference).required().multiple()],
        vec![],
    );
    add(
        missing(ENTRY, "contactShareNearby", CommandKind::Other,
            "Exchange own contact details in person, with a choice of which fields go out; receive-only possible; never overwrites an existing contact silently (`contact.shareNearby`)",
            "contact.shareNearby", OF_PEOPLE),
        vec![FieldSpec::new("fields", BasicType::Keywords)],
        vec![],
    );
    add(
        missing(ENTRY, "contactPromote", CommandKind::Other,
            "Keep addresses of people only written to apart from the address book until promoted (`contact.promote`)",
            "contact.promote", OF_PEOPLE),
        vec![person.required()],
        vec![],
    );
    add(
        missing(ENTRY, "contactHistory", CommandKind::Find,
            "Change history of a contact, with undo (`contact.history`)",
            "contact.history", OF_PEOPLE),
        vec![person.required()],
        vec![FieldSpec::new("entries", BasicType::Reference).to(ENTRY).multiple()],
    );
    add(
        missing(ENTRY, "contactKeepInTouch", CommandKind::Other,
            "Remind to get back in touch; show when a person was last spoken to (`contact.keepInTouch`)",
            "contact.keepInTouch", OF_PEOPLE),
        vec![person.required(), FieldSpec::new("remindAt", BasicType::DateTime)],
        vec![FieldSpec::new("lastSpoken", BasicType::DateTime)],
    );
    add(
        missing(ENTRY, "contactLifeEvent", CommandKind::Create,
            "Record life events and milestones of a person (`contact.lifeEvent`)",
            "contact.lifeEvent", OF_PEOPLE),
        vec![person.required(), FieldSpec::new("title", BasicType::Text).required(), FieldSpec::new("at", BasicType::Date)],
        vec![],
    );
    rows
}

/// The commands of `data_type` that the core cannot do, by name.
#[must_use]
pub fn unsupported_commands(data_type: &str) -> Vec<&'static str> {
    table()
        .into_iter()
        .filter(|r| r.data_type == data_type && !r.supported)
        .map(|r| r.name)
        .collect()
}

/// The row of a command, if the table has it.
#[must_use]
pub fn find(data_type: &str, name: &str) -> Option<CommandRow> {
    table()
        .into_iter()
        .find(|r| r.data_type == data_type && r.name == name)
}
