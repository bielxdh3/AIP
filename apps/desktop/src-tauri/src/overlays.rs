use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::{Arc, RwLock},
};

use serde::Deserialize;
use tauri::{
    window::Color, App, AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
use thiserror::Error;

use crate::{
    database::Database,
    native_overlay_region::{install_with, reset_with, PhysicalRegion, PlatformRegionInstaller},
};

const OVERLAY_WIDTH: f64 = 180.0;
const OVERLAY_HEIGHT: f64 = 192.0;
const BUBBLE_WIDTH: f64 = 380.0;
const BUBBLE_HEIGHT: f64 = 360.0;
const MAX_INTERACTIVE_REGIONS: usize = 256;
const MAX_REGION_COORDINATE: f64 = 4096.0;
const AGENT_IDS: [&str; 2] = ["agt_astra_provisional", "agt_luma_provisional"];
const OVERLAY_LABELS: [&str; 4] = [
    "agent-astra",
    "agent-luma",
    "agent-astra-bubble",
    "agent-luma-bubble",
];
const AGENT_LABELS: [&str; 2] = ["agent-astra", "agent-luma"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowSurfaceContract {
    transparent: bool,
    background: [u8; 4],
    decorations: bool,
    shadow: bool,
}

const fn overlay_surface_contract() -> WindowSurfaceContract {
    WindowSurfaceContract {
        transparent: true,
        background: [0, 0, 0, 0],
        decorations: false,
        shadow: false,
    }
}

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
    visible_bubbles: Arc<RwLock<HashSet<String>>>,
}

impl OverlayInputState {
    fn validate(
        &self,
        label: &str,
        regions: &[InteractiveRegion],
    ) -> Result<(), OverlayInputError> {
        if ownership_for_window_label(label).is_none() {
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

    fn set_bubble_visible(&self, agent_id: &str, visible: bool) {
        let mut bubbles = self
            .visible_bubbles
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if visible {
            bubbles.insert(agent_id.to_string());
        } else {
            bubbles.remove(agent_id);
        }
    }

    fn bubble_visible(&self, agent_id: &str) -> bool {
        self.visible_bubbles
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(agent_id)
    }

    fn remove_window(&self, label: &str) {
        self.regions
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(label);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OverlayOwnership {
    agent_id: &'static str,
    agent_label: &'static str,
    bubble_label: &'static str,
}

fn ownership_for_agent(agent_id: &str) -> Option<OverlayOwnership> {
    match agent_id {
        "agt_astra_provisional" => Some(OverlayOwnership {
            agent_id: "agt_astra_provisional",
            agent_label: "agent-astra",
            bubble_label: "agent-astra-bubble",
        }),
        "agt_luma_provisional" => Some(OverlayOwnership {
            agent_id: "agt_luma_provisional",
            agent_label: "agent-luma",
            bubble_label: "agent-luma-bubble",
        }),
        _ => None,
    }
}

fn ownership_for_window_label(label: &str) -> Option<OverlayOwnership> {
    match label {
        "agent-astra" | "agent-astra-bubble" => ownership_for_agent("agt_astra_provisional"),
        "agent-luma" | "agent-luma-bubble" => ownership_for_agent("agt_luma_provisional"),
        _ => None,
    }
}

pub fn window_label(agent_id: &str) -> Option<&'static str> {
    ownership_for_agent(agent_id).map(|ownership| ownership.agent_label)
}

pub fn bubble_window_label(agent_id: &str) -> Option<&'static str> {
    ownership_for_agent(agent_id).map(|ownership| ownership.bubble_label)
}

pub fn create_windows(
    app: &App,
    database: &Database,
    safe_mode: bool,
    input_state: OverlayInputState,
) -> Result<(), Box<dyn Error>> {
    let surface = overlay_surface_contract();
    for agent in database.snapshot()?.agents {
        let Some(ownership) = ownership_for_agent(&agent.id) else {
            continue;
        };
        let label = ownership.agent_label;
        let url = WebviewUrl::App(format!("index.html?agent={}", agent.id).into());
        let window = WebviewWindowBuilder::new(app, label, url)
            .title(format!("A.I.P. — {}", agent.name))
            .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
            .position(agent.position.x, agent.position.y)
            .transparent(surface.transparent)
            .background_color(Color(
                surface.background[0],
                surface.background[1],
                surface.background[2],
                surface.background[3],
            ))
            .decorations(surface.decorations)
            .shadow(surface.shadow)
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
            agent.id.clone(),
            input_state.clone(),
            label,
        );
        if !safe_mode {
            window.show()?;
        }

        let bubble_label = ownership.bubble_label;
        let bubble_url = WebviewUrl::App(format!("index.html?bubble={}", agent.id).into());
        let bubble = WebviewWindowBuilder::new(app, bubble_label, bubble_url)
            .title(format!("A.I.P. â€” conversa com {}", agent.name))
            .inner_size(BUBBLE_WIDTH, BUBBLE_HEIGHT)
            .transparent(surface.transparent)
            .background_color(Color(
                surface.background[0],
                surface.background[1],
                surface.background[2],
                surface.background[3],
            ))
            .decorations(surface.decorations)
            .shadow(surface.shadow)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .visible(false)
            .build()?;
        install_regions(&bubble, bubble_label, &input_state, Vec::new())?;
        let bubble_state = input_state.clone();
        let bubble_agent_id = ownership.agent_id;
        bubble.on_window_event(move |event| {
            if matches!(event, WindowEvent::Destroyed) {
                bubble_state.remove_window(bubble_label);
                bubble_state.set_bubble_visible(bubble_agent_id, false);
            }
        });
    }
    Ok(())
}

pub fn set_visible(app: &AppHandle, input_state: &OverlayInputState, visible: bool) {
    for label in AGENT_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            if visible {
                // Windows can leave a borderless overlay minimized after it is hidden.
                // Restore it before showing so every tracked agent follows safe-mode changes.
                let _ = window.unminimize();
                let _ = window.show();
            } else {
                let _ = window.hide();
            }
        }
    }
    for agent_id in AGENT_IDS {
        if let Some(label) = bubble_window_label(agent_id) {
            if let Some(window) = app.get_webview_window(label) {
                if visible && input_state.bubble_visible(agent_id) {
                    let _ = window.unminimize();
                    let _ = window.show();
                } else {
                    let _ = window.hide();
                }
            }
        }
    }
}

