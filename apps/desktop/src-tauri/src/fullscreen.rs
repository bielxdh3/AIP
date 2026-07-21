use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use tauri::{AppHandle, Manager};

use crate::overlays;

pub fn spawn_monitor(app: AppHandle, safe_mode: Arc<AtomicBool>) {
    thread::spawn(move || {
        let mut last_hidden = None;
        loop {
            if app.get_webview_window("main").is_none() {
                return;
            }
            let hidden = safe_mode.load(Ordering::SeqCst) || foreground_is_fullscreen();
            if last_hidden != Some(hidden) {
                overlays::set_visible(&app, !hidden);
                last_hidden = Some(hidden);
            }
            thread::sleep(Duration::from_millis(750));
        }
    });
}

#[cfg(not(target_os = "windows"))]
fn foreground_is_fullscreen() -> bool {
    false
}

#[cfg(target_os = "windows")]
fn foreground_is_fullscreen() -> bool {
    const MONITOR_DEFAULT_TO_NEAREST: u32 = 2;
    const TOLERANCE: i32 = 2;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct MonitorInfo {
        size: u32,
        monitor: Rect,
        _work: Rect,
        _flags: u32,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        #[link_name = "GetForegroundWindow"]
        fn get_foreground_window() -> isize;
        #[link_name = "GetWindowRect"]
        fn get_window_rect(window: isize, rect: *mut Rect) -> i32;
        #[link_name = "MonitorFromWindow"]
        fn monitor_from_window(window: isize, flags: u32) -> isize;
        #[link_name = "GetMonitorInfoW"]
        fn get_monitor_info(monitor: isize, info: *mut MonitorInfo) -> i32;
    }

    // This isolated probe is best-effort and remains subject to manual Windows validation.
    unsafe {
        let window = get_foreground_window();
        if window == 0 {
            return false;
        }
        let mut window_rect = Rect::default();
        if get_window_rect(window, &mut window_rect) == 0 {
            return false;
        }
        let monitor = monitor_from_window(window, MONITOR_DEFAULT_TO_NEAREST);
        if monitor == 0 {
            return false;
        }
        let mut monitor_info = MonitorInfo {
            size: std::mem::size_of::<MonitorInfo>() as u32,
            monitor: Rect::default(),
            _work: Rect::default(),
            _flags: 0,
        };
        if get_monitor_info(monitor, &mut monitor_info) == 0 {
            return false;
        }
        window_rect.left <= monitor_info.monitor.left + TOLERANCE
            && window_rect.top <= monitor_info.monitor.top + TOLERANCE
            && window_rect.right >= monitor_info.monitor.right - TOLERANCE
            && window_rect.bottom >= monitor_info.monitor.bottom - TOLERANCE
    }
}
