//! Error type of the history core.
//!
//! Pattern: none (plain error enum via `thiserror`) — one `Error` for every public operation, with
//! `From` conversions for the store and JSON layers.

/// Result of every fallible operation of this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong in the history core.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The object store failed.
    #[error("object store: {0}")]
    Store(#[from] fs_store::Error),
    /// Stored JSON could not be read or written.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// A stored object is not a readable entry.
    #[error("invalid stored entry: {0}")]
    Invalid(String),
    /// The request is refused on purpose (an empty forget filter, a type that is never read).
    #[error("refused: {0}")]
    Refused(String),
    /// The activity source failed (pulling or acknowledging).
    #[error("activity source: {0}")]
    Source(String),
    /// The explicit "not possible" of a provider (ADR 0045).
    #[error(transparent)]
    Unsupported(#[from] fs_interfaces::Unsupported),
}
