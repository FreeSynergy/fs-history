# fs-history – Design

`fs-history` observes what happens across programs (ADR 0020). This repo holds the **program core**, no GUI. Contract:
`history.capnp` in `fs-interfaces` v0.0.11 (draft, provisional). Program docs: `fs-documentation/docs/en/programs/apps/history/`.

## State

Version 0.0.1 (branch `r3-admin`, thread R3, not tagged). Core, capnp server (`types` included) and window.

| Area | State |
|---|---|
| Entries: derived index, unlogged, no payload; timeline, get, filters, keyset paging | done |
| Digest | done |
| Forget with tombstones, past only, salted filter texts | done |
| Observer over the port `ActivitySource`; hub adapter `fs-history-hub` against a real hub | done; at most 16 activities per pull until the hub's next tag |
| `Relationship` and `SurveyResponse` never read | done |
| Actor | empty; `originalActor` stays empty until the person releases it (strategy, in `hub.capnp`) |
| Original time | receive time for now; `originalAt` comes with the next fs-hub tag |
| A dead hub session | not reconnected; the host starts a new one |
| capnp server | `fs-history-server`: all methods except `subscribe` (answers `Unsupported`), `types` included, tested over a real socket |
| Tests | 49, real stores and a real hub, the 80 MiB test included |

Checked by an independent reviewer over three rounds; the last found one small error, fixed in 0.1.1 with a test.

## Decisions (the owner may change them; all provisional)

### Does it have its own data?
No content of its own, but it needs an index. The hub only offers a cursor-based `pull`: no query by time, person or project, and a view would have to scan the whole log each time. So entries are a **derived index**: a summary of each activity, never its `payload`. A diary's text stays a wiki entry (ADR 0031). So it stays a small program, and the views are its frontends.

### An entry
One `StoredObject` per delivered activity; envelope type `Activity`, `context` = the object's context, `published` = `Entry.at`, id = the hub activity id made into an `ObjectId` where the types allow it, otherwise an index field `activity`. Index fields: `activity`, `program`, `kind`, `object`, `objectType`, `person`. Content: the summary fields. Written with `UnloggedObjects`, because the index is derived: logging it would double every activity and send it towards the hub again. Query through `ObjectQuery`, ordered `PublishedDesc` with a tie break on id so paging is stable.

### Who and when: limits of today's hub
The hub's `Activity.actor` is the program name and `at` is when the hub applied the push. So `Entry.actor` is empty, `at` is the receive time, and `atIsReceived` is true. An offline week arrives all at once and "what did I edit in someone else's object" cannot be answered. **Requested from thread 3:** append the original actor and time to `hub.capnp` `Activity`. When they arrive, the source fills them in; the contract already has the fields. `person` in a filter matches `actor`, or `attributedTo` where `actor` is empty.

### Log-only verbs
`Invite`, `Accept` and the like carry only an id in their envelope. Name, type, context and `attributedTo` are filled in from the object's latest entry, if there is one.

### Privacy
* Local to this person. No sharing and no group view in this version: it comes with the sharing interface.
* `Relationship` is never read (sprint rule, role privacy). `reads` is an explicit default list without it, and a constructor refuses a list that names it. Survey answers (`SurveyResponse`) are not read either.
* Sensitive objects never arrive (ADR 0029). The hub reports an object that became sensitive as a `Delete`, so **on `Delete` all entries of that object lose `name` and `context`**, and `objectType` stays. A delete therefore also removes the name from the history; provisional, because the two cases cannot be told apart.
* `forget(filter)` removes the matching entries and stores a tombstone: the forget time, the removed activity ids, the removed object ids, and the filter. **Forgetting covers the past only.** An activity counts as forgotten if its id was removed, or if its `at` is at or before the forget time and it matches the filter or names a removed object; later activities are recorded again. While `at` is the hub's receive time, a late push gets a later time and comes back; that goes away once the hub delivers the original time.
* Filter texts in a tombstone are salted hashes (SHA-256, a random salt per tombstone), so a forgotten project name is not kept readable. Stated limit: a guessed value can still be tested. Forgetting happens in this index only; the hub's log is not touched, and the interface must say so. An empty filter is refused, and all-zero values count as unset.
* `Update` after `Delete` with no new `Create` stays blank. An `Undo` is not handled, since the hub's log has none.
* Types are compared normalised (case, `as:` prefix). A log-only verb of unknown type is kept only from a source that filters types itself (`ActivitySource::filters_types`, true for the hub).

### Ingest
Port `ActivitySource` (Strategy): `pull(limit) -> Batch { items: Vec<SourceActivity>, last, more }`, `ack(last)`. `SourceActivity` is a plain value with the summary fields, so the core does not know the hub. `Observer::run_once` (Template method): pull, drop what tombstones say, write, ack only after the write; idempotent by activity id, so redelivery changes nothing. `Observer::catch_up` loops while `more`. The payload is not read or kept.

### Digest
A pure function: entries newest first → grouped when object and program are equal and the gap between neighbours is at most `windowMinutes`. Paged by keyset. A page ends where no run of one object and program crosses. Over 10 000 entries a run is cut, and `DigestEntry.continues` says so.

