use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, RwLock},
};

use serde::Deserialize;
use tauri::{
    App, AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use thiserror::Error;

use crate::{
    database::Database,
    native_overlay_region::{install_with, reset_with, PhysicalRegion, PlatformRegionInstaller},
};

const OVERLAY_WIDTH: f64 = 180.0;
const OVERLAY_HEIGHT: f64 = 192.0;
const MAX_INTERACTIVE_REGIONS: usize = 256;
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
            && self.x >= 0.0
            && self.y >= 0.0
            && self.x <= MAX_REGION_COORDINATE
            && self.y <= MAX_REGION_COORDINATE
            && self.width > 0.0
            && self.height > 0.0
            && self.width <= MAX_REGION_COORDINATE
            && self.height <= MAX_REGION_COORDINATE
            && self.x + self.width <= MAX_REGION_COORDINATE
            && self.y + self.height <= MAX_REGION_COORDINATE
    }

    #[cfg(test)]
    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    fn to_physical(self, scale: f64) -> Option<PhysicalRegion> {
        if !scale.is_finite() || scale <= 0.0 || !self.is_valid() {
            return None;
        }
        let physical = PhysicalRegion {
            left: (self.x * scale).floor() as i32,
            top: (self.y * scale).floor() as i32,
            right: ((self.x + self.width) * scale).ceil() as i32,
            bottom: ((self.y + self.height) * scale).ceil() as i32,
        };
        physical.is_valid().then_some(physical)
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
    #[error("native overlay region failed")]
    NativeRegionFailed,
}

#[derive(Clone, Default)]
pub struct OverlayInputState {
    regions: Arc<RwLock<HashMap<String, Vec<InteractiveRegion>>>>,
}

impl OverlayInputState {
    fn validate(
        &self,
        label: &str,
        regions: &[InteractiveRegion],
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
        Ok(())
    }

    fn replace(&self, label: &str, regions: Vec<InteractiveRegion>) {
        self.regions
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(label.to_string(), regions);
    }

    #[cfg(test)]
    fn regions(&self, label: &str) -> Vec<InteractiveRegion> {
        self.regions
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(label)
            .cloned()
            .unwrap_or_default()
    }

    pub fn clear_all(&self) {
        for regions in self
            .regions
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values_mut()
        {
            regions.clear();
        }
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
            .visible(false)
            .build()?;

        if install_regions(&window, label, &input_state, Vec::new()).is_err() {
            continue;
        }
        track_lifecycle(
            &window,
            database.clone(),
            agent.id,
            input_state.clone(),
            label,
        );
        if !safe_mode {
            window.show()?;
        }
    }
    Ok(())
}

pub fn set_visible(app: &AppHandle, visible: bool) {
    for label in OVERLAY_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            let _ = if visible {
                window.show()
            } else {
                window.hide()
            };
        }
    }
}

pub fn clear_native_regions(app: &AppHandle, input_state: &OverlayInputState) {
    input_state.clear_all();
    for label in OVERLAY_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            let _ = apply_native_regions(&window, &[]);
        }
    }
}

pub fn reset_native_regions(app: &AppHandle) {
    for label in OVERLAY_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            #[cfg(windows)]
            if let Ok(hwnd) = window.hwnd() {
                let _ = reset_with(&PlatformRegionInstaller, hwnd.0 as isize);
            }
        }
    }
}

pub fn install_regions(
    window: &WebviewWindow,
    label: &str,
    input_state: &OverlayInputState,
    regions: Vec<InteractiveRegion>,
) -> Result<(), OverlayInputError> {
    input_state.validate(label, &regions)?;
    apply_native_regions(window, &regions)?;
    input_state.replace(label, regions);
    Ok(())
}

fn apply_native_regions(
    window: &WebviewWindow,
    regions: &[InteractiveRegion],
) -> Result<(), OverlayInputError> {
    let scale = window
        .scale_factor()
        .map_err(|_| OverlayInputError::NativeRegionFailed)?;
    let physical = physical_regions(regions, scale)?;
    #[cfg(windows)]
    let native_window = window
        .hwnd()
        .map_err(|_| OverlayInputError::NativeRegionFailed)?
        .0 as isize;
    #[cfg(not(windows))]
    let native_window = 0;

    install_with(&PlatformRegionInstaller, native_window, &physical)
        .map_err(|_| OverlayInputError::NativeRegionFailed)
}

