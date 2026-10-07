// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Decodes H.264 made by the encoder Windows ships, standing in for the phone's.

#![cfg(windows)]

use continue_camera::Decoder;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;
const GREY: u8 = 180;

fn encoder() -> IMFTransform {
    unsafe {
        let encoder: IMFTransform =
            CoCreateInstance(&CLSID_MSH264EncoderMFT, None, CLSCTX_INPROC_SERVER).unwrap();
        let size = u64::from(WIDTH) << 32 | u64::from(HEIGHT);
        let output = MFCreateMediaType().unwrap();
        output
            .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .unwrap();
        output.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264).unwrap();
        output.SetUINT64(&MF_MT_FRAME_SIZE, size).unwrap();
        output.SetUINT64(&MF_MT_FRAME_RATE, 30 << 32 | 1).unwrap();
        output.SetUINT32(&MF_MT_AVG_BITRATE, 1_000_000).unwrap();
        output
            .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
            .unwrap();
        encoder.SetOutputType(0, &output, 0).unwrap();
        let input = MFCreateMediaType().unwrap();
        input
            .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .unwrap();
        input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12).unwrap();
        input.SetUINT64(&MF_MT_FRAME_SIZE, size).unwrap();
        input.SetUINT64(&MF_MT_FRAME_RATE, 30 << 32 | 1).unwrap();
        input
            .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
            .unwrap();
        encoder.SetInputType(0, &input, 0).unwrap();
        encoder
    }
}

/// Encodes one grey frame and returns what the encoder had ready.
fn encode(encoder: &IMFTransform, index: i64) -> Vec<Vec<u8>> {
    unsafe {
        let bytes = (WIDTH * HEIGHT * 3 / 2) as usize;
        let buffer = MFCreateMemoryBuffer(bytes as u32).unwrap();
        let mut data = std::ptr::null_mut();
        buffer.Lock(&mut data, None, None).unwrap();
        let frame = std::slice::from_raw_parts_mut(data, bytes);
        frame[..(WIDTH * HEIGHT) as usize].fill(GREY);
        frame[(WIDTH * HEIGHT) as usize..].fill(128);
        buffer.Unlock().unwrap();
        buffer.SetCurrentLength(bytes as u32).unwrap();
        let sample = MFCreateSample().unwrap();
        sample.AddBuffer(&buffer).unwrap();
        sample.SetSampleTime(index * 333_333).unwrap();
        sample.SetSampleDuration(333_333).unwrap();
        encoder.ProcessInput(0, &sample, 0).unwrap();

        let mut encoded = Vec::new();
        loop {
            let size = encoder.GetOutputStreamInfo(0).unwrap().cbSize.max(1 << 20);
            let out = MFCreateSample().unwrap();
            out.AddBuffer(&MFCreateMemoryBuffer(size).unwrap()).unwrap();
            let mut output = [MFT_OUTPUT_DATA_BUFFER {
                dwStreamID: 0,
                pSample: std::mem::ManuallyDrop::new(Some(out)),
                dwStatus: 0,
                pEvents: std::mem::ManuallyDrop::new(None),
            }];
            let mut status = 0;
            let result = encoder.ProcessOutput(0, &mut output, &mut status);
            let out = std::mem::ManuallyDrop::take(&mut output[0].pSample).unwrap();
            drop(std::mem::ManuallyDrop::take(&mut output[0].pEvents));
            if result.is_err() {
                return encoded;
            }
            let buffer = out.ConvertToContiguousBuffer().unwrap();
            let mut length = 0;
            buffer.Lock(&mut data, None, Some(&mut length)).unwrap();
            encoded.push(std::slice::from_raw_parts(data, length as usize).to_vec());
            buffer.Unlock().unwrap();
        }
    }
}

#[test]
fn the_decoder_gives_back_the_picture_that_was_encoded() {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
        MFStartup(MF_VERSION, MFSTARTUP_FULL).unwrap();
    }
    let encoder = encoder();
    let mut decoder = Decoder::new().unwrap();
    for index in 0..60 {
        for chunk in encode(&encoder, index) {
            if let Some(picture) = decoder.decode(&chunk).unwrap() {
                assert_eq!(
                    (picture.width, picture.height),
                    (WIDTH as usize, HEIGHT as usize)
                );
                let middle =
                    picture.data[picture.stride * (HEIGHT as usize / 2) + WIDTH as usize / 2];
                assert!(middle.abs_diff(GREY) <= 4, "brightness {middle}");
                return;
            }
        }
    }
    panic!("the decoder never finished a picture");
}
