//! The data of the two chart screens: entries per day, digest lines per program.
//!
//! Pattern: Strategy — each is a [`ChartSource`] (trait of `fs-components-chart-surface`): it reads the history and gives a
//! Vega-Lite spec as text. It chooses data and the kind of mark, never a pixel or a colour (ADR 0066).

use std::sync::Arc;

use fs_components_chart_surface::ChartSource;
use fs_history_core::{EntryFilter, History};
use serde_json::{Value, json};
use tokio::runtime::Handle;

use crate::MAX_LIST;

fn read<T>(
    handle: &Handle,
    work: impl std::future::Future<Output = fs_history_core::Result<T>>,
) -> Option<T> {
    handle.block_on(work).ok()
}

/// How many entries per day, by program: the timeline as a chart.
pub struct EntriesPerDay {
    history: Arc<dyn History>,
    handle: Handle,
    filter: EntryFilter,
}

impl EntriesPerDay {
    /// The entries that pass `filter` (empty = all), newest first, up to [`MAX_LIST`].
    pub fn new(history: Arc<dyn History>, handle: Handle, filter: EntryFilter) -> Self {
        Self {
            history,
            handle,
            filter,
        }
    }

    fn rows(&self) -> Vec<Value> {
        let mut rows = Vec::new();
        let mut cursor = None;
        while rows.len() < MAX_LIST {
            let Some(page) = read(
                &self.handle,
                self.history.timeline(&self.filter, cursor.as_ref()),
            ) else {
                break;
            };
            rows.extend(page.items.iter().map(
                |e| json!({"day": e.at.format("%Y-%m-%d").to_string(), "program": e.program}),
            ));
            cursor = page.next;
            if cursor.is_none() {
                break;
            }
        }
        rows
    }
}

impl ChartSource for EntriesPerDay {
    fn spec(&self) -> String {
        json!({
            "title": "Entries per day",
            "data": {"values": self.rows()},
            "mark": "bar",
            "encoding": {
                "x": {"field": "day", "type": "ordinal"},
                "y": {"aggregate": "count", "type": "quantitative"},
                "color": {"field": "program", "type": "nominal"}
            }
        })
        .to_string()
    }
}

/// How many changes the digest lines hold per program.
pub struct DigestPerProgram {
    history: Arc<dyn History>,
    handle: Handle,
    filter: EntryFilter,
    window_minutes: u32,
}

impl DigestPerProgram {
    /// The digest of the entries that pass `filter`, lines at most `window_minutes` apart.
    pub fn new(
        history: Arc<dyn History>,
        handle: Handle,
        filter: EntryFilter,
        window_minutes: u32,
    ) -> Self {
        Self {
            history,
            handle,
            filter,
            window_minutes,
        }
    }

    fn rows(&self) -> Vec<Value> {
        let mut rows = Vec::new();
        let mut cursor = None;
        while rows.len() < MAX_LIST {
            let Some(page) = read(
                &self.handle,
                self.history
                    .digest(&self.filter, self.window_minutes, cursor.as_ref()),
            ) else {
                break;
            };
            rows.extend(
                page.items
                    .iter()
                    .map(|d| json!({"program": d.first.program, "changes": d.count})),
            );
            cursor = page.next;
            if cursor.is_none() {
                break;
            }
        }
        rows
    }
}

impl ChartSource for DigestPerProgram {
    fn spec(&self) -> String {
        json!({
            "title": "Changes per program (digest)",
            "data": {"values": self.rows()},
            "mark": "bar",
            "encoding": {
                "x": {"field": "program", "type": "nominal"},
                "y": {"aggregate": "sum", "field": "changes", "type": "quantitative"}
            }
        })
        .to_string()
    }
}