### Feed
`added` and `forgotten` through `tokio::sync::broadcast`, filtered per subscription; dropping the guard unsubscribes. In-process only, the RPC server comes from thread 3.

### Not built
Telemetry import (OpenTelemetry not read), time tracking derived from history, sharing and group views. They have no contract method yet.

## Thread R3: type description, server, window (2026-11-10, provisional)

### Data types and the one command table
Two types are described (`describe.rs`, `impl DescribesType`): `Entry` (AS2 `Activity`, summary) and `Digest` (grouped line). Fields are the members of `Entry` and `DigestEntry` in `history.capnp`. **One table** (`commands.rs`: type, name, kind, `supported`, capability, requirement text, input, result) is the source of the description, of `unsupported_commands()`, of the backend's `supported()` and refusal, and of the test against `requirements.md`. The description lists **every** command; a missing one carries `supported = false` (fs-types v0.0.11, `CommandDescription::unsupported()`). Switched from the transitional rule (flag only in the table) to the schema flag: one line in `describe.rs::describe_command`.

| Type | Command | Kind | State |
|---|---|---|---|
| Entry | `find` (standard find: `term` + filter members `from to program person objectType context object`) | find | done (`history.read`; `term` is matched in the program over name, program, verb, type, context) |
| Entry | `get` | find | done |
| Entry | `forget` (this entry's object in its program) | delete | done (`history.forget`) |
| Entry | `forgetMatching` (filter + `confirm`) | other | done; an empty filter is refused |
| Entry | `subscribe` | other | Unsupported: a stream; the core has it in process, a command answers once, the RPC listener is not built |
| Entry | `change` (place and weather) | change | Unsupported: an entry is a derived summary; proposed |
| Entry | `onThisDay`, `capture`, `categorise`, `import`, `correlate` | find/create/other | Unsupported: proposed in requirements.md, not built |
| Digest | `find` (+ `windowMinutes`) | find | done (`history.digest`) |
| Digest | `weeklySummary` | find | Unsupported: proposed |
| Entry | `contactMergeSuggest`, `contactMerge`, `contactShareNearby`, `contactPromote`, `contactHistory`, `contactKeepInTouch`, `contactLifeEvent` | various | Unsupported. **Probably misfiled:** these seven rows of `requirements.md` (names `contact.*`) are copied from the people analysis (Google/Apple Contacts, Monica) and belong to fs-people. They are in the table as the rule demands; the question goes to the strategy |

The test `tests/describe.rs` reads every row of `requirements.md` (a snapshot in `tests/data/requirements.md`; `FS_HISTORY_REQUIREMENTS=<path>` checks the live file) and requires a table row for each row and each named command, and that every supported row is a capability of the provider (test against the trait `History`).

Not described (no basic type yet, `Unsupported`): `atIsReceived` and `continues` (yes/no), `count` is text with the decimal number, `place`/`weather` fields.

### Pattern map
| Where | Pattern | Why |
|---|---|---|
| `commands.rs` table | Registry (table-driven) | one source for description, refusal and tests |
| `describe.rs` | Registry entry + Adapter | `DescribesType` for `Entry`, `DigestEntry` |
| `fs-history-server` | Adapter | each method of the contract is one call of the trait `History` |
| `fs-history-app` `HistoryBackend` | Adapter + Repository | the frontend's `Backend` over `History`, one per type |
| `fs-history-app` `EntriesPerDay`, `DigestPerProgram` | Strategy | `ChartSource`: data and spec only, no pixel (ADR 0066) |
| `fs-components-chart-surface` (in fs-components, branch `r3-history-charts`) | Strategy + Adapter | the chart screen of the shell; this is where the drawing lives |

### The window
`fs-history` (crate `fs-history`) opens `fs-history-app::gui::run`: `admin_window` of the generated frontend with a screen for `Entry` and one for `Digest`; no render code, no `blinc_*` import. It reads the history index from a store file (`--store`, default `$XDG_DATA_HOME/freesynergy/history/index.db`) in process. Blinc through `blinc-app-fix` / `blinc-layout-fix` at the pins of fs-tasks (`5180d4ea`, `cb17db81`). fs-components at one commit `99a6575` (branch `r3-history-charts` = `thread-a-admin-frontend` 7b47a65 + the chart crate), provisional until merged and tagged.

### Unsupported (named, not faked)
* **Charts in the main window.** `admin_window` takes only data-type surfaces; the two charts open as their own window (`--chart entries-per-day|digest-per-program`) until the frontend takes extra screens (thread A).
* **`BackendError` has no variant for "not possible".** `HistoryBackend::run_checked` answers a typed `RunError::Unsupported`; `Backend::run` can only say `NotFound` for such a command. `Backend::supported` answers false for them, so the frontend shows them locked.
* **Nothing fills the index in this window.** The observer over the hub (`fs-history-hub`) has no process; the window shows what the store file holds. A server/daemon (`fs-history serve`) and the remote client (as fs-people's `RemotePeople`) are not built; the window uses the core in process.
* `subscribe` over RPC (see table); the list is cut at 2000 records (`MAX_LIST`), the filter is how a person narrows it.
* Words: English only, in `words.rs` (ADR 0053 will replace it). Entries cannot be changed (the index is derived): no `change`.
