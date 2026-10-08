// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Windows Cloud Files API (Cloud Filter API / cldapi) integration.
//!
//! Mounts remote phone catalogs into Windows File Explorer as native cloud placeholders.
//! Placeholders show file names, sizes, and cloud sync icons with zero physical disk usage
//! until hydrated on-demand when opened or copied by the user.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::TransferError;

/// Configuration for a cloud files sync root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudFilesConfig {
    /// Local directory where placeholder files will be placed.
    pub sync_root_path: PathBuf,
    /// Human-readable provider name shown in File Explorer (e.g. "Continue").
    pub provider_name: String,
    /// Provider version string.
    pub provider_version: String,
    /// Name of the paired phone (e.g. "Pixel 8").
    pub device_name: String,
    /// Unique identifier for the paired phone.
    pub device_id: String,
}

impl CloudFilesConfig {
    pub fn new<P: Into<PathBuf>>(
        sync_root_path: P,
        provider_name: impl Into<String>,
        device_name: impl Into<String>,
        device_id: impl Into<String>,
    ) -> Self {
        Self {
            sync_root_path: sync_root_path.into(),
            provider_name: provider_name.into(),
            provider_version: "1.0.0".to_string(),
            device_name: device_name.into(),
            device_id: device_id.into(),
        }
    }
}

/// Metadata describing a remote item to present as a local placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudFilesEntry {
    /// Relative path inside the sync root (e.g. "Photos/vacation.jpg").
    pub relative_path: PathBuf,
    /// Size of the file in bytes (0 for directories).
    pub file_size: u64,
    /// Whether this entry represents a directory.
    pub is_directory: bool,
    /// Creation or modification timestamp in milliseconds since UNIX epoch.
    pub modified_time_ms: u64,
    /// Remote identifier used to request content from the peer.
    pub file_identity: Vec<u8>,
}

impl CloudFilesEntry {
    pub fn file<P: Into<PathBuf>>(
        relative_path: P,
        file_size: u64,
        modified_time_ms: u64,
        file_identity: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            relative_path: relative_path.into(),
            file_size,
            is_directory: false,
            modified_time_ms,
            file_identity: file_identity.into(),
        }
    }

    pub fn directory<P: Into<PathBuf>>(
        relative_path: P,
        modified_time_ms: u64,
        file_identity: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            relative_path: relative_path.into(),
            file_size: 0,
            is_directory: true,
            modified_time_ms,
            file_identity: file_identity.into(),
        }
    }
}

/// Request passed to the fetch data callback when Windows needs file content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchDataRequest {
    /// Remote identifier specified when creating the placeholder.
    pub file_identity: Vec<u8>,
    /// Relative path within the sync root.
    pub relative_path: PathBuf,
    /// Byte offset requested by the reading process.
    pub offset: u64,
    /// Number of bytes requested.
    pub length: u64,
}

/// Hydration state of a placeholder file on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HydrationState {
    /// Placeholder exists on disk but contains no data (cloud icon).
    Unhydrated,
    /// File content is partially cached locally.
    PartiallyHydrated,
    /// Full file content is cached locally (green checkmark).
    FullyHydrated,
    /// Path does not exist or is not a cloud placeholder.
    NotAPlaceholder,
}

#[cfg(windows)]
#[allow(clippy::upper_case_acronyms)]
mod win32 {
    use std::ffi::c_void;

    pub type HRESULT = i32;
    pub const S_OK: HRESULT = 0;

    #[repr(C)]
    #[derive(Copy, Clone, Default)]
    pub struct GUID {
        pub data1: u32,
        pub data2: u16,
        pub data3: u16,
        pub data4: [u8; 8],
    }

