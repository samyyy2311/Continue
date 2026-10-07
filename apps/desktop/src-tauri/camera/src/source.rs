// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! The camera as Windows' camera service sees it: an activation object that makes a live media
//! source with one NV12 stream, which hands out the app's newest frame each time it's asked.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows::core::{
    implement, AsImpl, IUnknown, Interface, Ref, Result, BOOL, GUID, HRESULT, PCWSTR, PWSTR,
};
use windows::Win32::Foundation::S_OK;
use windows::Win32::Media::KernelStreaming::{
    IKsControl, IKsControl_Impl, KSIDENTIFIER, PINNAME_VIDEO_CAPTURE,
};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;

use crate::shared::{SharedFrame, FRAME_BYTES, HEIGHT, WIDTH};

const FRAME_RATE: u64 = 30;
/// In 100-nanosecond units, as Media Foundation times everything.
const FRAME_TICKS: i64 = 10_000_000 / FRAME_RATE as i64;
const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / FRAME_RATE);

/// What Windows answers for a property set nobody implements.
const NOT_SUPPORTED: HRESULT = HRESULT(0x8007_0492_u32 as i32);

fn queue_event(
    events: &IMFMediaEventQueue,
    event: MF_EVENT_TYPE,
    value: Option<&IUnknown>,
) -> Result<()> {
    unsafe {
        match value {
            Some(value) => events.QueueEventParamUnk(event.0 as u32, &GUID::zeroed(), S_OK, value),
            None => events.QueueEventParamVar(
                event.0 as u32,
                &GUID::zeroed(),
                S_OK,
                &PROPVARIANT::default(),
            ),
        }
    }
}

fn media_type() -> Result<IMFMediaType> {
    let pack = |high: u64, low: u64| high << 32 | low;
    unsafe {
        let media_type = MFCreateMediaType()?;
        media_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        media_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
        media_type.SetUINT64(&MF_MT_FRAME_SIZE, pack(WIDTH as u64, HEIGHT as u64))?;
        media_type.SetUINT64(&MF_MT_FRAME_RATE, pack(FRAME_RATE, 1))?;
        media_type.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))?;
        media_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        media_type.SetUINT32(&MF_MT_ALL_SAMPLES_INDEPENDENT, 1)?;
        media_type.SetUINT32(&MF_MT_FIXED_SIZE_SAMPLES, 1)?;
        media_type.SetUINT32(&MF_MT_SAMPLE_SIZE, FRAME_BYTES as u32)?;
        media_type.SetUINT32(&MF_MT_DEFAULT_STRIDE, WIDTH as u32)?;
        Ok(media_type)
    }
}

#[implement(IMFMediaSourceEx, IMFGetService, IKsControl)]
pub struct Source {
    events: IMFMediaEventQueue,
    attributes: IMFAttributes,
    descriptor: IMFPresentationDescriptor,
    stream: OnceLock<IMFMediaStream2>,
    shut_down: AtomicBool,
}

impl Source {
    pub fn create() -> Result<IMFMediaSourceEx> {
        unsafe {
            let stream_descriptor = MFCreateStreamDescriptor(0, &[Some(media_type()?)])?;
            let handler = stream_descriptor.GetMediaTypeHandler()?;
            handler.SetCurrentMediaType(&media_type()?)?;
            stream_descriptor.SetGUID(&MF_DEVICESTREAM_STREAM_CATEGORY, &PINNAME_VIDEO_CAPTURE)?;
            stream_descriptor.SetUINT32(&MF_DEVICESTREAM_STREAM_ID, 0)?;
            stream_descriptor.SetUINT32(&MF_DEVICESTREAM_FRAMESERVER_SHARED, 1)?;
            stream_descriptor.SetUINT32(
                &MF_DEVICESTREAM_ATTRIBUTE_FRAMESOURCE_TYPES,
                MFFrameSourceTypes_Color.0 as u32,
            )?;
            let descriptor =
                MFCreatePresentationDescriptor(Some(&[Some(stream_descriptor.clone())]))?;
            descriptor.SelectStream(0)?;

            let mut attributes = None;
            MFCreateAttributes(&mut attributes, 1)?;
            let source: IMFMediaSourceEx = Source {
                events: MFCreateEventQueue()?,
                attributes: attributes.ok_or_else(windows::core::Error::empty)?,
                descriptor,
                stream: OnceLock::new(),
                shut_down: AtomicBool::new(false),
            }
            .into();
            let stream = Stream::create(source.cast()?, stream_descriptor)?;
            let _ = source.as_impl().stream.set(stream);
            Ok(source)
        }
    }