pub fn set_bubble_visible(
    app: &AppHandle,
    input_state: &OverlayInputState,
    agent_id: &str,
    visible: bool,
) -> Result<(), OverlayInputError> {
    let ownership = ownership_for_agent(agent_id).ok_or(OverlayInputError::UnknownWindow)?;
    let bubble_label = ownership.bubble_label;
    let bubble = app
        .get_webview_window(bubble_label)
        .ok_or(OverlayInputError::UnknownWindow)?;
    if visible {
        install_regions(
            &bubble,
            bubble_label,
            input_state,
            vec![InteractiveRegion {
                x: 8.0,
                y: 8.0,
                width: BUBBLE_WIDTH - 16.0,
                height: 108.0,
            }],
        )?;
        if let Err(error) = position_bubble(app, agent_id) {
            let _ = install_regions(&bubble, bubble_label, input_state, Vec::new());
            return Err(error);
        }
        input_state.set_bubble_visible(agent_id, true);
        bubble
            .unminimize()
            .map_err(|_| OverlayInputError::NativeRegionFailed)?;
        if bubble.show().is_err() {
            input_state.set_bubble_visible(agent_id, false);
            let _ = install_regions(&bubble, bubble_label, input_state, Vec::new());
            return Err(OverlayInputError::NativeRegionFailed);
        }
        let _ = bubble.set_focus();
    } else {
        input_state.set_bubble_visible(agent_id, false);
        install_regions(&bubble, bubble_label, input_state, Vec::new())?;
        bubble
            .hide()
            .map_err(|_| OverlayInputError::NativeRegionFailed)?;
    }
    Ok(())
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

pub fn close_all(app: &AppHandle, input_state: &OverlayInputState) {
    clear_native_regions(app, input_state);
    for label in OVERLAY_LABELS {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.destroy();
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
    let app = window.app_handle().clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(position) => {
            let scale = tracked_window.scale_factor().unwrap_or(1.0);
            let _ = database.update_position(
                &agent_id,
                f64::from(position.x) / scale,
                f64::from(position.y) / scale,
            );
            if input_state.bubble_visible(&agent_id) {
                let _ = position_bubble(&app, &agent_id);
            }
        }
        WindowEvent::Destroyed => input_state.remove_window(label),
        _ => {}
    });
}

fn position_bubble(app: &AppHandle, agent_id: &str) -> Result<(), OverlayInputError> {
    let ownership = ownership_for_agent(agent_id).ok_or(OverlayInputError::UnknownWindow)?;
    let agent_label = ownership.agent_label;
    let bubble_label = ownership.bubble_label;
    let agent = app
        .get_webview_window(agent_label)
        .ok_or(OverlayInputError::UnknownWindow)?;
    let bubble = app
        .get_webview_window(bubble_label)
        .ok_or(OverlayInputError::UnknownWindow)?;
    let agent_position = agent
        .outer_position()
        .map_err(|_| OverlayInputError::NativeRegionFailed)?;
    let scale = agent
        .scale_factor()
        .map_err(|_| OverlayInputError::NativeRegionFailed)?;
    let monitor = agent
        .current_monitor()
        .map_err(|_| OverlayInputError::NativeRegionFailed)?
        .ok_or(OverlayInputError::NativeRegionFailed)?;
    let bubble_width = (BUBBLE_WIDTH * scale).round() as i32;
    let bubble_height = (BUBBLE_HEIGHT * scale).round() as i32;
    let agent_width = (OVERLAY_WIDTH * scale).round() as i32;
    let work_area = monitor.work_area();
    let position = bubble_position(
        agent_position,
        agent_width,
        bubble_width,
        bubble_height,
        work_area.position,
        work_area.size,
    );
    bubble
        .set_position(position)
        .map_err(|_| OverlayInputError::NativeRegionFailed)
}