    #[repr(C)]
    pub struct CF_SYNC_REGISTRATION {
        pub struct_size: u32,
        pub provider_name: *const u16,
        pub provider_version: *const u16,
        pub sync_root_identity: *const c_void,
        pub sync_root_identity_length: u32,
        pub file_identity: *const c_void,
        pub file_identity_length: u32,
        pub provider_id: GUID,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct CF_HYDRATION_POLICY {
        pub primary: u16,
        pub modifier: u16,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct CF_POPULATION_POLICY {
        pub primary: u16,
        pub modifier: u16,
    }

    #[repr(C)]
    pub struct CF_SYNC_POLICIES {
        pub struct_size: u32,
        pub hydration: CF_HYDRATION_POLICY,
        pub population: CF_POPULATION_POLICY,
        pub in_sync: u32,
        pub hard_link: u32,
        pub placeholder_management: u32,
    }

    pub const CF_HYDRATION_POLICY_PRIMARY_PARTIAL: u16 = 0;
    pub const CF_POPULATION_POLICY_PRIMARY_PARTIAL: u16 = 0;
    pub const CF_INSYNC_POLICY_NONE: u32 = 0;
    pub const CF_HARDLINK_POLICY_NONE: u32 = 0;
    pub const CF_PLACEHOLDER_MANAGEMENT_POLICY_DEFAULT: u32 = 0;

    pub const CF_REGISTER_FLAG_UPDATE: u32 = 1;
    pub const CF_CONNECT_FLAG_REQUIRE_FULL_FILE_PATH: u32 = 2;

    pub const CF_CALLBACK_TYPE_FETCH_DATA: u32 = 0;
    pub const CF_CALLBACK_TYPE_CANCEL_FETCH_DATA: u32 = 2;
    pub const CF_CALLBACK_TYPE_NONE: u32 = 0xFFFFFFFF;

    #[repr(C)]
    pub struct CF_CALLBACK_INFO {
        pub struct_size: u32,
        pub connection_key: i64,
        pub callback_context: *mut c_void,
        pub volume_guid_name: *const u16,
        pub volume_dos_name: *const u16,
        pub volume_serial_number: u32,
        pub sync_root_file_id: i64,
        pub sync_root_identity: *const c_void,
        pub sync_root_identity_length: u32,
        pub file_id: i64,
        pub file_size: i64,
        pub file_identity: *const c_void,
        pub file_identity_length: u32,
        pub normalized_path: *const u16,
        pub transfer_key: i64,
        pub priority_hint: u8,
        pub correlation_vector: *const c_void,
        pub process_info: *const c_void,
        pub request_key: i64,
    }

    #[repr(C)]
    pub struct CF_CALLBACK_PARAMETERS {
        pub param_size: u32,
        pub required_file_offset: i64,
        pub required_length: i64,
        pub optional_file_offset: i64,
        pub optional_length: i64,
        pub last_dehydration_time: i64,
        pub last_dehydration_reason: u32,
    }

    pub type CfCallbackFn = unsafe extern "system" fn(
        callback_info: *const CF_CALLBACK_INFO,
        callback_parameters: *const c_void,
    );

    #[repr(C)]
    pub struct CF_CALLBACK_REGISTRATION {
        pub callback_type: u32,
        pub callback: Option<CfCallbackFn>,
    }

    #[repr(C)]
    #[derive(Copy, Clone, Default)]
    pub struct FILE_BASIC_INFO {
        pub creation_time: i64,
        pub last_access_time: i64,
        pub last_write_time: i64,
        pub change_time: i64,
        pub file_attributes: u32,
    }

    #[repr(C)]
    pub struct CF_FS_METADATA {
        pub basic_info: FILE_BASIC_INFO,
        pub file_size: i64,
    }

    #[repr(C)]
    pub struct CF_PLACEHOLDER_CREATE_INFO {
        pub relative_file_name: *const u16,
        pub fs_metadata: CF_FS_METADATA,
        pub file_identity: *const c_void,
        pub file_identity_length: u32,
        pub flags: u32,
        pub result: HRESULT,
        pub create_usn: i64,
    }

    pub const CF_PLACEHOLDER_CREATE_FLAG_MARK_IN_SYNC: u32 = 4;
    pub const CF_CREATE_FLAG_NONE: u32 = 0;

    pub const CF_OPERATION_TYPE_TRANSFER_DATA: u32 = 0;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct CF_OPERATION_TRANSFER_DATA {
        pub flags: u32,
        pub completion_status: HRESULT,
        pub buffer: *const c_void,
        pub offset: i64,
        pub length: i64,
    }

    #[repr(C)]
    pub struct CF_OPERATION_PARAMETERS {
        pub param_size: u32,
        pub op_type: u32,
        pub connection_key: i64,
        pub transfer_key: i64,
        pub correlation_vector: *const c_void,
        pub sync_status: *const c_void,
        pub request_completion_status: HRESULT,
        pub transfer_data: CF_OPERATION_TRANSFER_DATA,
    }

    pub const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x00000010;
    pub const FILE_ATTRIBUTE_NORMAL: u32 = 0x00000080;

    pub const CF_PIN_STATE_PINNED: u32 = 1;
    pub const CF_PIN_STATE_UNPINNED: u32 = 2;

    pub const CF_PLACEHOLDER_STATE_PLACEHOLDER: u32 = 0x00000001;
    pub const CF_PLACEHOLDER_STATE_PARTIALLY_ON_DISK: u32 = 0x00000010;
    pub const CF_PLACEHOLDER_STATE_COMPLETELY_FULL: u32 = 0x00000020;

    pub const HKEY_CURRENT_USER: isize = 0x80000001u32 as i32 as isize;
    pub const KEY_WRITE: u32 = 0x00020006;
    pub const REG_SZ: u32 = 1;
    pub const REG_MULTI_SZ: u32 = 7;

    extern "system" {
        pub fn LoadLibraryW(lpLibFileName: *const u16) -> isize;
        pub fn GetProcAddress(hModule: isize, lpProcName: *const u8) -> *const c_void;
        pub fn FreeLibrary(hLibModule: isize) -> i32;
        pub fn CreateFileW(
            lpFileName: *const u16,
            dwDesiredAccess: u32,
            dwShareMode: u32,
            lpSecurityAttributes: *const c_void,
            dwCreationDisposition: u32,
            dwFlagsAndAttributes: u32,
            hTemplateFile: isize,
        ) -> isize;
        pub fn CloseHandle(hObject: isize) -> i32;
        pub fn RegCreateKeyExW(
            hKey: isize,
            lpSubKey: *const u16,
            Reserved: u32,
            lpClass: *const u16,
            dwOptions: u32,
            samDesired: u32,
            lpSecurityAttributes: *const c_void,
            phkResult: *mut isize,
            lpdwDisposition: *mut u32,
        ) -> i32;
        pub fn RegSetValueExW(
            hKey: isize,
            lpValueName: *const u16,
            Reserved: u32,
            dwType: u32,
            lpData: *const u8,
            cbData: u32,
        ) -> i32;
        pub fn RegCloseKey(hKey: isize) -> i32;
        pub fn RegDeleteKeyW(hKey: isize, lpSubKey: *const u16) -> i32;
    }
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn unix_ms_to_filetime(ms: u64) -> i64 {
    // 100-nanosecond intervals between January 1, 1601 and January 1, 1970
    const UNIX_EPOCH_DELTA: i64 = 116_444_736_000_000_000;
    let intervals = (ms as i64).saturating_mul(10_000);
    intervals.saturating_add(UNIX_EPOCH_DELTA)
}

#[cfg(windows)]
struct CldApi {
    hmodule: isize,
    register_sync_root: unsafe extern "system" fn(
        sync_root_path: *const u16,
        registration: *const win32::CF_SYNC_REGISTRATION,
        policies: *const win32::CF_SYNC_POLICIES,
        register_flags: u32,
    ) -> win32::HRESULT,
    unregister_sync_root: unsafe extern "system" fn(sync_root_path: *const u16) -> win32::HRESULT,
    connect_sync_root: unsafe extern "system" fn(
        sync_root_path: *const u16,
        callback_table: *const win32::CF_CALLBACK_REGISTRATION,
        callback_context: *const std::ffi::c_void,
        connect_flags: u32,
        connection_key: *mut i64,
    ) -> win32::HRESULT,
    disconnect_sync_root: unsafe extern "system" fn(connection_key: i64) -> win32::HRESULT,
    create_placeholders: unsafe extern "system" fn(
        sync_root_path: *const u16,
        placeholder_array: *mut win32::CF_PLACEHOLDER_CREATE_INFO,
        placeholder_count: u32,
        create_flags: u32,
        entries_processed: *mut u32,
    ) -> win32::HRESULT,
    execute: unsafe extern "system" fn(
        op_params: *const win32::CF_OPERATION_PARAMETERS,
    ) -> win32::HRESULT,
    set_pin_state: unsafe extern "system" fn(
        file_handle: isize,
        pin_state: u32,
        pin_flags: u32,
        pin_usn: *mut i64,
    ) -> win32::HRESULT,
    get_placeholder_state_from_attribute_tag:
        unsafe extern "system" fn(file_attributes: u32, reparse_tag: u32) -> u32,
}

#[cfg(windows)]
impl CldApi {
    fn load() -> Option<Self> {
        let name = to_wide("cldapi.dll");
        let hmodule = unsafe { win32::LoadLibraryW(name.as_ptr()) };
        if hmodule == 0 {
            return None;
        }

        macro_rules! resolve {
            ($sym:expr, $ty:ty) => {
                match unsafe { win32::GetProcAddress(hmodule, concat!($sym, "\0").as_ptr()) } {
                    p if !p.is_null() => unsafe {
                        std::mem::transmute::<*const std::ffi::c_void, $ty>(p)
                    },
                    _ => {
                        unsafe { win32::FreeLibrary(hmodule) };
                        return None;
                    }
                }
            };
        }

        let api = Self {
            hmodule,
            register_sync_root: resolve!(
                "CfRegisterSyncRoot",
                unsafe extern "system" fn(
                    *const u16,
                    *const win32::CF_SYNC_REGISTRATION,
                    *const win32::CF_SYNC_POLICIES,
                    u32,
                ) -> win32::HRESULT
            ),
            unregister_sync_root: resolve!(
                "CfUnregisterSyncRoot",
                unsafe extern "system" fn(*const u16) -> win32::HRESULT
            ),
            connect_sync_root: resolve!(
                "CfConnectSyncRoot",
                unsafe extern "system" fn(
                    *const u16,
                    *const win32::CF_CALLBACK_REGISTRATION,
                    *const std::ffi::c_void,
                    u32,
                    *mut i64,
                ) -> win32::HRESULT
            ),
            disconnect_sync_root: resolve!(
                "CfDisconnectSyncRoot",
                unsafe extern "system" fn(i64) -> win32::HRESULT
            ),
            create_placeholders: resolve!(
                "CfCreatePlaceholders",
                unsafe extern "system" fn(
                    *const u16,
                    *mut win32::CF_PLACEHOLDER_CREATE_INFO,
                    u32,
                    u32,
                    *mut u32,
                ) -> win32::HRESULT
            ),
            execute: resolve!(
                "CfExecute",
                unsafe extern "system" fn(*const win32::CF_OPERATION_PARAMETERS) -> win32::HRESULT
            ),
            set_pin_state: resolve!(
                "CfSetPinState",
                unsafe extern "system" fn(isize, u32, u32, *mut i64) -> win32::HRESULT
            ),
            get_placeholder_state_from_attribute_tag: resolve!(
                "CfGetPlaceholderStateFromAttributeTag",
                unsafe extern "system" fn(u32, u32) -> u32
            ),
        };

        Some(api)
    }
}

#[cfg(windows)]
impl Drop for CldApi {
    fn drop(&mut self) {
        if self.hmodule != 0 {
            unsafe { win32::FreeLibrary(self.hmodule) };
            self.hmodule = 0;
        }
    }
}

#[cfg(windows)]
struct MountContext {
    cldapi: Arc<CldApi>,
    fetch_handler: Box<dyn Fn(FetchDataRequest) -> Result<Vec<u8>, TransferError> + Send + Sync>,
}

#[cfg(windows)]
unsafe extern "system" fn fetch_data_callback(
    callback_info: *const win32::CF_CALLBACK_INFO,
    callback_parameters: *const std::ffi::c_void,
) {
    if callback_info.is_null() || callback_parameters.is_null() {
        return;
    }
    let info = &*callback_info;
    let params = &*(callback_parameters as *const win32::CF_CALLBACK_PARAMETERS);
    if info.callback_context.is_null() {
        return;
    }
    let ctx = &*(info.callback_context as *const MountContext);

    let file_identity = if !info.file_identity.is_null() && info.file_identity_length > 0 {
        std::slice::from_raw_parts(
            info.file_identity as *const u8,
            info.file_identity_length as usize,
        )
        .to_vec()
    } else {
        Vec::new()
    };

    let relative_path = if !info.normalized_path.is_null() {
        let mut len = 0;
        while *info.normalized_path.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(info.normalized_path, len);
        PathBuf::from(String::from_utf16_lossy(slice))
    } else {
        PathBuf::new()
    };

    let request = FetchDataRequest {
        file_identity,
        relative_path,
        offset: params.required_file_offset.max(0) as u64,
        length: params.required_length.max(0) as u64,
    };

    let mut op_params = win32::CF_OPERATION_PARAMETERS {
        param_size: std::mem::size_of::<win32::CF_OPERATION_PARAMETERS>() as u32,
        op_type: win32::CF_OPERATION_TYPE_TRANSFER_DATA,
        connection_key: info.connection_key,
        transfer_key: info.transfer_key,
        correlation_vector: std::ptr::null(),
        sync_status: std::ptr::null(),
        request_completion_status: win32::S_OK,
        transfer_data: win32::CF_OPERATION_TRANSFER_DATA {
            flags: 0,
            completion_status: win32::S_OK,
            buffer: std::ptr::null(),
            offset: params.required_file_offset,
            length: params.required_length,
        },
    };

    match (ctx.fetch_handler)(request) {
        Ok(data) => {
            op_params.transfer_data.buffer = data.as_ptr() as *const std::ffi::c_void;
            op_params.transfer_data.length = data.len() as i64;
            op_params.transfer_data.completion_status = win32::S_OK;
            (ctx.cldapi.execute)(&op_params);
        }
        Err(_) => {
            // ERROR_FILE_NOT_FOUND (0x80070002)
            op_params.transfer_data.completion_status = -2147024894;
            op_params.request_completion_status = -2147024894;
            (ctx.cldapi.execute)(&op_params);
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn cancel_fetch_data_callback(
    _callback_info: *const win32::CF_CALLBACK_INFO,
    _callback_parameters: *const std::ffi::c_void,
) {
    // Cancellation acknowledged
}

#[cfg(windows)]
struct WindowsMountState {
    connection_key: i64,
    cldapi: Arc<CldApi>,
    raw_context: *mut MountContext,
}

#[cfg(windows)]
unsafe impl Send for WindowsMountState {}
#[cfg(windows)]
unsafe impl Sync for WindowsMountState {}

/// Mounts remote phone storage into the operating system filesystem.
pub struct CloudFilesMount {
    config: CloudFilesConfig,
    #[cfg(windows)]
    state: std::sync::Mutex<Option<WindowsMountState>>,
}

impl CloudFilesMount {
    pub fn new(config: CloudFilesConfig) -> Self {
        Self {
            config,
            #[cfg(windows)]
            state: std::sync::Mutex::new(None),
        }
    }

    pub fn config(&self) -> &CloudFilesConfig {
        &self.config
    }

    /// Checks if the host operating system provides cloud filter placeholder support.
    pub fn is_supported() -> bool {
        #[cfg(windows)]
        {
            CldApi::load().is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    /// Registers the local directory as a cloud sync root with Windows.
    pub fn register(&self) -> Result<(), TransferError> {
        #[cfg(windows)]
        {
            let cldapi = CldApi::load().ok_or_else(|| TransferError::UnsupportedPlatform)?;

            std::fs::create_dir_all(&self.config.sync_root_path)?;

            let sync_root_w = to_wide(&self.config.sync_root_path.to_string_lossy());
            let provider_name_w = to_wide(&self.config.provider_name);
            let provider_version_w = to_wide(&self.config.provider_version);

            let registration = win32::CF_SYNC_REGISTRATION {
                struct_size: std::mem::size_of::<win32::CF_SYNC_REGISTRATION>() as u32,
                provider_name: provider_name_w.as_ptr(),
                provider_version: provider_version_w.as_ptr(),
                sync_root_identity: std::ptr::null(),
                sync_root_identity_length: 0,
                file_identity: std::ptr::null(),
                file_identity_length: 0,
                provider_id: win32::GUID::default(),
            };

            let policies = win32::CF_SYNC_POLICIES {
                struct_size: std::mem::size_of::<win32::CF_SYNC_POLICIES>() as u32,
                hydration: win32::CF_HYDRATION_POLICY {
                    primary: win32::CF_HYDRATION_POLICY_PRIMARY_PARTIAL,
                    modifier: 0,
                },
                population: win32::CF_POPULATION_POLICY {
                    primary: win32::CF_POPULATION_POLICY_PRIMARY_PARTIAL,
                    modifier: 0,
                },
                in_sync: win32::CF_INSYNC_POLICY_NONE,
                hard_link: win32::CF_HARDLINK_POLICY_NONE,
                placeholder_management: win32::CF_PLACEHOLDER_MANAGEMENT_POLICY_DEFAULT,
            };

            let hr = unsafe {
                (cldapi.register_sync_root)(
                    sync_root_w.as_ptr(),
                    &registration,
                    &policies,
                    win32::CF_REGISTER_FLAG_UPDATE,
                )
            };

            if hr != win32::S_OK {
                return Err(TransferError::CloudFiles(format!(
                    "CfRegisterSyncRoot failed with HRESULT 0x{hr:08X}"
                )));
            }

            // Register with Windows Explorer navigation pane
            let _ = register_sync_root_registry(&self.config);

            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(TransferError::UnsupportedPlatform)
        }
    }

    /// Unregisters the local directory from the operating system cloud filter driver.
    pub fn unregister(&self) -> Result<(), TransferError> {
        #[cfg(windows)]
        {
            let cldapi = CldApi::load().ok_or_else(|| TransferError::UnsupportedPlatform)?;
            self.disconnect()?;

            let sync_root_w = to_wide(&self.config.sync_root_path.to_string_lossy());
            let hr = unsafe { (cldapi.unregister_sync_root)(sync_root_w.as_ptr()) };

            let _ = unregister_sync_root_registry(&self.config);

            if hr != win32::S_OK {
                return Err(TransferError::CloudFiles(format!(
                    "CfUnregisterSyncRoot failed with HRESULT 0x{hr:08X}"
                )));
            }

            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(TransferError::UnsupportedPlatform)
        }
    }

    /// Connects to the cloud sync root, enabling live on-demand hydration callbacks.
    pub fn connect<F>(&self, fetch_handler: F) -> Result<(), TransferError>
    where
        F: Fn(FetchDataRequest) -> Result<Vec<u8>, TransferError> + Send + Sync + 'static,
    {
        #[cfg(windows)]
        {
            let mut state_lock = self.state.lock().unwrap();
            if state_lock.is_some() {
                return Ok(());
            }

            let cldapi =
                Arc::new(CldApi::load().ok_or_else(|| TransferError::UnsupportedPlatform)?);

            let context = Box::into_raw(Box::new(MountContext {
                cldapi: Arc::clone(&cldapi),
                fetch_handler: Box::new(fetch_handler),
            }));

            let callbacks = [
                win32::CF_CALLBACK_REGISTRATION {
                    callback_type: win32::CF_CALLBACK_TYPE_FETCH_DATA,
                    callback: Some(fetch_data_callback),
                },
                win32::CF_CALLBACK_REGISTRATION {
                    callback_type: win32::CF_CALLBACK_TYPE_CANCEL_FETCH_DATA,
                    callback: Some(cancel_fetch_data_callback),
                },
                win32::CF_CALLBACK_REGISTRATION {
                    callback_type: win32::CF_CALLBACK_TYPE_NONE,
                    callback: None,
                },
            ];

            let sync_root_w = to_wide(&self.config.sync_root_path.to_string_lossy());
            let mut connection_key: i64 = 0;

            let hr = unsafe {
                (cldapi.connect_sync_root)(
                    sync_root_w.as_ptr(),
                    callbacks.as_ptr(),
                    context as *const std::ffi::c_void,
                    win32::CF_CONNECT_FLAG_REQUIRE_FULL_FILE_PATH,
                    &mut connection_key,
                )
            };

            if hr != win32::S_OK {
                unsafe { drop(Box::from_raw(context)) };
                return Err(TransferError::CloudFiles(format!(
                    "CfConnectSyncRoot failed with HRESULT 0x{hr:08X}"
                )));
            }

            *state_lock = Some(WindowsMountState {
                connection_key,
                cldapi,
                raw_context: context,
            });

            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = fetch_handler;
            Err(TransferError::UnsupportedPlatform)
        }
    }

    /// Disconnects from the sync root callback pipeline.
    pub fn disconnect(&self) -> Result<(), TransferError> {
        #[cfg(windows)]
        {
            let mut state_lock = self.state.lock().unwrap();
            if let Some(state) = state_lock.take() {
                unsafe {
                    (state.cldapi.disconnect_sync_root)(state.connection_key);
                    if !state.raw_context.is_null() {
                        drop(Box::from_raw(state.raw_context));
                    }
                }
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Ok(())
        }
    }

    /// Creates placeholder files and directories inside the sync root.
    pub fn create_placeholders(&self, entries: &[CloudFilesEntry]) -> Result<u32, TransferError> {
        #[cfg(windows)]
        {
            if entries.is_empty() {
                return Ok(0);
            }

            let cldapi = CldApi::load().ok_or_else(|| TransferError::UnsupportedPlatform)?;

            let sync_root_w = to_wide(&self.config.sync_root_path.to_string_lossy());

            // Ensure parent directories exist before creating child placeholders
            for entry in entries {
                let full_path = self.config.sync_root_path.join(&entry.relative_path);
                if let Some(parent) = full_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }

            let wide_names: Vec<Vec<u16>> = entries
                .iter()
                .map(|e| to_wide(&e.relative_path.to_string_lossy()))
                .collect();

            let mut create_infos: Vec<win32::CF_PLACEHOLDER_CREATE_INFO> = entries
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let ft = unix_ms_to_filetime(entry.modified_time_ms);
                    let attributes = if entry.is_directory {
                        win32::FILE_ATTRIBUTE_DIRECTORY
                    } else {
                        win32::FILE_ATTRIBUTE_NORMAL
                    };

                    win32::CF_PLACEHOLDER_CREATE_INFO {
                        relative_file_name: wide_names[i].as_ptr(),
                        fs_metadata: win32::CF_FS_METADATA {
                            basic_info: win32::FILE_BASIC_INFO {
                                creation_time: ft,
                                last_access_time: ft,
                                last_write_time: ft,
                                change_time: ft,
                                file_attributes: attributes,
                            },
                            file_size: entry.file_size as i64,
                        },
                        file_identity: entry.file_identity.as_ptr() as *const std::ffi::c_void,
                        file_identity_length: entry.file_identity.len() as u32,
                        flags: win32::CF_PLACEHOLDER_CREATE_FLAG_MARK_IN_SYNC,
                        result: 0,
                        create_usn: 0,
                    }
                })
                .collect();

            let mut processed: u32 = 0;
            let hr = unsafe {
                (cldapi.create_placeholders)(
                    sync_root_w.as_ptr(),
                    create_infos.as_mut_ptr(),
                    create_infos.len() as u32,
                    win32::CF_CREATE_FLAG_NONE,
                    &mut processed,
                )
            };

            if hr != win32::S_OK && processed == 0 {
                return Err(TransferError::CloudFiles(format!(
                    "CfCreatePlaceholders failed with HRESULT 0x{hr:08X}"
                )));
            }

            Ok(processed)
        }
        #[cfg(not(windows))]
        {
            let _ = entries;
            Err(TransferError::UnsupportedPlatform)
        }
    }

    /// Sets the pin state of a placeholder file (pinned keeps local copy, unpinned permits dehydration).
    pub fn set_pin_state<P: AsRef<Path>>(
        &self,
        relative_path: P,
        pinned: bool,
    ) -> Result<(), TransferError> {
        #[cfg(windows)]
        {
            let cldapi = CldApi::load().ok_or_else(|| TransferError::UnsupportedPlatform)?;
            let full_path = self.config.sync_root_path.join(relative_path);
            let path_w = to_wide(&full_path.to_string_lossy());

            // FILE_WRITE_ATTRIBUTES (0x0100), FILE_SHARE_READ | FILE_SHARE_WRITE (3), OPEN_EXISTING (3)
            // FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT (0x02200000)
            let handle = unsafe {
                win32::CreateFileW(
                    path_w.as_ptr(),
                    0x0100,
                    3,
                    std::ptr::null(),
                    3,
                    0x02200000,
                    0,
                )
            };

            if handle == -1 || handle == 0 {
                return Err(TransferError::CloudFiles(
                    "Unable to open placeholder file handle for pin state update".to_string(),
                ));
            }

            let pin_state = if pinned {
                win32::CF_PIN_STATE_PINNED
            } else {
                win32::CF_PIN_STATE_UNPINNED
            };

            let hr = unsafe {
                let res = (cldapi.set_pin_state)(handle, pin_state, 0, std::ptr::null_mut());
                win32::CloseHandle(handle);
                res
            };

            if hr != win32::S_OK {
                return Err(TransferError::CloudFiles(format!(
                    "CfSetPinState failed with HRESULT 0x{hr:08X}"
                )));
            }

            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = relative_path;
            let _ = pinned;
            Err(TransferError::UnsupportedPlatform)
        }
    }

    /// Queries the current hydration state of a file inside the sync root.
    pub fn get_hydration_state<P: AsRef<Path>>(
        &self,
        relative_path: P,
    ) -> Result<HydrationState, TransferError> {
        #[cfg(windows)]
        {
            let cldapi = CldApi::load().ok_or_else(|| TransferError::UnsupportedPlatform)?;
            let full_path = self.config.sync_root_path.join(relative_path);
            if !full_path.exists() {
                return Ok(HydrationState::NotAPlaceholder);
            }

            use std::os::windows::fs::MetadataExt;
            let metadata = std::fs::symlink_metadata(&full_path)?;
            let attributes = metadata.file_attributes();

            // Reparse tag for Cloud Filter placeholders is IO_REPARSE_TAG_CLOUD (0x9000001A)
            const IO_REPARSE_TAG_CLOUD: u32 = 0x9000001A;
            let state_flags = unsafe {
                (cldapi.get_placeholder_state_from_attribute_tag)(attributes, IO_REPARSE_TAG_CLOUD)
            };

            if (state_flags & win32::CF_PLACEHOLDER_STATE_COMPLETELY_FULL) != 0 {
                Ok(HydrationState::FullyHydrated)
            } else if (state_flags & win32::CF_PLACEHOLDER_STATE_PARTIALLY_ON_DISK) != 0 {
                Ok(HydrationState::PartiallyHydrated)
            } else if (state_flags & win32::CF_PLACEHOLDER_STATE_PLACEHOLDER) != 0 {
                Ok(HydrationState::Unhydrated)
            } else {
                Ok(HydrationState::NotAPlaceholder)
            }
        }
        #[cfg(not(windows))]
        {
            let _ = relative_path;
            Err(TransferError::UnsupportedPlatform)
        }
    }
}

impl Drop for CloudFilesMount {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

/// Registers the sync root in the Windows Registry to display it in File Explorer's sidebar.
#[cfg(windows)]
pub fn register_sync_root_registry(config: &CloudFilesConfig) -> Result<(), TransferError> {
    let subkey = format!(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\SyncRootManager\\{}!{}",
        config.provider_name, config.device_id
    );
    let subkey_w = to_wide(&subkey);
    let mut hkey: isize = 0;
    let mut disposition: u32 = 0;

    unsafe {
        let res = win32::RegCreateKeyExW(
            win32::HKEY_CURRENT_USER,
            subkey_w.as_ptr(),
            0,
            std::ptr::null(),
            0,
            win32::KEY_WRITE,
            std::ptr::null(),
            &mut hkey,
            &mut disposition,
        );
        if res != 0 {
            return Err(TransferError::CloudFiles(format!(
                "RegCreateKeyExW failed with code {res}"
            )));
        }

        let display_name = format!("{} ({})", config.provider_name, config.device_name);
        let display_name_w = to_wide(&display_name);
        win32::RegSetValueExW(
            hkey,
            to_wide("DisplayName").as_ptr(),
            0,
            win32::REG_SZ,
            display_name_w.as_ptr() as *const u8,
            (display_name_w.len() * 2) as u32,
        );

        let path_str = config.sync_root_path.to_string_lossy().to_string();
        let mut multi_sz = to_wide(&path_str);
        multi_sz.push(0); // Double null termination for REG_MULTI_SZ
        win32::RegSetValueExW(
            hkey,
            to_wide("UserSyncRoots").as_ptr(),
            0,
            win32::REG_MULTI_SZ,
            multi_sz.as_ptr() as *const u8,
            (multi_sz.len() * 2) as u32,
        );

        win32::RegCloseKey(hkey);
    }
    Ok(())
}

/// Removes the sync root entry from the Windows Registry.
#[cfg(windows)]
pub fn unregister_sync_root_registry(config: &CloudFilesConfig) -> Result<(), TransferError> {
    let subkey = format!(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\SyncRootManager\\{}!{}",
        config.provider_name, config.device_id
    );
    let subkey_w = to_wide(&subkey);
    unsafe {
        win32::RegDeleteKeyW(win32::HKEY_CURRENT_USER, subkey_w.as_ptr());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_initialization() {
        let config = CloudFilesConfig::new(
            PathBuf::from(r"C:\Users\test\Continue\Pixel8"),
            "Continue",
            "Pixel 8",
            "pixel-8-unique-id",
        );
        assert_eq!(config.provider_name, "Continue");
        assert_eq!(config.device_name, "Pixel 8");
        assert_eq!(config.provider_version, "1.0.0");
    }

    #[test]
    fn entry_constructors() {
        let file_entry =
            CloudFilesEntry::file("Photos/test.jpg", 1024, 1700000000, b"id-1".to_vec());
        assert_eq!(file_entry.file_size, 1024);
        assert!(!file_entry.is_directory);
        assert_eq!(file_entry.file_identity, b"id-1");

        let dir_entry = CloudFilesEntry::directory("Photos", 1700000000, b"dir-1".to_vec());
        assert_eq!(dir_entry.file_size, 0);
        assert!(dir_entry.is_directory);
    }

    #[test]
    fn platform_support_detection() {
        let supported = CloudFilesMount::is_supported();
        if cfg!(windows) {
            assert!(
                supported,
                "cldapi.dll should be supported on modern Windows"
            );
        } else {
            assert!(!supported);
        }
    }
}
