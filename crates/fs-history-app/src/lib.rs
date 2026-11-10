//! `fs-history-app`: the window of the history program (ADR 0005, 0063, 0065, 0066).
//!
//! The window shell and the generated Admin frontend come from `fs-components`; this crate gives them data and the trait
//! implementations they ask for: the description (from `fs-history-core`), a [`HistoryBackend`] per data type and two
//! [`ChartSource`](fs_components_chart_surface::ChartSource)s. It draws nothing and imports no `blinc_*` crate.
//!
//! Pattern: Facade — this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate, clippy::module_name_repetitions)]

mod backend;
mod charts;
pub mod gui;
mod words;

pub use backend::{HistoryBackend, MAX_LIST, RunError};
pub use charts::{DigestPerProgram, EntriesPerDay};
pub use words::{English, TITLE};