    fn check(&self) -> Result<()> {
        if self.shut_down.load(Ordering::Acquire) {
            return Err(MF_E_SHUTDOWN.into());
        }
        Ok(())
    }

    fn stream(&self) -> Result<(&IMFMediaStream2, &Stream)> {
        let stream = self
            .stream
            .get()
            .ok_or_else(|| windows::core::Error::from(MF_E_SHUTDOWN))?;
        Ok((stream, unsafe { stream.as_impl() }))
    }
}

impl IMFMediaEventGenerator_Impl for Source_Impl {
    fn GetEvent(&self, flags: MEDIA_EVENT_GENERATOR_GET_EVENT_FLAGS) -> Result<IMFMediaEvent> {
        unsafe { self.events.GetEvent(flags.0) }
    }

    fn BeginGetEvent(&self, callback: Ref<IMFAsyncCallback>, state: Ref<IUnknown>) -> Result<()> {
        unsafe { self.events.BeginGetEvent(callback.as_ref(), state.as_ref()) }
    }

    fn EndGetEvent(&self, result: Ref<IMFAsyncResult>) -> Result<IMFMediaEvent> {
        unsafe { self.events.EndGetEvent(result.as_ref()) }
    }

    fn QueueEvent(
        &self,
        event: u32,
        extended: *const GUID,
        status: HRESULT,
        value: *const PROPVARIANT,
    ) -> Result<()> {
        unsafe {
            self.events
                .QueueEventParamVar(event, extended, status, value)
        }
    }
}

impl IMFMediaSource_Impl for Source_Impl {
    fn GetCharacteristics(&self) -> Result<u32> {
        self.check()?;
        Ok(MFMEDIASOURCE_IS_LIVE.0 as u32)
    }

    fn CreatePresentationDescriptor(&self) -> Result<IMFPresentationDescriptor> {
        self.check()?;
        unsafe { self.descriptor.Clone() }
    }

    fn Start(
        &self,
        descriptor: Ref<IMFPresentationDescriptor>,
        _: *const GUID,
        _: *const PROPVARIANT,
    ) -> Result<()> {
        self.check()?;
        let (stream, inner) = self.stream()?;
        let mut selected = BOOL::default();
        let mut stream_descriptor = None;
        unsafe {
            descriptor
                .ok()?
                .GetStreamDescriptorByIndex(0, &mut selected, &mut stream_descriptor)?
        };
        if selected.as_bool() {
            let event = if inner.running() {
                MEUpdatedStream
            } else {
                MENewStream
            };
            queue_event(&self.events, event, Some(&stream.cast()?))?;
            inner.start()?;
        }
        queue_event(&self.events, MESourceStarted, None)
    }

    fn Stop(&self) -> Result<()> {
        self.check()?;
        self.stream()?.1.stop()?;
        queue_event(&self.events, MESourceStopped, None)
    }

    /// A live camera can't hold its place.
    fn Pause(&self) -> Result<()> {
        Err(MF_E_INVALID_STATE_TRANSITION.into())
    }

    fn Shutdown(&self) -> Result<()> {
        self.shut_down.store(true, Ordering::Release);
        if let Ok((_, stream)) = self.stream() {
            stream.shut_down();
        }
        unsafe { self.events.Shutdown() }
    }
}

impl IMFMediaSourceEx_Impl for Source_Impl {
    fn GetSourceAttributes(&self) -> Result<IMFAttributes> {
        self.check()?;
        Ok(self.attributes.clone())
    }

    fn GetStreamAttributes(&self, stream: u32) -> Result<IMFAttributes> {
        self.check()?;
        if stream != 0 {
            return Err(MF_E_INVALIDSTREAMNUMBER.into());
        }
        self.stream()?.1.descriptor.cast()
    }

    /// Frames are in plain memory, so a graphics device isn't used.
    fn SetD3DManager(&self, _: Ref<IUnknown>) -> Result<()> {
        Ok(())
    }
}

impl IMFGetService_Impl for Source_Impl {
    fn GetService(&self, _: *const GUID, _: *const GUID, _: *mut *mut c_void) -> Result<()> {
        Err(MF_E_UNSUPPORTED_SERVICE.into())
    }
}

