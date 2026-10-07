// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Reads the camera the way an app does, through the class Windows would create it from.

#![cfg(windows)]

use continue_camera::shared::{SharedFrame, FRAME_BYTES};
use windows::core::{Interface, GUID};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{CoInitializeEx, IClassFactory, COINIT_MULTITHREADED};

extern "system" {
    fn DllGetClassObject(
        class: *const GUID,
        iid: *const GUID,
        object: *mut *mut std::ffi::c_void,
    ) -> windows::core::HRESULT;
}

/// The first brightness byte of the next frame the camera gives.
fn next_frame(reader: &IMFSourceReader) -> u8 {
    unsafe {
        let mut flags = 0;
        let mut sample = None;
        reader
            .ReadSample(
                MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32,
                0,
                None,
                Some(&mut flags),
                None,
                Some(&mut sample),
            )
            .unwrap();
        let buffer = sample
            .expect("a frame")
            .ConvertToContiguousBuffer()
            .unwrap();
        let mut data = std::ptr::null_mut();
        let mut length = 0;
        buffer.Lock(&mut data, None, Some(&mut length)).unwrap();
        assert_eq!(length as usize, FRAME_BYTES);
        let first = *data;
        buffer.Unlock().unwrap();
        first
    }
}

#[test]
fn an_app_sees_a_dark_picture_then_the_frames_continue_sends() {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
        MFStartup(MF_VERSION, MFSTARTUP_FULL).unwrap();
        let mut factory = std::ptr::null_mut();
        DllGetClassObject(&continue_camera::CLSID, &IClassFactory::IID, &mut factory)
            .ok()
            .unwrap();
        let factory = IClassFactory::from_raw(factory);
        let activate: IMFActivate = factory.CreateInstance(None).unwrap();
        let source: IMFMediaSource = activate.ActivateObject().unwrap();
        let reader = MFCreateSourceReaderFromMediaSource(&source, None).unwrap();

        assert_eq!(next_frame(&reader), 16);

        let app = SharedFrame::open().expect("the camera shares its frames once in use");
        app.write(&vec![77; FRAME_BYTES]);
        assert_eq!(next_frame(&reader), 77);

        activate.ShutdownObject().unwrap();
    }
}
