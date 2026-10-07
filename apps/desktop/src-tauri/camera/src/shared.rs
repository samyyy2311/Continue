// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! The newest frame, passed from the app to the camera through shared memory. There are two
//! slots: the app fills the one not being read and then points the camera at it.

use std::sync::atomic::{AtomicU32, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL, INVALID_HANDLE_VALUE};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
    MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
};

pub const WIDTH: usize = 1280;
pub const HEIGHT: usize = 720;
/// NV12: a full-size brightness plane, then colour at half size both ways.
pub const FRAME_BYTES: usize = WIDTH * HEIGHT * 3 / 2;

const HEADER_BYTES: usize = 64;
const TOTAL_BYTES: usize = HEADER_BYTES + 2 * FRAME_BYTES;

/// The camera runs inside Windows' camera service, which can only share through the global
/// namespace. Local is for running both ends in one session, as the tests do.
const NAMES: [PCWSTR; 2] = [w!("Global\\ContinueCamera"), w!("Local\\ContinueCamera")];

/// Full access for the system, the camera service's account and whoever is signed in, writable
/// from a normal app.
const ACCESS: PCWSTR = w!("D:(A;;GA;;;SY)(A;;GA;;;LS)(A;;GA;;;IU)S:(ML;;NW;;;LW)");

#[repr(C)]
struct Header {
    /// The slot holding the newest frame.
    latest: AtomicU32,
    /// How many frames have been written; none yet means there's nothing to show.
    written: AtomicU32,
}

pub struct SharedFrame {
    mapping: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
}

// The view is plain memory, and every access to it goes through the header's atomics.
unsafe impl Send for SharedFrame {}
unsafe impl Sync for SharedFrame {}

impl SharedFrame {
    /// Made by the camera when an app starts using it.
    pub fn create() -> Option<Self> {
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                ACCESS,
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
            .ok()?;
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: false.into(),
        };
        let mapping = NAMES.iter().find_map(|name| unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                Some(&attributes),
                PAGE_READWRITE,
                0,
                TOTAL_BYTES as u32,
                *name,
            )
            .ok()
        });
        unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
        Self::map(mapping?)
    }

    /// Opened by the app; None until something is using the camera.
    pub fn open() -> Option<Self> {
        let mapping = NAMES.iter().find_map(|name| unsafe {
            OpenFileMappingW(FILE_MAP_ALL_ACCESS.0, false, *name).ok()
        })?;
        Self::map(mapping)
    }

    fn map(mapping: HANDLE) -> Option<Self> {
        let view = unsafe { MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, TOTAL_BYTES) };
        if view.Value.is_null() {
            unsafe {
                let _ = CloseHandle(mapping);
            }
            return None;
        }
        Some(Self { mapping, view })
    }

    fn header(&self) -> &Header {
        unsafe { &*(self.view.Value as *const Header) }
    }

    fn slot(&self, index: u32) -> *mut u8 {
        let offset = HEADER_BYTES + index as usize * FRAME_BYTES;
        unsafe { (self.view.Value as *mut u8).add(offset) }
    }

    /// `frame` is a whole NV12 frame of WIDTH by HEIGHT.
    pub fn write(&self, frame: &[u8]) {
        assert_eq!(frame.len(), FRAME_BYTES);
        let header = self.header();
        let spare = 1 - header.latest.load(Ordering::Acquire);
        unsafe { std::ptr::copy_nonoverlapping(frame.as_ptr(), self.slot(spare), FRAME_BYTES) };
        header.latest.store(spare, Ordering::Release);
        header.written.fetch_add(1, Ordering::Release);
    }

    /// Copies the newest frame into `into`; false while the app hasn't written one.
    pub fn read(&self, into: &mut [u8]) -> bool {
        let header = self.header();
        if header.written.load(Ordering::Acquire) == 0 {
            return false;
        }
        let latest = header.latest.load(Ordering::Acquire) & 1;
        unsafe { std::ptr::copy_nonoverlapping(self.slot(latest), into.as_mut_ptr(), FRAME_BYTES) };
        true
    }
}

impl Drop for SharedFrame {
    fn drop(&mut self) {
        unsafe {
            let _ = UnmapViewOfFile(self.view);
            let _ = CloseHandle(self.mapping);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_writes_frames_the_camera_reads() {
        let camera = SharedFrame::create().unwrap();
        let app = SharedFrame::open().unwrap();
        let mut seen = vec![0; FRAME_BYTES];
        assert!(!camera.read(&mut seen));

        app.write(&vec![7; FRAME_BYTES]);
        app.write(&vec![9; FRAME_BYTES]);
        assert!(camera.read(&mut seen));
        assert!(seen.iter().all(|&byte| byte == 9));
    }
}