impl IKsControl_Impl for Source_Impl {
    fn KsProperty(
        &self,
        _: *const KSIDENTIFIER,
        _: u32,
        _: *mut c_void,
        _: u32,
        _: *mut u32,
    ) -> Result<()> {
        Err(NOT_SUPPORTED.into())
    }

    fn KsMethod(
        &self,
        _: *const KSIDENTIFIER,
        _: u32,
        _: *mut c_void,
        _: u32,
        _: *mut u32,
    ) -> Result<()> {
        Err(NOT_SUPPORTED.into())
    }

    fn KsEvent(
        &self,
        _: *const KSIDENTIFIER,
        _: u32,
        _: *mut c_void,
        _: u32,
        _: *mut u32,
    ) -> Result<()> {
        Err(NOT_SUPPORTED.into())
    }
}

#[implement(IMFMediaStream2)]
struct Stream {
    /// Cleared on shutdown, which breaks the source and stream holding each other.
    source: Mutex<Option<IMFMediaSource>>,
    descriptor: IMFStreamDescriptor,
    events: IMFMediaEventQueue,
    state: Mutex<MF_STREAM_STATE>,
    /// Made when the stream starts, so the app can find it while the camera is in use.
    frames: Mutex<Option<SharedFrame>>,
    last_sample: Mutex<Option<Instant>>,
}

impl Stream {
    fn create(source: IMFMediaSource, descriptor: IMFStreamDescriptor) -> Result<IMFMediaStream2> {
        Ok(Stream {
            source: Mutex::new(Some(source)),
            descriptor,
            events: unsafe { MFCreateEventQueue()? },
            state: Mutex::new(MF_STREAM_STATE_STOPPED),
            frames: Mutex::new(None),
            last_sample: Mutex::new(None),
        }
        .into())
    }

    fn running(&self) -> bool {
        *self.state.lock().unwrap() == MF_STREAM_STATE_RUNNING
    }

    fn start(&self) -> Result<()> {
        *self.state.lock().unwrap() = MF_STREAM_STATE_RUNNING;
        let mut frames = self.frames.lock().unwrap();
        if frames.is_none() {
            *frames = SharedFrame::create();
        }
        queue_event(&self.events, MEStreamStarted, None)
    }

    fn stop(&self) -> Result<()> {
        *self.state.lock().unwrap() = MF_STREAM_STATE_STOPPED;
        queue_event(&self.events, MEStreamStopped, None)
    }

    fn shut_down(&self) {
        *self.state.lock().unwrap() = MF_STREAM_STATE_STOPPED;
        self.source.lock().unwrap().take();
        self.frames.lock().unwrap().take();
        unsafe {
            let _ = self.events.Shutdown();
        }
    }

    /// The app's newest frame, or a dark one before it sends any.
    fn sample(&self) -> Result<IMFSample> {
        unsafe {
            let buffer = MFCreateMemoryBuffer(FRAME_BYTES as u32)?;
            let mut data = std::ptr::null_mut();
            buffer.Lock(&mut data, None, None)?;
            let frame = std::slice::from_raw_parts_mut(data, FRAME_BYTES);
            let filled = self
                .frames
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|shared| shared.read(frame));
            if !filled {
                let (brightness, colour) = frame.split_at_mut(WIDTH * HEIGHT);
                brightness.fill(16);
                colour.fill(128);
            }
            buffer.Unlock()?;
            buffer.SetCurrentLength(FRAME_BYTES as u32)?;
            let sample = MFCreateSample()?;
            sample.AddBuffer(&buffer)?;
            sample.SetSampleTime(MFGetSystemTime())?;
            sample.SetSampleDuration(FRAME_TICKS)?;
            Ok(sample)
        }
    }

    /// Keeps to the frame rate when samples are asked for faster.
    fn wait_for_next_frame(&self) {
        let mut last = self.last_sample.lock().unwrap();
        if let Some(elapsed) = last.map(|at| at.elapsed()) {
            if elapsed < FRAME_TIME {
                std::thread::sleep(FRAME_TIME - elapsed);
            }
        }
        *last = Some(Instant::now());
    }
}

impl IMFMediaEventGenerator_Impl for Stream_Impl {
    fn GetEvent(&self, flags: MEDIA_EVENT_GENERATOR_GET_EVENT_FLAGS) -> Result<IMFMediaEvent> {
        unsafe { self.events.GetEvent(flags.0) }
    }

    fn BeginGetEvent(&self, callback: Ref<IMFAsyncCallback>, state: Ref<IUnknown>) -> Result<()> {
        unsafe { self.events.BeginGetEvent(callback.as_ref(), state.as_ref()) }
    }

