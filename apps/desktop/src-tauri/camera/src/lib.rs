// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! The phone's camera as a webcam other apps can pick, on Windows 11. Windows' camera service
//! loads this DLL and shows whatever frames the app puts in shared memory.

#![cfg(windows)]

mod feed;
pub mod fit;
pub mod shared;
mod source;

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicPtr, Ordering};

use windows::core::{implement, IUnknown, Interface, Ref, Result, BOOL, GUID, HRESULT, HSTRING};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_FAIL, HMODULE, S_FALSE, S_OK,
};
use windows::Win32::Media::MediaFoundation::{MFStartup, MFSTARTUP_FULL, MF_VERSION};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{
    RegDeleteTreeW, RegSetKeyValueW, HKEY_LOCAL_MACHINE, REG_SZ,
};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

pub use feed::{Decoder, StartError, Webcam};
pub use source::Source;

/// The class Windows creates the camera from.
pub const CLSID: GUID = GUID::from_u128(0xa2c613ab_fca6_4a62_b887_927d653e4cef);

static MODULE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

#[no_mangle]
extern "system" fn DllMain(module: HMODULE, reason: u32, _: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        MODULE.store(module.0, Ordering::Relaxed);
    }
    true.into()
}

#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<IUnknown>,
        iid: *const GUID,
        object: *mut *mut c_void,
    ) -> Result<()> {
        if !outer.is_null() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        unsafe {
            MFStartup(MF_VERSION, MFSTARTUP_FULL)?;
            source::Activator::create()?.query(iid, object).ok()
        }
    }

    fn LockServer(&self, _: BOOL) -> Result<()> {
        Ok(())
    }
}

#[no_mangle]
unsafe extern "system" fn DllGetClassObject(
    class: *const GUID,
    iid: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    if unsafe { *class } != CLSID {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = Factory.into();
    unsafe { factory.query(iid, object) }
}

/// The camera service may hold it for as long as it runs.
#[no_mangle]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}

fn class_key() -> HSTRING {
    HSTRING::from(format!("SOFTWARE\\Classes\\CLSID\\{{{CLSID:?}}}"))
}

/// Where the registered copy lives: the camera service can't read files in someone's profile.
pub fn installed_path() -> Option<PathBuf> {
    let data = std::env::var_os("ProgramData")?;
    Some(
        PathBuf::from(data)
            .join("Continue")
            .join("continue_camera.dll"),
    )
}

fn this_dll() -> Option<PathBuf> {
    let module = HMODULE(MODULE.load(Ordering::Relaxed));
    let mut path = [0u16; 1024];
    let length = unsafe { GetModuleFileNameW(Some(module), &mut path) } as usize;
    (length > 0 && length < path.len())
        .then(|| PathBuf::from(String::from_utf16_lossy(&path[..length])))
}

/// Run elevated through regsvr32: copies this DLL to `installed_path` and registers that copy.
#[no_mangle]
extern "system" fn DllRegisterServer() -> HRESULT {
    let (Some(from), Some(to)) = (this_dll(), installed_path()) else {
        return E_FAIL;
    };
    if from != to {
        let copied = to
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::copy(&from, &to));
        if copied.is_err() {
            return E_FAIL;
        }
    }
    let server = HSTRING::from(format!("{}\\InprocServer32", class_key()));
    let set = |name: Option<&str>, value: &str| {
        let value: Vec<u16> = value.encode_utf16().chain([0]).collect();
        let name = name.map(HSTRING::from).unwrap_or_default();
        unsafe {
            RegSetKeyValueW(
                HKEY_LOCAL_MACHINE,
                &server,
                &name,
                REG_SZ.0,
                Some(value.as_ptr().cast()),
                (value.len() * 2) as u32,
            )
        }
        .ok()
    };
    match set(None, &to.to_string_lossy()).and_then(|()| set(Some("ThreadingModel"), "Both")) {
        Ok(()) => S_OK,
        Err(error) => error.code(),
    }
}

/// The installed copy is in use while this runs, so the uninstaller deletes it afterwards.
#[no_mangle]
extern "system" fn DllUnregisterServer() -> HRESULT {
    let _ = unsafe { RegDeleteTreeW(HKEY_LOCAL_MACHINE, &class_key()) };
    S_OK
}
