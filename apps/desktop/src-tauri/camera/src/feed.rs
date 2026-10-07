// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! The app's side: turns the phone's H.264 into frames for the camera, and puts the camera on
//! the system while it runs.

use std::path::{Path, PathBuf};

use windows::core::{s, w, Interface, Result, GUID, HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, FreeLibrary, SIZE};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

use crate::fit::{fit, Picture};
use crate::shared::{SharedFrame, FRAME_BYTES};
use crate::{installed_path, CLSID};

/// The phone's H.264 as NV12 pictures, through the decoder Windows ships.
pub struct Decoder {
    transform: IMFTransform,
    /// Width, height, stride and rows in the brightness plane, once the decoder says.
    layout: Option<(usize, usize, usize, usize)>,
    picture: Vec<u8>,
    frames: i64,
}

impl Decoder {
    pub fn new() -> Result<Self> {
        unsafe {
            let transform: IMFTransform =
                CoCreateInstance(&CLSID_MSH264DecoderMFT, None, CLSCTX_INPROC_SERVER)?;
            transform.GetAttributes()?.SetUINT32(&MF_LOW_LATENCY, 1)?;
            let input = MFCreateMediaType()?;
            input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
            transform.SetInputType(0, &input, 0)?;
            let mut decoder = Self {
                transform,
                layout: None,
                picture: Vec::new(),
                frames: 0,
            };
            decoder.choose_output()?;
            decoder
                .transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)?;
            Ok(decoder)
        }
    }

    /// Picks NV12 and notes the picture's layout; again whenever the stream changes size.
    fn choose_output(&mut self) -> Result<()> {
        unsafe {
            let mut index = 0;
            let output = loop {
                let output = self.transform.GetOutputAvailableType(0, index)?;
                if output.GetGUID(&MF_MT_SUBTYPE)? == MFVideoFormat_NV12 {
                    break output;
                }
                index += 1;
            };
            self.transform.SetOutputType(0, &output, 0)?;
            // Decoders pad the size to whole blocks; the part to show is the aperture.
            let size = output.GetUINT64(&MF_MT_FRAME_SIZE)?;
            let (padded_width, padded_height) =
                ((size >> 32) as usize, (size & 0xffff_ffff) as usize);
            let mut area = MFVideoArea::default();
            let shown = output.GetBlob(
                &MF_MT_MINIMUM_DISPLAY_APERTURE,
                std::slice::from_raw_parts_mut(
                    (&mut area as *mut MFVideoArea).cast(),
                    size_of::<MFVideoArea>(),
                ),
                None,
            );
            let SIZE { cx, cy } = if shown.is_ok() {
                area.Area
            } else {
                SIZE {
                    cx: padded_width as i32,
                    cy: padded_height as i32,
                }
            };
            // A contiguous copy packs rows at the padded width.
            self.layout = Some((cx as usize, cy as usize, padded_width, padded_height));
            Ok(())
        }
    }

    /// Takes one encoded frame; the newest picture it finished, if any.
    pub fn decode(&mut self, data: &[u8]) -> Result<Option<Picture<'_>>> {
        let mut finished = false;
        unsafe {
            let buffer = MFCreateMemoryBuffer(data.len() as u32)?;
            let mut into = std::ptr::null_mut();
            buffer.Lock(&mut into, None, None)?;
            std::ptr::copy_nonoverlapping(data.as_ptr(), into, data.len());
            buffer.Unlock()?;
            buffer.SetCurrentLength(data.len() as u32)?;
            let sample = MFCreateSample()?;
            sample.AddBuffer(&buffer)?;
            sample.SetSampleTime(self.frames * 333_333)?;
            self.frames += 1;
            loop {
                match self.transform.ProcessInput(0, &sample, 0) {
                    Err(error) if error.code() == MF_E_NOTACCEPTING => finished |= self.drain()?,
                    result => break result?,
                }
            }
            finished |= self.drain()?;
        }
        let Some((width, height, stride, plane_rows)) = self.layout else {
            return Ok(None);
        };
        Ok(finished.then(|| Picture {
            data: &self.picture,
            width,
            height,
            stride,
            plane_rows,
        }))
    }

    /// Takes every picture the decoder has ready, keeping the last. True if there was one.
    fn drain(&mut self) -> Result<bool> {
        let mut finished = false;
        loop {
            unsafe {
                let size = self.transform.GetOutputStreamInfo(0)?.cbSize;
                let sample = MFCreateSample()?;
                sample.AddBuffer(&MFCreateMemoryBuffer(size)?)?;
                let mut output = [MFT_OUTPUT_DATA_BUFFER {
                    dwStreamID: 0,
                    pSample: std::mem::ManuallyDrop::new(Some(sample)),
                    dwStatus: 0,
                    pEvents: std::mem::ManuallyDrop::new(None),
                }];
                let mut status = 0;
                let result = self.transform.ProcessOutput(0, &mut output, &mut status);
                let sample = std::mem::ManuallyDrop::take(&mut output[0].pSample);
                drop(std::mem::ManuallyDrop::take(&mut output[0].pEvents));
                match result {
                    Ok(()) => {}
                    Err(error) if error.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => {
                        return Ok(finished)
                    }
                    Err(error) if error.code() == MF_E_TRANSFORM_STREAM_CHANGE => {
                        self.choose_output()?;
                        continue;
                    }
                    Err(error) => return Err(error),
                }
                let buffer = sample
                    .ok_or_else(windows::core::Error::empty)?
                    .ConvertToContiguousBuffer()?;
                let mut data = std::ptr::null_mut();
                let mut length = 0;
                buffer.Lock(&mut data, None, Some(&mut length))?;
                self.picture.clear();
                self.picture
                    .extend_from_slice(std::slice::from_raw_parts(data, length as usize));
                buffer.Unlock()?;
                finished = true;
            }
        }
    }
}

