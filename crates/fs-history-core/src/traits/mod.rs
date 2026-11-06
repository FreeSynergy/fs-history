//! The contract as Rust traits: module map.
//!
//! Pattern: Facade (declarations) — one file for the one root interface of `history.capnp`.

mod history;

pub use history::History;
