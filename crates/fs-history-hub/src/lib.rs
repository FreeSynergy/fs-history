//! `fs-history-hub`: feeds the history core from the FreeSynergy hub.
//!
//! [`HubSource`] is an `ActivitySource` over a registered `HubSession`; [`manifest`] is what history
//! tells the hub when it registers: the default `reads` and no `writes`. Everything that knows the
//! hub, its types and its store version stays in this crate, so a newer hub tag touches only this.
//!
//! Pattern: Facade — this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]

mod manifest;
mod map;
mod source;

pub use manifest::{PROGRAM, manifest, manifest_with};
pub use source::HubSource;
