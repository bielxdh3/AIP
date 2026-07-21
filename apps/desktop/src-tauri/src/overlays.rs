use std::{error::Error, thread, time::Duration};

use tauri::{
    App, AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use crate::database::Database;

const OVERLAY_WIDTH: f64 = 180.0;
const OVERLAY_HEIGHT: f64 = 192.0;

pub fn window_label(agent_id: &str) -> Option<&'static str> {
    match agent_id {
        "agt_astra_provisional" => Some("agent-astra"),
        "agt_luma_provisional" => Some("agent-luma"),
        _ => None,
    }
}

pub fn create_windows(
    app: &App,
    database: &Database,
    safe_mode: bool,
) -> Result<(), Box<dyn Error>> {
    for agent in database.snapshot()?.agents {
        let Some(label) = window_label(&agent.id) else {
            continue;
        };
        let url = WebviewUrl::App(format!("index.html?agent={}", agent.id).into());
        let window = WebviewWindowBuilder::new(app, label, url)
            .title(format!("A.I.P. — {}", agent.name))
            .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
            .position(agent.position.x, agent.position.y)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .visible(!safe_mode)
            .build()?;

        track_position(&window, database.clone(), agent.id);
        spawn_geometry_hit_test(window);
    }
    Ok(())
}

pub fn set_visible(app: &AppHandle, visible: bool) {
    for label in ["agent-astra", "agent-luma"] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = if visible {
                window.show()
            } else {
                window.hide()
            };
        }
    }
}

fn track_position(window: &WebviewWindow, database: Database, agent_id: String) {
    let tracked_window = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Moved(position) = event {
            let scale = tracked_window.scale_factor().unwrap_or(1.0);
            let _ = database.update_position(
                &agent_id,
                f64::from(position.x) / scale,
                f64::from(position.y) / scale,
            );
        }
    });
}

fn spawn_geometry_hit_test(window: WebviewWindow) {
    thread::spawn(move || {
        let mut last_ignored = None;
        loop {
            let visible = match window.is_visible() {
                Ok(visible) => visible,
                Err(_) => return,
            };
            if !visible {
                last_ignored = None;
                thread::sleep(Duration::from_millis(80));
                continue;
            }

            let interactive = match (
                window.cursor_position(),
                window.outer_position(),
                window.scale_factor(),
            ) {
                (Ok(cursor), Ok(origin), Ok(scale)) => {
                    let local_x = (cursor.x - f64::from(origin.x)) / scale;
                    let local_y = (cursor.y - f64::from(origin.y)) / scale;
                    (18.0..=162.0).contains(&local_x) && (8.0..=184.0).contains(&local_y)
                }
                _ => true,
            };
            let ignored = !interactive;
            if last_ignored != Some(ignored) {
                let _ = window.set_ignore_cursor_events(ignored);
                last_ignored = Some(ignored);
            }
            thread::sleep(Duration::from_millis(32));
        }
    });
}
