# fs-history – Design

`fs-history` observes what happens across programs (ADR 0020). This repo holds the **program core**, no GUI. Contract:
`history.capnp` in `fs-interfaces` v0.20.0 or later (draft, provisional). Program docs: `fs-documentation/docs/en/programs/apps/history/`.

## State

Version 0.1.0. No capnp server and no GUI.

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
| capnp server | not built; thread 3 |
| Tests | 45, real stores and a real hub |

Checked by an independent reviewer over two rounds before the tag.

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
