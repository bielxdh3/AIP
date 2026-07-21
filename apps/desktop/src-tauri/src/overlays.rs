use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use serde::Deserialize;
use tauri::{
    App, AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use thiserror::Error;

use crate::database::Database;

const OVERLAY_WIDTH: f64 = 180.0;
const OVERLAY_HEIGHT: f64 = 192.0;
const MAX_INTERACTIVE_REGIONS: usize = 16;
const MAX_REGION_COORDINATE: f64 = 4096.0;
const OVERLAY_LABELS: [&str; 2] = ["agent-astra", "agent-luma"];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveRegion {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl InteractiveRegion {
    fn is_valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .into_iter()
            .all(f64::is_finite)
            && self.x.abs() <= MAX_REGION_COORDINATE
            && self.y.abs() <= MAX_REGION_COORDINATE
            && self.width > 0.0
            && self.height > 0.0
            && self.width <= MAX_REGION_COORDINATE
            && self.height <= MAX_REGION_COORDINATE
    }

    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OverlayInputError {
    #[error("unknown overlay window")]
    UnknownWindow,
    #[error("too many interactive regions")]
    TooManyRegions,
    #[error("invalid interactive region")]
    InvalidRegion,
}

#[derive(Clone, Default)]
pub struct OverlayInputState {
    regions: Arc<RwLock<HashMap<String, Vec<InteractiveRegion>>>>,
}

impl OverlayInputState {
    pub fn set_regions(
        &self,
        label: &str,
        regions: Vec<InteractiveRegion>,
    ) -> Result<(), OverlayInputError> {
        if !OVERLAY_LABELS.contains(&label) {
            return Err(OverlayInputError::UnknownWindow);
        }
        if regions.len() > MAX_INTERACTIVE_REGIONS {
            return Err(OverlayInputError::TooManyRegions);
        }
        if regions.iter().any(|region| !region.is_valid()) {
            return Err(OverlayInputError::InvalidRegion);
        }
        self.regions
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(label.to_string(), regions);
        Ok(())
    }

    fn regions(&self, label: &str) -> Vec<InteractiveRegion> {
        self.regions
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(label)
            .cloned()
            .unwrap_or_default()
    }

    pub fn clear_all(&self) {
        self.regions
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    fn remove_window(&self, label: &str) {
        self.regions
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(label);
    }
}

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
    input_state: OverlayInputState,
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
        spawn_interactive_region_hit_test(window, input_state.clone(), label);
    }
    Ok(())
}

pub fn set_visible(app: &AppHandle, visible: bool) {
    for label in OVERLAY_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            if !visible {
                let _ = window.set_ignore_cursor_events(true);
            }
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

fn spawn_interactive_region_hit_test(
    window: WebviewWindow,
    input_state: OverlayInputState,
    label: &'static str,
) {
    thread::spawn(move || {
        let mut last_ignored = None;
        loop {
            let visible = match window.is_visible() {
                Ok(visible) => visible,
                Err(_) => {
                    input_state.remove_window(label);
                    return;
                }
            };
            if !visible {
                if last_ignored != Some(true) {
                    let _ = window.set_ignore_cursor_events(true);
                    last_ignored = Some(true);
                }
                thread::sleep(Duration::from_millis(80));
                continue;
            }

            let interactive = match (
                window.cursor_position(),
                window.outer_position(),
                window.scale_factor(),
            ) {
                (Ok(cursor), Ok(origin), Ok(scale)) => logical_cursor_position(
                    cursor.x,
                    cursor.y,
                    f64::from(origin.x),
                    f64::from(origin.y),
                    scale,
                )
                .is_some_and(|(x, y)| {
                    point_is_interactive(true, &input_state.regions(label), x, y)
                }),
                _ => false,
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

fn logical_cursor_position(
    cursor_x: f64,
    cursor_y: f64,
    origin_x: f64,
    origin_y: f64,
    scale: f64,
) -> Option<(f64, f64)> {
    if ![cursor_x, cursor_y, origin_x, origin_y, scale]
        .into_iter()
        .all(f64::is_finite)
        || scale <= 0.0
    {
        return None;
    }
    Some(((cursor_x - origin_x) / scale, (cursor_y - origin_y) / scale))
}

fn point_is_interactive(visible: bool, regions: &[InteractiveRegion], x: f64, y: f64) -> bool {
    visible && x.is_finite() && y.is_finite() && regions.iter().any(|region| region.contains(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(x: f64, y: f64, width: f64, height: f64) -> InteractiveRegion {
        InteractiveRegion {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn interactive_regions_use_half_open_boundaries() {
        let regions = [region(10.0, 20.0, 30.0, 40.0)];
        assert!(point_is_interactive(true, &regions, 10.0, 20.0));
        assert!(point_is_interactive(true, &regions, 39.999, 59.999));
        assert!(!point_is_interactive(true, &regions, 40.0, 60.0));
        assert!(!point_is_interactive(true, &regions, 9.999, 20.0));
    }

    #[test]
    fn multiple_and_empty_regions_are_deterministic() {
        let regions = [region(1.0, 1.0, 4.0, 4.0), region(20.0, 20.0, 5.0, 5.0)];
        assert!(point_is_interactive(true, &regions, 22.0, 22.0));
        assert!(!point_is_interactive(true, &regions, 10.0, 10.0));
        assert!(!point_is_interactive(true, &[], 2.0, 2.0));
    }

    #[test]
    fn hidden_overlay_never_retains_an_interactive_area() {
        let state = OverlayInputState::default();
        state
            .set_regions("agent-astra", vec![region(0.0, 0.0, 100.0, 100.0)])
            .expect("visible region should be accepted");
        state.clear_all();
        assert!(state.regions("agent-astra").is_empty());
        assert!(!point_is_interactive(
            false,
            &[region(0.0, 0.0, 100.0, 100.0)],
            50.0,
            50.0
        ));
    }

    #[test]
    fn scale_conversion_supports_common_windows_values() {
        assert_eq!(
            logical_cursor_position(100.0, 80.0, 0.0, 0.0, 1.0),
            Some((100.0, 80.0))
        );
        assert_eq!(
            logical_cursor_position(125.0, 100.0, 0.0, 0.0, 1.25),
            Some((100.0, 80.0))
        );
        assert_eq!(
            logical_cursor_position(150.0, 120.0, 0.0, 0.0, 1.5),
            Some((100.0, 80.0))
        );
    }

    #[test]
    fn malformed_geometry_is_rejected() {
        let state = OverlayInputState::default();
        for invalid in [
            region(0.0, 0.0, -1.0, 1.0),
            region(0.0, 0.0, 1.0, -1.0),
            region(0.0, 0.0, 1.0, 0.0),
            region(f64::NAN, 0.0, 1.0, 1.0),
            region(0.0, f64::INFINITY, 1.0, 1.0),
            region(0.0, 0.0, MAX_REGION_COORDINATE + 1.0, 1.0),
        ] {
            assert_eq!(
                state.set_regions("agent-astra", vec![invalid]),
                Err(OverlayInputError::InvalidRegion)
            );
        }
    }

    #[test]
    fn unknown_windows_and_excessive_region_counts_are_rejected() {
        let state = OverlayInputState::default();
        assert_eq!(window_label("malformed-agent"), None);
        assert_eq!(
            state.set_regions("main", vec![region(0.0, 0.0, 1.0, 1.0)]),
            Err(OverlayInputError::UnknownWindow)
        );
        assert_eq!(
            state.set_regions(
                "agent-astra",
                vec![region(0.0, 0.0, 1.0, 1.0); MAX_INTERACTIVE_REGIONS + 1]
            ),
            Err(OverlayInputError::TooManyRegions)
        );
    }

    #[test]
    fn agent_regions_remain_independent() {
        let state = OverlayInputState::default();
        state
            .set_regions("agent-astra", vec![region(1.0, 1.0, 2.0, 2.0)])
            .expect("Astra regions should be accepted");
        state
            .set_regions("agent-luma", vec![region(20.0, 20.0, 2.0, 2.0)])
            .expect("Luma regions should be accepted");
        assert_ne!(state.regions("agent-astra"), state.regions("agent-luma"));
    }

    #[test]
    fn visible_regions_can_be_added_removed_and_invalidated() {
        let state = OverlayInputState::default();
        let sprite = region(10.0, 10.0, 64.0, 64.0);
        let thought = region(80.0, 2.0, 24.0, 18.0);
        state
            .set_regions("agent-astra", vec![sprite, thought])
            .expect("visible regions should be accepted");
        assert_eq!(state.regions("agent-astra"), vec![sprite, thought]);

        state
            .set_regions("agent-astra", vec![sprite])
            .expect("hidden thought region should be removed");
        assert_eq!(state.regions("agent-astra"), vec![sprite]);

        state.remove_window("agent-astra");
        assert!(state.regions("agent-astra").is_empty());
    }
}
