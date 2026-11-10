//! `fs-history-server`: the contract `history.capnp` over a Unix socket, in front of any provider of
//! [`History`]. It only translates: no history logic lives here.
//!
//! Besides the data methods it serves `types` (ADR 0066): the catalog of `Entry` and `Digest`, built from the
//! description in `fs-history-core`, so a generated frontend needs nothing else.
//!
//! Interface status: draft (ADR 0045). Not served (named in `DESIGN.md`): `subscribe` over RPC answers
//! `Unsupported`; the in-process feed of the core exists, its RPC listener is not built.
//!
//! Pattern: Adapter — each method of the contract is one call of the provider; a Facade file declares the modules.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// The generated `Server` trait is async for every method, also those that never wait.
#![allow(clippy::missing_errors_doc, clippy::unused_async_trait_impl)]

mod errors;
mod server;
mod wire;

pub use server::{CONTRACT_VERSION, HistoryServer};