fn bubble_position(
    agent_position: tauri::PhysicalPosition<i32>,
    agent_width: i32,
    bubble_width: i32,
    bubble_height: i32,
    work_area_position: tauri::PhysicalPosition<i32>,
    work_area_size: tauri::PhysicalSize<u32>,
) -> tauri::PhysicalPosition<i32> {
    let left = i64::from(work_area_position.x);
    let top = i64::from(work_area_position.y);
    let right = left + i64::from(work_area_size.width);
    let bottom = top + i64::from(work_area_size.height);
    let bubble_width = i64::from(bubble_width);
    let bubble_height = i64::from(bubble_height);
    let preferred_x = i64::from(agent_position.x) + i64::from(agent_width);
    let candidate_x = if preferred_x + bubble_width <= right {
        preferred_x
    } else {
        i64::from(agent_position.x) - bubble_width
    };
    let x = candidate_x.clamp(left, (right - bubble_width).max(left));
    let y = i64::from(agent_position.y).clamp(top, (bottom - bubble_height).max(top));
    tauri::PhysicalPosition::new(x as i32, y as i32)
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
    fn native_shape_keeps_transparent_holes_pass_through() {
        let painted = [
            region(0.0, 0.0, 4.0, 1.0),
            region(0.0, 1.0, 1.0, 1.0),
            region(3.0, 1.0, 1.0, 1.0),
            region(0.0, 2.0, 4.0, 1.0),
        ];
        assert!(point_is_interactive(true, &painted, 0.5, 0.5));
        assert!(point_is_interactive(true, &painted, 3.5, 1.5));
        assert!(!point_is_interactive(true, &painted, 1.5, 1.5));
        assert!(!point_is_interactive(true, &painted, 2.5, 1.5));
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

    #[test]
    fn simultaneous_bubble_visibility_is_independent() {
        let state = OverlayInputState::default();
        state.set_bubble_visible("agt_astra_provisional", true);
        state.set_bubble_visible("agt_luma_provisional", true);
        assert!(state.bubble_visible("agt_astra_provisional"));
        assert!(state.bubble_visible("agt_luma_provisional"));
        state.set_bubble_visible("agt_astra_provisional", false);
        assert!(!state.bubble_visible("agt_astra_provisional"));
        assert!(state.bubble_visible("agt_luma_provisional"));
    }

    #[test]
    fn bubble_position_stays_inside_work_area_and_flips_left_at_the_edge() {
        let work_area_position = tauri::PhysicalPosition::new(100, 40);
        let work_area_size = tauri::PhysicalSize::new(800, 600);
        let right = bubble_position(
            tauri::PhysicalPosition::new(700, 500),
            180,
            380,
            360,
            work_area_position,
            work_area_size,
        );
        assert_eq!(right, tauri::PhysicalPosition::new(320, 280));

        let top_left = bubble_position(
            tauri::PhysicalPosition::new(100, 40),
            180,
            380,
            360,
            work_area_position,
            work_area_size,
        );
        assert_eq!(top_left, tauri::PhysicalPosition::new(280, 40));
    }

    #[test]
    fn ownership_keeps_each_bubble_bound_to_its_agent_id() {
        let astra = ownership_for_agent("agt_astra_provisional").unwrap();
        let luma = ownership_for_window_label("agent-luma-bubble").unwrap();
        assert_eq!(astra.agent_label, "agent-astra");
        assert_eq!(astra.bubble_label, "agent-astra-bubble");
        assert_eq!(luma.agent_id, "agt_luma_provisional");
        assert_eq!(ownership_for_agent("unknown"), None);
    }

    #[test]
    fn transparent_surface_contract_has_no_window_chrome_or_shadow() {
        assert_eq!(
            overlay_surface_contract(),
            WindowSurfaceContract {
                transparent: true,
                background: [0, 0, 0, 0],
                decorations: false,
                shadow: false,
            }
        );
    }

    #[test]
    fn bubble_position_clamps_negative_and_narrow_monitor_coordinates() {
        let position = bubble_position(
            tauri::PhysicalPosition::new(-300, -100),
            180,
            380,
            360,
            tauri::PhysicalPosition::new(-400, -200),
            tauri::PhysicalSize::new(300, 240),
        );
        assert_eq!(position, tauri::PhysicalPosition::new(-400, -200));
    }
}