    fn EndGetEvent(&self, result: Ref<IMFAsyncResult>) -> Result<IMFMediaEvent> {
        unsafe { self.events.EndGetEvent(result.as_ref()) }
    }

    fn QueueEvent(
        &self,
        event: u32,
        extended: *const GUID,
        status: HRESULT,
        value: *const PROPVARIANT,
    ) -> Result<()> {
        unsafe {
            self.events
                .QueueEventParamVar(event, extended, status, value)
        }
    }
}

impl IMFMediaStream_Impl for Stream_Impl {
    fn GetMediaSource(&self) -> Result<IMFMediaSource> {
        self.source
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| MF_E_SHUTDOWN.into())
    }

    fn GetStreamDescriptor(&self) -> Result<IMFStreamDescriptor> {
        Ok(self.descriptor.clone())
    }

    fn RequestSample(&self, token: Ref<IUnknown>) -> Result<()> {
        if self.source.lock().unwrap().is_none() {
            return Err(MF_E_SHUTDOWN.into());
        }
        if !self.running() {
            return Err(MF_E_INVALID_STATE_TRANSITION.into());
        }
        self.wait_for_next_frame();
        let sample = self.sample()?;
        if let Some(token) = token.as_ref() {
            unsafe { sample.SetUnknown(&MFSampleExtension_Token, token)? };
        }
        queue_event(&self.events, MEMediaSample, Some(&sample.cast()?))
    }
}

impl IMFMediaStream2_Impl for Stream_Impl {
    fn SetStreamState(&self, state: MF_STREAM_STATE) -> Result<()> {
        *self.state.lock().unwrap() = state;
        Ok(())
    }

    fn GetStreamState(&self) -> Result<MF_STREAM_STATE> {
        Ok(*self.state.lock().unwrap())
    }
}

/// What the camera service creates from the registered class: it makes the source on request
/// and keeps attributes the service sets on it.
#[implement(IMFActivate)]
pub struct Activator {
    attributes: IMFAttributes,
    source: Mutex<Option<IMFMediaSourceEx>>,
}

impl Activator {
    pub fn create() -> Result<IMFActivate> {
        let mut attributes = None;
        unsafe { MFCreateAttributes(&mut attributes, 1)? };
        Ok(Activator {
            attributes: attributes.ok_or_else(windows::core::Error::empty)?,
            source: Mutex::new(None),
        }
        .into())
    }
}

impl IMFActivate_Impl for Activator_Impl {
    fn ActivateObject(&self, iid: *const GUID, object: *mut *mut c_void) -> Result<()> {
        let mut source = self.source.lock().unwrap();
        if source.is_none() {
            *source = Some(Source::create()?);
        }
        let source = source.as_ref().ok_or_else(windows::core::Error::empty)?;
        unsafe { source.query(iid, object).ok() }
    }

    fn ShutdownObject(&self) -> Result<()> {
        if let Some(source) = self.source.lock().unwrap().take() {
            unsafe { source.Shutdown()? };
        }
        Ok(())
    }

    fn DetachObject(&self) -> Result<()> {
        self.source.lock().unwrap().take();
        Ok(())
    }
}

/// Raw calls for the three methods whose safe wrappers reshape their arguments.
macro_rules! raw {
    ($attributes:expr, $method:ident($($arg:expr),*)) => {
        unsafe { (Interface::vtable($attributes).$method)(Interface::as_raw($attributes), $($arg),*).ok() }
    };
}

impl IMFAttributes_Impl for Activator_Impl {
    fn GetItem(&self, key: *const GUID, value: *mut PROPVARIANT) -> Result<()> {
        unsafe { self.attributes.GetItem(key, Some(value)) }
    }

    fn GetItemType(&self, key: *const GUID) -> Result<MF_ATTRIBUTE_TYPE> {
        unsafe { self.attributes.GetItemType(key) }
    }

    fn CompareItem(&self, key: *const GUID, value: *const PROPVARIANT) -> Result<BOOL> {
        unsafe { self.attributes.CompareItem(key, value) }
    }

    fn Compare(
        &self,
        theirs: Ref<IMFAttributes>,
        match_type: MF_ATTRIBUTES_MATCH_TYPE,
    ) -> Result<BOOL> {
        unsafe { self.attributes.Compare(theirs.as_ref(), match_type) }
    }

