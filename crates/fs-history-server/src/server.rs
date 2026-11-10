//! `History` root interface: capabilities, timeline, get, digest, forget, types.
//!
//! Pattern: Adapter — [`HistoryServer`] is what a program serves; it depends on the trait [`History`], never on
//! `HistoryRepo` (ADR 0043).

use std::rc::Rc;
use std::sync::Arc;

use capnp_rpc::new_client;
use fs_history_core::{History, registry};
use fs_interfaces::Unsupported;
use fs_interfaces::history_capnp::history;
use fs_interfaces::rpc::{read_object_id, run_to_end, unsupported};
use fs_types::describe::CatalogServer;

use crate::errors::rpc_error;
use crate::wire::{read_cursor, read_filter, write_digests, write_entries, write_entry};

/// The version of `history.capnp` this server answers.
pub const CONTRACT_VERSION: &str = "history 0.1 (draft)";

/// The root capability of the history program.
pub struct HistoryServer {
    history: Arc<dyn History>,
}

impl HistoryServer {
    /// Serve this provider.
    pub fn new(history: Arc<dyn History>) -> Self {
        Self { history }
    }

    /// The capability to hand to `fs_interfaces::rpc::serve`.
    #[must_use]
    pub fn into_client(self) -> history::Client {
        new_client(self)
    }
}

type Done = Result<(), capnp::Error>;

impl history::Server for HistoryServer {
    async fn capabilities(
        self: Rc<Self>,
        _: history::CapabilitiesParams,
        mut results: history::CapabilitiesResults,
    ) -> Done {
        self.history
            .capabilities()
            .write_list(CONTRACT_VERSION, results.get().init_capabilities());
        Ok(())
    }

    async fn support_report(
        self: Rc<Self>,
        _: history::SupportReportParams,
        mut results: history::SupportReportResults,
    ) -> Done {
        let report = self.history.support_report().await.map_err(rpc_error)?;
        report.write(CONTRACT_VERSION, results.get().init_report());
        Ok(())
    }

    async fn timeline(
        self: Rc<Self>,
        params: history::TimelineParams,
        mut results: history::TimelineResults,
    ) -> Done {
        let p = params.get()?;
        let filter = read_filter(p.get_filter()?)?;
        let cursor = read_cursor(p.get_cursor()?)?;
        let page = self
            .history
            .timeline(&filter, cursor.as_ref())
            .await
            .map_err(rpc_error)?;
        write_entries(results.get().init_page(), &page);
        Ok(())
    }

    async fn get(
        self: Rc<Self>,
        params: history::GetParams,
        mut results: history::GetResults,
    ) -> Done {
        let id = read_object_id(params.get()?.get_id()?)?;
        let found = self.history.get(id).await.map_err(rpc_error)?;
        let mut out = results.get();
        out.set_found(found.is_some());
        if let Some(entry) = found {
            write_entry(out.init_entry(), &entry);
        }
        Ok(())
    }

    async fn digest(
        self: Rc<Self>,
        params: history::DigestParams,
        mut results: history::DigestResults,
    ) -> Done {
        let p = params.get()?;
        let filter = read_filter(p.get_filter()?)?;
        let cursor = read_cursor(p.get_cursor()?)?;
        let window = p.get_window_minutes();
        let page = self
            .history
            .digest(&filter, window, cursor.as_ref())
            .await
            .map_err(rpc_error)?;
        write_digests(results.get().init_page(), &page);
        Ok(())
    }

    async fn forget(
        self: Rc<Self>,
        params: history::ForgetParams,
        mut results: history::ForgetResults,
    ) -> Done {
        let filter = read_filter(params.get()?.get_filter()?)?;
        let history = self.history.clone();
        // The forget writes tombstone and removals: it ends even if the caller goes away.
        let removed = run_to_end(async move { history.forget(&filter).await })
            .await?
            .map_err(rpc_error)?;
        results.get().set_removed(removed);
        Ok(())
    }

    async fn subscribe(
        self: Rc<Self>,
        _: history::SubscribeParams,
        _: history::SubscribeResults,
    ) -> Done {
        // The core has an in-process feed; its RPC listener (`EntryListener`) is not built yet.
        Err(unsupported(&Unsupported::new(
            "history.subscribe",
            "the live feed is in-process only; the RPC listener is not built",
        )))
    }

    async fn types(
        self: Rc<Self>,
        _: history::TypesParams,
        mut results: history::TypesResults,
    ) -> Done {
        let registry =
            registry().map_err(|e| capnp::Error::failed(format!("types.unsound: {e:?}")))?;
        results
            .get()
            .set_catalog(new_client(CatalogServer::new(registry)));
        Ok(())
    }
}
