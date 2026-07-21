use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalRegion {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PhysicalRegion {
    pub fn is_valid(self) -> bool {
        self.left >= 0 && self.top >= 0 && self.right > self.left && self.bottom > self.top
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NativeRegionError {
    #[error("invalid native window handle")]
    InvalidWindow,
    #[error("invalid physical region")]
    InvalidRegion,
    #[error("native region allocation failed")]
    AllocationFailed,
    #[error("native region combination failed")]
    CombinationFailed,
    #[error("native region installation failed")]
    InstallationFailed,
    #[cfg(not(windows))]
    #[error("native overlay regions are unsupported")]
    Unsupported,
}

pub trait NativeRegionInstaller {
    fn install(
        &self,
        native_window: isize,
        regions: &[PhysicalRegion],
    ) -> Result<(), NativeRegionError>;

    fn reset(&self, native_window: isize) -> Result<(), NativeRegionError>;
}

pub struct PlatformRegionInstaller;

#[cfg(windows)]
impl NativeRegionInstaller for PlatformRegionInstaller {
    fn install(
        &self,
        native_window: isize,
        regions: &[PhysicalRegion],
    ) -> Result<(), NativeRegionError> {
        use windows_sys::Win32::{
            Foundation::HWND,
            Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, ERROR, RGN_OR},
        };

        if native_window == 0 {
            return Err(NativeRegionError::InvalidWindow);
        }
        if regions.iter().any(|region| !region.is_valid()) {
            return Err(NativeRegionError::InvalidRegion);
        }

        // SAFETY: every GDI handle created here is checked. Temporary handles are deleted in
        // this function. After successful SetWindowRgn, Windows owns the combined region.
        unsafe {
            let combined = CreateRectRgn(0, 0, 0, 0);
            if combined.is_null() {
                return Err(NativeRegionError::AllocationFailed);
            }

            for region in regions {
                let part = CreateRectRgn(region.left, region.top, region.right, region.bottom);
                if part.is_null() {
                    let _ = DeleteObject(combined);
                    return Err(NativeRegionError::AllocationFailed);
                }
                let combination = CombineRgn(combined, combined, part, RGN_OR);
                let _ = DeleteObject(part);
                if combination == ERROR {
                    let _ = DeleteObject(combined);
                    return Err(NativeRegionError::CombinationFailed);
                }
            }

            if SetWindowRgn(native_window as HWND, combined, 1) == 0 {
                let _ = DeleteObject(combined);
                return Err(NativeRegionError::InstallationFailed);
            }
        }
        Ok(())
    }

    fn reset(&self, native_window: isize) -> Result<(), NativeRegionError> {
        use std::ptr::null_mut;
        use windows_sys::Win32::{Foundation::HWND, Graphics::Gdi::SetWindowRgn};

        if native_window == 0 {
            return Err(NativeRegionError::InvalidWindow);
        }
        // SAFETY: a null region asks Windows to restore the full rectangular window shape.
        if unsafe { SetWindowRgn(native_window as HWND, null_mut(), 1) } == 0 {
            return Err(NativeRegionError::InstallationFailed);
        }
        Ok(())
    }
}

#[cfg(not(windows))]
impl NativeRegionInstaller for PlatformRegionInstaller {
    fn install(
        &self,
        _native_window: isize,
        _regions: &[PhysicalRegion],
    ) -> Result<(), NativeRegionError> {
        Err(NativeRegionError::Unsupported)
    }

    fn reset(&self, _native_window: isize) -> Result<(), NativeRegionError> {
        Err(NativeRegionError::Unsupported)
    }
}

pub fn install_with<I: NativeRegionInstaller>(
    installer: &I,
    native_window: isize,
    regions: &[PhysicalRegion],
) -> Result<(), NativeRegionError> {
    installer.install(native_window, regions)
}

pub fn reset_with<I: NativeRegionInstaller>(
    installer: &I,
    native_window: isize,
) -> Result<(), NativeRegionError> {
    installer.reset(native_window)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct RecordingInstaller {
        installs: Mutex<Vec<Vec<PhysicalRegion>>>,
        resets: Mutex<usize>,
        fail: bool,
    }

    impl NativeRegionInstaller for RecordingInstaller {
        fn install(
            &self,
            _native_window: isize,
            regions: &[PhysicalRegion],
        ) -> Result<(), NativeRegionError> {
            if self.fail {
                return Err(NativeRegionError::InstallationFailed);
            }
            self.installs
                .lock()
                .expect("recording installer lock should remain valid")
                .push(regions.to_vec());
            Ok(())
        }

        fn reset(&self, _native_window: isize) -> Result<(), NativeRegionError> {
            *self
                .resets
                .lock()
                .expect("recording installer lock should remain valid") += 1;
            Ok(())
        }
    }

    #[test]
    fn native_adapter_maps_empty_and_interactive_shapes() {
        let installer = RecordingInstaller::default();
        install_with(&installer, 1, &[]).expect("empty pass-through shape should install");
        let painted = PhysicalRegion {
            left: 20,
            top: 10,
            right: 40,
            bottom: 30,
        };
        install_with(&installer, 1, &[painted]).expect("interactive painted shape should install");
        assert_eq!(
            *installer
                .installs
                .lock()
                .expect("recording installer lock should remain valid"),
            vec![vec![], vec![painted]]
        );
    }

    #[test]
    fn native_adapter_propagates_installation_failure() {
        let installer = RecordingInstaller {
            installs: Mutex::new(Vec::new()),
            resets: Mutex::new(0),
            fail: true,
        };
        assert_eq!(
            install_with(&installer, 1, &[]),
            Err(NativeRegionError::InstallationFailed)
        );
    }

    #[test]
    fn native_adapter_releases_region_state_on_teardown() {
        let installer = RecordingInstaller::default();
        reset_with(&installer, 1).expect("native shape should reset");
        assert_eq!(
            *installer
                .resets
                .lock()
                .expect("recording installer lock should remain valid"),
            1
        );
    }
}
