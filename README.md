# fs-history

FreeSynergy program that answers: what have I, or we, done? It observes what happens across programs and shows it as a timeline.

* `crates/fs-history-core`: the program core, a derived local index (no GUI, no capnp server yet). See `DESIGN.md`.
* `crates/fs-history-hub`: feeds the core from the hub.
* The contract is `history.capnp` in [fs-interfaces](https://github.com/FreeSynergy/fs-interfaces) (tag v0.17.0).

Licence: MIT OR Apache-2.0.
