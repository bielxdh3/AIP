mod database;
mod domain;
mod fullscreen;
mod overlays;
mod protocol;
mod runtime;

use std::{
    io,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use database::Database;
use domain::AppSnapshot;
use runtime::RuntimeController;
use tauri::{AppHandle, Manager, State};

struct AppState {
    database: Option<Database>,
    runtime: RuntimeController,
    safe_mode: Arc<AtomicBool>,
}

#[tauri::command]
fn get_app_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot, &'static str> {
    snapshot(&state)
}

#[tauri::command]
fn set_safe_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<AppSnapshot, &'static str> {
    state
        .database
        .as_ref()
        .ok_or("operation_unavailable")?
        .set_safe_mode(enabled)
        .map_err(|_| "operation_failed")?;
    state.safe_mode.store(enabled, Ordering::SeqCst);
    if enabled {
        state.runtime.enter_safe_mode();
        overlays::set_visible(&app, false);
    } else {
        state.runtime.leave_safe_mode();
        overlays::set_visible(&app, true);
    }
    snapshot(&state)
}

#[tauri::command]
fn start_overlay_drag(
    app: AppHandle,
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<(), &'static str> {
    if state.safe_mode.load(Ordering::SeqCst) {
        return Err("operation_unavailable");
    }
    let label = overlays::window_label(&agent_id).ok_or("operation_unavailable")?;
    let window = app
        .get_webview_window(label)
        .ok_or("operation_unavailable")?;
    window.start_dragging().map_err(|_| "operation_failed")
}

fn snapshot(state: &AppState) -> Result<AppSnapshot, &'static str> {
    let Some(database) = state.database.as_ref() else {
        return Ok(AppSnapshot {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            safe_mode: true,
            database_ready: false,
            migration_version: 0,
            runtime: state.runtime.snapshot(),
            agents: Vec::new(),
        });
    };
    let stored = database.snapshot().map_err(|_| "operation_failed")?;
    Ok(AppSnapshot {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        safe_mode: stored.safe_mode,
        database_ready: true,
        migration_version: stored.migration_version,
        runtime: state.runtime.snapshot(),
        agents: stored.agents,
    })
}

fn runtime_source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../services/runtime/src")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let data_directory = app
                .path()
                .app_local_data_dir()
                .map_err(|_| io::Error::other("app_data_unavailable"))?;
            let database = Database::initialize(data_directory.join("database/aip.sqlite3"));
            let force_safe_mode =
                std::env::args_os().any(|argument| argument == std::ffi::OsStr::new("--safe-mode"));
            let (database, stored_safe_mode) = match database {
                Ok(database) => {
                    let stored = database
                        .snapshot()
                        .map_err(|_| io::Error::other("database_unavailable"))?;
                    let safe_mode = stored.safe_mode || force_safe_mode;
                    if safe_mode != stored.safe_mode {
                        database
                            .set_safe_mode(safe_mode)
                            .map_err(|_| io::Error::other("database_unavailable"))?;
                    }
                    (Some(database), safe_mode)
                }
                Err(_) => (None, true),
            };
            let safe_mode = Arc::new(AtomicBool::new(stored_safe_mode));
            let runtime = RuntimeController::new(runtime_source_root(), stored_safe_mode);

            app.manage(AppState {
                database: database.clone(),
                runtime: runtime.clone(),
                safe_mode: Arc::clone(&safe_mode),
            });
            if let Some(database) = database.as_ref() {
                overlays::create_windows(app, database, stored_safe_mode)?;
            }
            fullscreen::spawn_monitor(app.handle().clone(), safe_mode);
            if !stored_safe_mode {
                runtime.start();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_snapshot,
            set_safe_mode,
            start_overlay_drag
        ])
        .build(tauri::generate_context!())
        .expect("AIP desktop initialization failed");

    app.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            if let Some(state) = app_handle.try_state::<AppState>() {
                state.runtime.shutdown();
            }
        }
    });
}