#[derive(Debug)]
pub enum StartError {
    /// Virtual cameras came with Windows 11.
    Unsupported,
    /// Someone said no to Windows asking to let Continue add its camera.
    Declined,
    Failed(windows::core::Error),
}

impl From<windows::core::Error> for StartError {
    fn from(error: windows::core::Error) -> Self {
        Self::Failed(error)
    }
}

type CreateVirtualCamera = unsafe extern "system" fn(
    MFVirtualCameraType,
    MFVirtualCameraLifetime,
    MFVirtualCameraAccess,
    PCWSTR,
    PCWSTR,
    *const GUID,
    u32,
    *mut *mut std::ffi::c_void,
) -> windows::core::HRESULT;

/// The camera other apps can pick, there until this is dropped, with frames from the phone.
pub struct Webcam {
    camera: IMFVirtualCamera,
    decoder: Decoder,
    shared: Option<SharedFrame>,
    frame: Vec<u8>,
}

impl Webcam {
    /// `dll` is the camera DLL that came with the app. The first time, and after an update,
    /// Windows asks to let it install the camera.
    pub fn start(dll: &Path) -> std::result::Result<Self, StartError> {
        unsafe {
            MFStartup(MF_VERSION, MFSTARTUP_FULL)?;
            // Looked up rather than linked, so the app still starts on Windows 10.
            let library =
                LoadLibraryW(w!("mfsensorgroup.dll")).map_err(|_| StartError::Unsupported)?;
            let create = GetProcAddress(library, s!("MFCreateVirtualCamera"));
            let Some(create) = create else {
                let _ = FreeLibrary(library);
                return Err(StartError::Unsupported);
            };
            let create: CreateVirtualCamera = std::mem::transmute(create);
            if !installed(dll) {
                install(dll)?;
            }
            let class = HSTRING::from(format!("{{{CLSID:?}}}"));
            let mut camera = std::ptr::null_mut();
            create(
                MFVirtualCameraType_SoftwareCameraSource,
                MFVirtualCameraLifetime_Session,
                MFVirtualCameraAccess_CurrentUser,
                w!("Continue"),
                PCWSTR(class.as_ptr()),
                std::ptr::null(),
                0,
                &mut camera,
            )
            .ok()?;
            let camera = IMFVirtualCamera::from_raw(camera);
            camera.Start(None)?;
            Ok(Self {
                camera,
                decoder: Decoder::new()?,
                shared: None,
                frame: vec![0; FRAME_BYTES],
            })
        }
    }

    /// Shows the next frame from the phone to any app using the camera.
    pub fn push(&mut self, h264: &[u8], quarter_turns: u32) -> Result<()> {
        let Some(picture) = self.decoder.decode(h264)? else {
            return Ok(());
        };
        // The camera makes the shared frame when an app starts using it.
        if self.shared.is_none() {
            self.shared = SharedFrame::open();
        }
        if let Some(shared) = &self.shared {
            fit(&picture, quarter_turns, &mut self.frame);
            shared.write(&self.frame);
        }
        Ok(())
    }
}

impl Drop for Webcam {
    fn drop(&mut self) {
        unsafe {
            let _ = self.camera.Shutdown();
        }
    }
}

/// Whether the registered camera is this copy of the DLL.
fn installed(dll: &Path) -> bool {
    let Some(target) = installed_path() else {
        return false;
    };
    let key = HSTRING::from(format!(
        "SOFTWARE\\Classes\\CLSID\\{{{CLSID:?}}}\\InprocServer32"
    ));
    let mut path = [0u16; 1024];
    let mut size = (path.len() * 2) as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &key,
            PCWSTR::null(),
            RRF_RT_REG_SZ,
            None,
            Some(path.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    let registered = PathBuf::from(String::from_utf16_lossy(
        &path[..(size as usize / 2).saturating_sub(1)],
    ));
    read.is_ok() && registered == target && std::fs::read(&target).ok() == std::fs::read(dll).ok()
}

/// Registers the camera through regsvr32, which Windows asks permission for.
fn install(dll: &Path) -> std::result::Result<(), StartError> {
    let arguments = HSTRING::from(format!("/s \"{}\"", dll.display()));
    let mut run = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: w!("regsvr32.exe"),
        lpParameters: PCWSTR(arguments.as_ptr()),
        ..Default::default()
    };
    unsafe {
        ShellExecuteExW(&mut run).map_err(|_| StartError::Declined)?;
        WaitForSingleObject(run.hProcess, INFINITE);
        let mut code = 1;
        let waited = GetExitCodeProcess(run.hProcess, &mut code);
        let _ = CloseHandle(run.hProcess);
        waited?;
        if code != 0 {
            return Err(StartError::Failed(windows::core::Error::from_hresult(
                windows::core::HRESULT(code as i32),
            )));
        }
    }
    Ok(())
}
