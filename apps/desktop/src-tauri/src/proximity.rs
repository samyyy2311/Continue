// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Locks this computer when a paired phone moves away, judged by the Bluetooth signal the
//! phone sends out with its presence token. Windows only for now.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

/// The service the phone's presence token is sent under, shared with the phone app.
#[cfg(windows)]
const PRESENCE_SERVICE: u128 = 0xfc36928a_e182_4ac1_adef_598feab5b6c8;

#[derive(Clone, Default)]
pub struct LockWhenAway(Arc<AtomicBool>);

fn saved(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("lock-when-away"))
}

pub fn start(app: &AppHandle) -> LockWhenAway {
    let on = saved(app).is_some_and(|path| path.exists());
    let setting = LockWhenAway(Arc::new(AtomicBool::new(on)));
    #[cfg(windows)]
    windows_watch::watch(app.clone(), setting.clone());
    setting
}

/// None where this isn't available.
#[tauri::command]
pub fn get_lock_when_away(setting: State<LockWhenAway>) -> Option<bool> {
    cfg!(windows).then(|| setting.0.load(Ordering::Relaxed))
}

#[tauri::command]
pub fn set_lock_when_away(app: AppHandle, setting: State<LockWhenAway>, on: bool) {
    setting.0.store(on, Ordering::Relaxed);
    if let Some(path) = saved(&app) {
        let result = if on {
            std::fs::write(path, b"")
        } else {
            std::fs::remove_file(path)
        };
        if let Err(error) = result {
            tracing::warn!("Couldn't save whether to lock when away: {error}");
        }
    }
}

#[cfg(windows)]
mod windows_watch {
    use std::collections::HashMap;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use parking_lot::Mutex;
    use tauri::{AppHandle, Manager};
    use windows::Devices::Bluetooth::Advertisement::{
        BluetoothLEAdvertisementReceivedEventArgs, BluetoothLEAdvertisementWatcher,
        BluetoothLEScanningMode,
    };
    use windows::Foundation::TypedEventHandler;
    use windows::Storage::Streams::DataReader;

    use super::{LockWhenAway, PRESENCE_SERVICE};
    use crate::DesktopRuntimeState;

    const CHECK: Duration = Duration::from_secs(2);
    /// Not heard from for this long counts as gone.
    const GONE_AFTER: Duration = Duration::from_secs(30);
    /// Signal strengths in dBm, smoothed. Below FAR is out of the room; back above NEAR the
    /// computer may lock again next time. Rough values: walls and pockets vary a lot.
    const FAR_DBM: f32 = -85.0;
    const NEAR_DBM: f32 = -75.0;
    /// How much each new reading moves the smoothed signal.
    const SMOOTHING: f32 = 0.2;
    /// Bluetooth's type for service data under a 128-bit service id.
    const SERVICE_DATA_128: u8 = 0x21;

    struct Sighting {
        at: Instant,
        signal: f32,
    }

    pub(super) fn watch(app: AppHandle, setting: LockWhenAway) {
        // Which phone each token belongs to, refreshed as the windows move on.
        let tokens: Arc<Mutex<HashMap<[u8; 8], String>>> = Arc::default();
        let sightings: Arc<Mutex<HashMap<String, Sighting>>> = Arc::default();
        std::thread::spawn(move || {
            let mut watcher: Option<BluetoothLEAdvertisementWatcher> = None;
            // Locked for the phone's current absence; it has to come back before locking again.
            let mut locked = false;
            loop {
                std::thread::sleep(CHECK);
                let on = setting.0.load(Ordering::Relaxed);
                if on && watcher.is_none() {
                    // Tried again each round, since Bluetooth may be turned on later.
                    watcher = listen(tokens.clone(), sightings.clone())
                        .map_err(|error| tracing::debug!("No Bluetooth to watch with: {error}"))
                        .ok();
                } else if !on {
                    if let Some(stopped) = watcher.take() {
                        let _ = stopped.Stop();
                        sightings.lock().clear();
                    }
                }
                if watcher.is_none() {
                    continue;
                }
                *tokens.lock() = current_tokens(&app);
                // Only phones seen since watching started count, so Bluetooth being off never locks.
                let sightings = sightings.lock();
                if sightings.is_empty() {
                    continue;
                }
                let away = sightings
                    .values()
                    .all(|s| s.at.elapsed() > GONE_AFTER || s.signal < FAR_DBM);
                let near = sightings
                    .values()
                    .any(|s| s.at.elapsed() <= GONE_AFTER && s.signal > NEAR_DBM);
                if away && !locked {
                    locked = unsafe { windows::Win32::System::Shutdown::LockWorkStation() }.is_ok();
                } else if near {
                    locked = false;
                }
            }
        });
    }

    /// Each paired phone's token for this window and the last, in case the clocks differ a little.
    fn current_tokens(app: &AppHandle) -> HashMap<[u8; 8], String> {
        let device = app.state::<DesktopRuntimeState>().device.clone();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        let earlier = now.saturating_sub(pairing::PRESENCE_WINDOW_SECS);
        device
            .stores
            .trust
            .list_peers()
            .unwrap_or_default()
            .into_iter()
            .flat_map(|peer| {
                [now, earlier].map(|at| {
                    (
                        pairing::presence_token(&peer.fingerprint, at),
                        peer.fingerprint.clone(),
                    )
                })
            })
            .collect()
    }

    fn listen(
        tokens: Arc<Mutex<HashMap<[u8; 8], String>>>,
        sightings: Arc<Mutex<HashMap<String, Sighting>>>,
    ) -> windows::core::Result<BluetoothLEAdvertisementWatcher> {
        let watcher = BluetoothLEAdvertisementWatcher::new()?;
        watcher.SetScanningMode(BluetoothLEScanningMode::Passive)?;
        let handler = TypedEventHandler::<
            BluetoothLEAdvertisementWatcher,
            BluetoothLEAdvertisementReceivedEventArgs,
        >::new(move |_, args| {
            let Some(args) = args.as_ref() else {
                return Ok(());
            };
            let Some(token) = presence_token(args) else {
                return Ok(());
            };
            let Some(phone) = tokens.lock().get(&token).cloned() else {
                return Ok(());
            };
            let reading = f32::from(args.RawSignalStrengthInDBm()?);
            let mut sightings = sightings.lock();
            let sighting = sightings.entry(phone).or_insert(Sighting {
                at: Instant::now(),
                signal: reading,
            });
            sighting.at = Instant::now();
            sighting.signal += (reading - sighting.signal) * SMOOTHING;
            Ok(())
        });
        watcher.Received(&handler)?;
        watcher.Start()?;
        Ok(watcher)
    }

    /// The token in the phone's advertisement: service data under the presence service.
    fn presence_token(args: &BluetoothLEAdvertisementReceivedEventArgs) -> Option<[u8; 8]> {
        let sections = args.Advertisement().ok()?.DataSections().ok()?;
        let service = PRESENCE_SERVICE.to_le_bytes();
        (0..sections.Size().ok()?).find_map(|index| {
            let section = sections.GetAt(index).ok()?;
            if section.DataType().ok()? != SERVICE_DATA_128 {
                return None;
            }
            let buffer = section.Data().ok()?;
            let mut data = vec![0; buffer.Length().ok()? as usize];
            DataReader::FromBuffer(&buffer)
                .ok()?
                .ReadBytes(&mut data)
                .ok()?;
            let token = data.strip_prefix(&service[..])?;
            token.try_into().ok()
        })
    }
}
