//! Paging: an opaque cursor and one page of results.
//!
//! Pattern: Iterator (cursor paging) — the caller hands back `next` until it is `None`. The cursor is
//! opaque to callers; this core encodes the position of the last item (time and id), so paging stays
//! stable when entries arrive between two calls.

/// Opaque position in a list (`Cursor` in `common.capnp`; none = start).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor(pub String);

/// One page of results; `next` is `None` on the last page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page<T> {
    /// The items of this page.
    pub items: Vec<T>,
    /// Where the next page starts.
    pub next: Option<Cursor>,
}
