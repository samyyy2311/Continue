// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Live screen or camera video to a peer, with its controls coming back on the same stream.

use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use limits::MAX_FRAME_VIDEO_BYTES;
use prost::Message;
use protocol::v1::{
    CameraControl, CameraRequest, ScreenInput, ScreenRequest, VideoFrame, VideoStart,
};
use protocol::CapabilityId;
use tokio::sync::{mpsc, Notify};
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::off_runtime;
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// How many frames may wait for the network before new ones are dropped, about a second's worth.
const FRAME_QUEUE: usize = 30;

pub trait VideoSource<R, C>: Send + Sync {
    /// Starts handing frames to the `VideoFeed`. The stream's size, or None if the person
    /// declined or it can't start. Called off the async runtime, and may wait for them.
    fn start(&self, request: R) -> Option<VideoStart>;
    fn control(&self, control: C);
    fn stop(&self);
}

pub type ScreenSource = dyn VideoSource<ScreenRequest, ScreenInput>;
pub type CameraSource = dyn VideoSource<CameraRequest, CameraControl>;

#[derive(Clone, Default)]
pub struct VideoFeed(Arc<Mutex<Feed>>);

#[derive(Default)]
struct Feed {
    frames: Option<mpsc::Sender<VideoFrame>>,
    /// After a dropped frame, the ones that follow can't be decoded until the next key frame.
    waiting_for_key: bool,
}

impl VideoFeed {
    fn open(&self) -> mpsc::Receiver<VideoFrame> {
        let (frames, receiver) = mpsc::channel(FRAME_QUEUE);
        *self.0.lock().unwrap() = Feed {
            frames: Some(frames),
            waiting_for_key: false,
        };
        receiver
    }

    /// Hands over a frame. False when nobody is watching or the frame was dropped because the
    /// network is behind; the app should then make its next frame a key frame.
    pub fn push(&self, frame: VideoFrame) -> bool {
        let mut feed = self.0.lock().unwrap();
        let Some(frames) = &feed.frames else {
            return false;
        };
        if feed.waiting_for_key && !frame.key {
            return false;
        }
        let taken = frames.try_send(frame).is_ok();
        feed.waiting_for_key = !taken;
        taken
    }

    /// Hands over a slice of sound. Dropped when the network is behind, which costs a moment of
    /// sound rather than the pictures after it.
    pub fn push_audio(&self, pcm: Vec<u8>) -> bool {
        let feed = self.0.lock().unwrap();
        let frame = VideoFrame {
            audio: pcm,
            ..Default::default()
        };
        feed.frames
            .as_ref()
            .is_some_and(|frames| frames.try_send(frame).is_ok())
    }

    pub fn end(&self) {
        self.0.lock().unwrap().frames = None;
    }
}

pub(crate) async fn serve_video<R, C>(
    source: Option<Arc<dyn VideoSource<R, C>>>,
    feed: &VideoFeed,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError>
where
    R: Message + Default + Send + 'static,
    C: Message + Default + Send + 'static,
{
    let request: R = read_msg(&mut stream.recv_stream, MAX_FRAME_VIDEO_BYTES).await?;
    // Opened first, so the first key frame isn't missed.
    let mut frames = feed.open();
    let start = match source.clone() {
        Some(source) => off_runtime(move || source.start(request)).await,
        None => None,
    };
    let reply = start.unwrap_or_default();
    write_msg(&mut stream.send_stream, &reply, MAX_FRAME_VIDEO_BYTES).await?;
    let (Some(source), Some(_)) = (source, start) else {
        feed.end();
        return Ok(());
    };

    // Controls arrive on their own task, so a frame being written never holds one up.
    let watching = Arc::new(Notify::new());
    let gone = watching.clone();
    let controls = source.clone();
    let mut recv = stream.recv_stream;
    tokio::spawn(async move {
        while let Ok(control) = read_msg::<C>(&mut recv, MAX_FRAME_VIDEO_BYTES).await {
            let controls = controls.clone();
            tokio::task::spawn_blocking(move || controls.control(control));
        }
        gone.notify_one();
    });

    loop {
        tokio::select! {
            frame = frames.recv() => match frame {
                Some(frame) => write_msg(&mut stream.send_stream, &frame, MAX_FRAME_VIDEO_BYTES).await?,
                None => break,
            },
            () = watching.notified() => break,
        }
    }
    feed.end();
    tokio::task::spawn_blocking(move || source.stop());
    let _ = stream.send_stream.finish();
    Ok(())
}

pub struct VideoFrames(quinn::RecvStream);

impl VideoFrames {
    /// The next frame, or an error once the peer stops.
    pub async fn next(&mut self) -> Result<VideoFrame, TransportError> {
        read_msg(&mut self.0, MAX_FRAME_VIDEO_BYTES).await
    }
}

/// Dropping it stops watching.
pub struct VideoControl<C>(quinn::SendStream, PhantomData<C>);

impl<C: Message> VideoControl<C> {
    pub async fn send(&mut self, control: &C) -> Result<(), TransportError> {
        write_msg(&mut self.0, control, MAX_FRAME_VIDEO_BYTES).await
    }
}

impl<C> Drop for VideoControl<C> {
    fn drop(&mut self) {
        let _ = self.0.finish();
    }
}

pub type Watched<C> = (VideoStart, VideoFrames, VideoControl<C>);

impl SessionMultiplexer {
    /// None if the peer declined.
    pub async fn watch_screen(
        &self,
        request: ScreenRequest,
    ) -> Result<Option<Watched<ScreenInput>>, TransportError> {
        self.watch(CapabilityId::SCREEN, &request).await
    }

    /// None if the peer declined.
    pub async fn watch_camera(
        &self,
        request: CameraRequest,
    ) -> Result<Option<Watched<CameraControl>>, TransportError> {
        self.watch(CapabilityId::CAMERA, &request).await
    }

    async fn watch<R: Message, C>(
        &self,
        capability: CapabilityId,
        request: &R,
    ) -> Result<Option<Watched<C>>, TransportError> {
        let (mut send, mut recv) = self.open_stream(capability).await?;
        write_msg(&mut send, request, MAX_FRAME_VIDEO_BYTES).await?;
        let start: VideoStart = read_msg(&mut recv, MAX_FRAME_VIDEO_BYTES).await?;
        Ok(start
            .started
            .then(|| (start, VideoFrames(recv), VideoControl(send, PhantomData))))
    }
}