    fn GetUINT32(&self, key: *const GUID) -> Result<u32> {
        unsafe { self.attributes.GetUINT32(key) }
    }

    fn GetUINT64(&self, key: *const GUID) -> Result<u64> {
        unsafe { self.attributes.GetUINT64(key) }
    }

    fn GetDouble(&self, key: *const GUID) -> Result<f64> {
        unsafe { self.attributes.GetDouble(key) }
    }

    fn GetGUID(&self, key: *const GUID) -> Result<GUID> {
        unsafe { self.attributes.GetGUID(key) }
    }

    fn GetStringLength(&self, key: *const GUID) -> Result<u32> {
        unsafe { self.attributes.GetStringLength(key) }
    }

    fn GetString(&self, key: *const GUID, value: PWSTR, size: u32, length: *mut u32) -> Result<()> {
        raw!(&self.attributes, GetString(key, value, size, length))
    }

    fn GetAllocatedString(
        &self,
        key: *const GUID,
        value: *mut PWSTR,
        length: *mut u32,
    ) -> Result<()> {
        unsafe { self.attributes.GetAllocatedString(key, value, length) }
    }

    fn GetBlobSize(&self, key: *const GUID) -> Result<u32> {
        unsafe { self.attributes.GetBlobSize(key) }
    }

    fn GetBlob(
        &self,
        key: *const GUID,
        buffer: *mut u8,
        size: u32,
        length: *mut u32,
    ) -> Result<()> {
        raw!(&self.attributes, GetBlob(key, buffer, size, length))
    }

    fn GetAllocatedBlob(
        &self,
        key: *const GUID,
        buffer: *mut *mut u8,
        size: *mut u32,
    ) -> Result<()> {
        unsafe { self.attributes.GetAllocatedBlob(key, buffer, size) }
    }

    fn GetUnknown(
        &self,
        key: *const GUID,
        iid: *const GUID,
        object: *mut *mut c_void,
    ) -> Result<()> {
        raw!(&self.attributes, GetUnknown(key, iid, object))
    }

    fn SetItem(&self, key: *const GUID, value: *const PROPVARIANT) -> Result<()> {
        unsafe { self.attributes.SetItem(key, value) }
    }

    fn DeleteItem(&self, key: *const GUID) -> Result<()> {
        unsafe { self.attributes.DeleteItem(key) }
    }

    fn DeleteAllItems(&self) -> Result<()> {
        unsafe { self.attributes.DeleteAllItems() }
    }

    fn SetUINT32(&self, key: *const GUID, value: u32) -> Result<()> {
        unsafe { self.attributes.SetUINT32(key, value) }
    }

    fn SetUINT64(&self, key: *const GUID, value: u64) -> Result<()> {
        unsafe { self.attributes.SetUINT64(key, value) }
    }

    fn SetDouble(&self, key: *const GUID, value: f64) -> Result<()> {
        unsafe { self.attributes.SetDouble(key, value) }
    }

    fn SetGUID(&self, key: *const GUID, value: *const GUID) -> Result<()> {
        unsafe { self.attributes.SetGUID(key, value) }
    }

    fn SetString(&self, key: *const GUID, value: &PCWSTR) -> Result<()> {
        unsafe { self.attributes.SetString(key, *value) }
    }

    fn SetBlob(&self, key: *const GUID, buffer: *const u8, size: u32) -> Result<()> {
        let blob = if buffer.is_null() {
            &[][..]
        } else {
            unsafe { std::slice::from_raw_parts(buffer, size as usize) }
        };
        unsafe { self.attributes.SetBlob(key, blob) }
    }

    fn SetUnknown(&self, key: *const GUID, value: Ref<IUnknown>) -> Result<()> {
        unsafe { self.attributes.SetUnknown(key, value.as_ref()) }
    }

    fn LockStore(&self) -> Result<()> {
        unsafe { self.attributes.LockStore() }
    }

    fn UnlockStore(&self) -> Result<()> {
        unsafe { self.attributes.UnlockStore() }
    }

    fn GetCount(&self) -> Result<u32> {
        unsafe { self.attributes.GetCount() }
    }

    fn GetItemByIndex(&self, index: u32, key: *mut GUID, value: *mut PROPVARIANT) -> Result<()> {
        unsafe { self.attributes.GetItemByIndex(index, key, Some(value)) }
    }

    fn CopyAllItems(&self, destination: Ref<IMFAttributes>) -> Result<()> {
        unsafe { self.attributes.CopyAllItems(destination.as_ref()) }
    }
}
