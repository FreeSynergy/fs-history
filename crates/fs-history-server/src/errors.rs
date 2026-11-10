//! Errors of the core as RPC errors.
//!
//! Pattern: Adapter — each error the core can name becomes a typed RFC 9457 problem, so a consumer can tell "not
//! possible" from "refused" without reading a message.

use fs_history_core::Error;
use fs_interfaces::rpc::{failed, invalid_request, unsupported};

/// A core error as an RPC error.
#[allow(clippy::needless_pass_by_value)] // used as `map_err(rpc_error)`
pub(crate) fn rpc_error(e: Error) -> capnp::Error {
    match &e {
        Error::Unsupported(not_possible) => unsupported(not_possible),
        // an empty forget filter, a type that is never read: the caller asked for something refused on purpose
        Error::Refused(_) => invalid_request(e.to_string()),
        Error::Store(_) | Error::Json(_) | Error::Invalid(_) | Error::Source(_) => {
            failed(e.to_string())
        }
    }
}
