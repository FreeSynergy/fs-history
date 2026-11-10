//! The window of `fs-history` (ADR 0063): wiring only.
//!
//! Pattern: Facade consumer — open the history index, read the catalog of type descriptions, give every data type a
//! surface of the generated Admin frontend, add the two chart screens, and put them in the standard window shell
//! (ADR 0065). The index is read in this process from the store file (no server needed); what fills it is the observer
//! over the hub (`fs-history-hub`), which has no process of its own yet (`DESIGN.md`, Unsupported).

use std::path::PathBuf;
use std::sync::Arc;

use fs_admin_frontend::view::{AboutInfo, AdminSurface};
use fs_admin_frontend::window::{TypeView, admin_window};
use fs_admin_frontend::{Backend, Session, SystemClock};
use fs_components_app::{AppError, AppWidget, Screen, WindowOptions, launch, parse_size};
use fs_components_chart_surface::ChartSurface;
use fs_components_charts::Labels;
use fs_components_fields::{Look, Say};
use fs_components_styles::StyleId;
use fs_history_core::commands::{DIGEST, ENTRY};
use fs_history_core::{EntryFilter, History, HistoryRepo, registry};
use fs_objects::TursoStore;
use fs_objects::traits::UnloggedObjects;
use tokio::runtime::Handle;

use crate::words::{English, TITLE};
use crate::{DigestPerProgram, EntriesPerDay, HistoryBackend};

/// The help text.
pub const HELP: &str = "fs-history: the history window (without arguments it opens)

  --store <file>   the history index to read (default: $XDG_DATA_HOME/freesynergy/history/index.db)
  --chart <name>   open a chart instead of the lists: entries-per-day or digest-per-program
                   (provisional: the charts join the main window when the generated frontend takes extra screens)
  --size <WxH>     window size (default 1200x800)";

/// The page the Help entry opens.
pub const HELP_URL: &str = "https://github.com/FreeSynergy/fs-documentation/blob/main/docs/en/programs/apps/history/index.md";

/// Minutes within which changes to one object by one program are one digest line, in the chart.
const CHART_WINDOW: u32 = 60;

fn about() -> AboutInfo {
    AboutInfo {
        name: "FreeSynergy.History".into(),
        version: format!("v{}", env!("CARGO_PKG_VERSION")),
        origin: "https://github.com/FreeSynergy/fs-history".into(),
        licence: "MIT OR Apache-2.0".into(),
        repository: "https://github.com/FreeSynergy/fs-history".into(),
    }
}

/// The window over `history`: a screen per data type of the catalog and a screen per chart.
///
/// # Errors
/// [`AppError`] if the shell cannot be made; the text of a description defect.
pub fn app_widget(history: &Arc<dyn History>, handle: &Handle) -> Result<AppWidget, String> {
    let catalog = registry().map_err(|e| format!("types.unsound: {e:?}"))?;
    let say: Arc<dyn Say> = Arc::new(English);
    let views: Vec<TypeView> = [ENTRY, DIGEST]
        .iter()
        .filter_map(|name| catalog.get(name).map(|d| (*name, d.clone())))
        .map(|(name, description)| {
            let backend: Arc<dyn Backend> = Arc::new(if name == ENTRY {
                HistoryBackend::entries(Arc::clone(history), handle.clone())
            } else {
                HistoryBackend::digests(Arc::clone(history), handle.clone())
            });
            let session = Session::new(description.clone(), backend, Arc::new(SystemClock::new()));
            TypeView {
                id: name.to_owned(),
                label: description.label.clone(),
                surface: Arc::new(AdminSurface::new(
                    session,
                    Look::new(StyleId::Admin, false),
                    Arc::clone(&say),
                    None,
                )),
            }
        })
        .collect();
    admin_window(TITLE, &say, views, about(), HELP_URL.to_owned())
        .map_err(|e: AppError| e.to_string())
}

/// The two chart screens, for the shell of a program that adds screens itself.
pub fn chart_surfaces(
    history: &Arc<dyn History>,
    handle: &Handle,
) -> [(&'static str, ChartSurface); 2] {
    [
        (
            "chart.entries-per-day",
            ChartSurface::new(
                Arc::new(EntriesPerDay::new(
                    Arc::clone(history),
                    handle.clone(),
                    EntryFilter::default(),
                )),
                Labels::default(),
            ),
        ),
        (
            "chart.digest-per-program",
            ChartSurface::new(
                Arc::new(DigestPerProgram::new(
                    Arc::clone(history),
                    handle.clone(),
                    EntryFilter::default(),
                    CHART_WINDOW,
                )),
                Labels::default(),
            ),
        ),
    ]
}

/// A window with one chart screen. `name` is `entries-per-day` or `digest-per-program`.
///
/// # Errors
/// The text for the person: an unknown chart name, or the shell cannot be made.
pub fn chart_widget(
    history: &Arc<dyn History>,
    handle: &Handle,
    name: &str,
) -> Result<AppWidget, String> {
    let say = English;
    let key = format!("chart.{name}");
    let surface = chart_surfaces(history, handle)
        .into_iter()
        .find(|(k, _)| *k == key)
        .map(|(_, s)| s)
        .ok_or_else(|| format!("unknown chart {name}: entries-per-day or digest-per-program"))?;
    AppWidget::new(
        TITLE,
        vec![Screen::new(key.clone(), say.say(&key), Arc::new(surface))],
    )
    .map_err(|e| e.to_string())
}

struct Args {
    chart: Option<String>,
    store: Option<PathBuf>,
    size: (u32, u32),
    help: bool,
}

fn parse(words: &[String]) -> Result<Args, String> {
    let mut out = Args {
        chart: None,
        store: None,
        size: (1200, 800),
        help: false,
    };
    let mut it = words.iter();
    while let Some(word) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match word.as_str() {
            "--store" => out.store = Some(PathBuf::from(value("--store")?)),
            "--chart" => out.chart = Some(value("--chart")?),
            "--size" => out.size = parse_size(&value("--size")?)?,
            "-h" | "--help" => out.help = true,
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(out)
}

/// The index file in the data folder of the user, whose folder is made.
fn default_store() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .ok_or("no data folder: give --store or set $XDG_DATA_HOME or $HOME")?;
    let dir = base.join("freesynergy/history");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir.join("index.db"))
}

/// Open the window. `words` are the words without the program name.
///
/// # Errors
/// The message for the person: bad words, a store that does not open.
pub fn run(words: &[String]) -> Result<(), String> {
    let args = parse(words)?;
    if args.help {
        println!("{HELP}");
        return Ok(());
    }
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let file = args.store.map_or_else(default_store, Ok)?;
    let store = runtime
        .block_on(TursoStore::open(&file))
        .map_err(|e| format!("cannot open the history index {}: {e}", file.display()))?;
    let objects: Arc<dyn UnloggedObjects> = Arc::new(store);
    let history: Arc<dyn History> = Arc::new(HistoryRepo::new(objects));
    let app = match &args.chart {
        Some(name) => chart_widget(&history, runtime.handle(), name)?,
        None => app_widget(&history, runtime.handle())?,
    };
    let (width, height) = args.size;
    let mut options = WindowOptions::default();
    options.width = width;
    options.height = height;
    launch(options, app).map_err(|e| e.to_string())
}
