# FreeSynergy History — remember what I or we did

**A timeline of your own work, without anyone watching you.**

FreeSynergy History notices what happens across your FreeSynergy programs and keeps a short line for each event: "edited this page", "finished that task". You can scroll it, filter it, collapse a busy hour into one line, and forget parts of it.

What is different here: it keeps a summary, never the content. A diary text stays a wiki page, and History only points to it. The history is local to you. Sensitive things never arrive in it. Forgetting is real for the past: matching entries are removed and a record remembers only salted hashes of what you forgot, so a forgotten project name is not stored in readable form.

## What it can do today

Latest tag: `v0.1.1`.

- A timeline of activities from other programs, newest first, with filters and paging.
- A digest that groups runs of events on the same object.
- Forget entries by filter, for the past only.
- Reading from the FreeSynergy hub (adapter `fs-history-hub`), tested against a real hub.
- Never reads relationships or survey answers.
- Tests: 49, with real stores and a real hub.
- Checked by an independent reviewer over three rounds.

## What is not there yet

- No window and no server.
- Who did something is empty for now, and the time is when the entry arrived, not necessarily when it happened (see DESIGN.md).
- Forgetting covers this index only. The hub's own log is not touched.
- No sharing and no group view. No import of telemetry, no time tracking.
- A hub connection that dies is not reconnected by itself.
- The interface is a draft, marked provisional.

## Part of FreeSynergy

History belongs to the project-management suite, and to anyone who wants to remember. FreeSynergy is a family of programs that are free for everyone, forever, and that keep your data yours. The documentation, the manifesto and the overall plan live in [fs-documentation](https://github.com/FreeSynergy/fs-documentation); the reading version is at <https://freesynergy.github.io/fs-documentation/>. Start with the [manifesto](https://freesynergy.github.io/fs-documentation/manifesto/) if you want to know why we do this.


## For developers

- Workspace crates: `fs-history-core` (core, no GUI), `fs-history-hub` (reads activities from the hub).
- The contract is `history.capnp` in [fs-interfaces](https://github.com/FreeSynergy/fs-interfaces) (draft, provisional). History owns it.
- Build and test: `cargo build --workspace` and `cargo test --workspace`.
- Design and decisions: [DESIGN.md](DESIGN.md).
- Licence: MIT OR Apache-2.0.