fn physical_regions(
    regions: &[InteractiveRegion],
    scale: f64,
) -> Result<Vec<PhysicalRegion>, OverlayInputError> {
    regions
        .iter()
        .copied()
        .map(|region| {
            region
                .to_physical(scale)
                .ok_or(OverlayInputError::InvalidRegion)
        })
        .collect()
}

fn track_lifecycle(
    window: &WebviewWindow,
    database: Database,
    agent_id: String,
    input_state: OverlayInputState,
    label: &'static str,
) {
    let tracked_window = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(position) => {
            let scale = tracked_window.scale_factor().unwrap_or(1.0);
            let _ = database.update_position(
                &agent_id,
                f64::from(position.x) / scale,
                f64::from(position.y) / scale,
            );
        }
        WindowEvent::Destroyed => input_state.remove_window(label),
        _ => {}
    });
}

#[cfg(test)]
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
    fn native_shape_distinguishes_interactive_and_pass_through_points() {
        let painted = [region(10.0, 20.0, 30.0, 40.0)];
        assert!(point_is_interactive(true, &painted, 10.0, 20.0));
        assert!(!point_is_interactive(true, &painted, 9.0, 20.0));
        assert!(!point_is_interactive(true, &[], 10.0, 20.0));
        assert!(!point_is_interactive(false, &painted, 10.0, 20.0));
    }

    #[test]
    fn physical_conversion_applies_scale_once_with_outward_rounding() {
        let logical = [region(1.2, 2.2, 3.2, 4.2)];
        for (scale, expected) in [
            (1.0, (1, 2, 5, 7)),
            (1.25, (1, 2, 6, 8)),
            (1.5, (1, 3, 7, 10)),
            (1.75, (2, 3, 8, 12)),
            (2.0, (2, 4, 9, 13)),
        ] {
            let converted = physical_regions(&logical, scale).expect("scale should be valid");
            assert_eq!(
                converted,
                vec![PhysicalRegion {
                    left: expected.0,
                    top: expected.1,
                    right: expected.2,
                    bottom: expected.3,
                }]
            );
        }
    }

    #[test]
    fn malformed_snapshots_are_rejected() {
        let state = OverlayInputState::default();
        for invalid in [
            region(-1.0, 0.0, 1.0, 1.0),
            region(0.0, 0.0, -1.0, 1.0),
            region(0.0, 0.0, 1.0, 0.0),
            region(f64::NAN, 0.0, 1.0, 1.0),
            region(0.0, f64::INFINITY, 1.0, 1.0),
            region(0.0, 0.0, MAX_REGION_COORDINATE + 1.0, 1.0),
            region(MAX_REGION_COORDINATE, 0.0, 1.0, 1.0),
        ] {
            assert_eq!(
                state.validate("agent-astra", &[invalid]),
                Err(OverlayInputError::InvalidRegion)
            );
        }
        assert_eq!(
            state.validate("unknown", &[region(0.0, 0.0, 1.0, 1.0)]),
            Err(OverlayInputError::UnknownWindow)
        );
        assert_eq!(
            state.validate(
                "agent-astra",
                &vec![region(0.0, 0.0, 1.0, 1.0); MAX_INTERACTIVE_REGIONS + 1],
            ),
            Err(OverlayInputError::TooManyRegions)
        );
    }

    #[test]
    fn overlay_state_is_isolated_and_safe_mode_clears_shapes() {
        let state = OverlayInputState::default();
        state.replace("agent-astra", vec![region(1.0, 1.0, 2.0, 2.0)]);
        state.replace("agent-luma", vec![region(20.0, 20.0, 2.0, 2.0)]);
        assert_ne!(state.regions("agent-astra"), state.regions("agent-luma"));
        state.clear_all();
        assert!(state.regions("agent-astra").is_empty());
        assert!(state.regions("agent-luma").is_empty());
        state.remove_window("agent-astra");
        assert!(state.regions("agent-astra").is_empty());
    }
}
