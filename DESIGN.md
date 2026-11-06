# fs-history – Design

`fs-history` observes what happens across programs (ADR 0020). This repo holds the **program core**, no GUI. Contract:
`history.capnp` in `fs-interfaces` v0.17.0 (draft, provisional). Program docs: `fs-documentation/docs/en/programs/apps/history/`.

## State

Version 0.1.0, being built. The State table is filled in as parts are finished.

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
* `forget(filter)` removes matching entries and stores a tombstone: the removed activity ids and the filter. Ingest checks both, so a redelivery after a crash and a late push that falls into a forgotten range stay out. Tombstones are unlogged too. "For good" is this index only; the hub's log is not touched, and the interface must say so. An empty filter is refused, and all-zero values count as unset.

### Ingest
Port `ActivitySource` (Strategy): `pull(limit) -> Batch { items: Vec<SourceActivity>, last, more }`, `ack(last)`. `SourceActivity` is a plain value with the summary fields, so the core does not know the hub. `Observer::run_once` (Template method): pull, drop what tombstones say, write, ack only after the write; idempotent by activity id, so redelivery changes nothing. `Observer::catch_up` loops while `more`. The payload is not read or kept.

### Digest
A pure function: entries newest first → grouped when object and program are equal and the gap between neighbours is at most `windowMinutes`. Paged over entries, so one group is never split across pages without saying so: a page ends only at a group boundary.

### Feed
`added` and `forgotten` through `tokio::sync::broadcast`, filtered per subscription; dropping the guard unsubscribes. In-process only, the RPC server comes from thread 3.

### Not built
Telemetry import (OpenTelemetry not read), time tracking derived from history, sharing and group views. They have no contract method yet.
