// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef, useState } from "react";
import { Channel } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { errorMessage } from "./api.ts";

/** Any H.264 profile the phone's encoder picks decodes under this one. */
const CODEC = "avc1.640028";

/** The phone's sound is 16-bit stereo at this rate. */
const SAMPLE_RATE = 48000;
/** Sound plays this far behind, so small gaps in delivery aren't heard. */
const SOUND_DELAY_S = 0.08;
/** Further behind than this, sound skips ahead to stay with the picture. */
const MOST_BEHIND_S = 0.5;

export interface VideoFeed {
  start: (frames: Channel<ArrayBuffer>) => Promise<unknown>;
  stop: () => Promise<void>;
  onEnded: (handler: () => void) => Promise<UnlistenFn>;
}

/**
 * Each message is a byte saying what it is (0 a frame, 1 a key frame, 2 sound), the quarter turns
 * that show a frame upright, then H.264 or the sound's PCM.
 */
export function useVideo(feed: VideoFeed, onClose: () => void, onError: (message: string) => void) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [showing, setShowing] = useState(false);
  const [muted, setMuted] = useState(false);
  const volume = useRef<GainNode | null>(null);
  const { start, stop, onEnded } = feed;

  useEffect(() => {
    if (volume.current) volume.current.gain.value = muted ? 0 : 1;
  }, [muted]);

  useEffect(() => {
    const sound = new AudioContext({ sampleRate: SAMPLE_RATE, latencyHint: "interactive" });
    const gain = sound.createGain();
    gain.connect(sound.destination);
    volume.current = gain;
    let playAt = 0;
    const play = (pcm: Uint8Array) => {
      const samples = new Int16Array(pcm.buffer, pcm.byteOffset, Math.floor(pcm.byteLength / 4) * 2);
      const length = samples.length / 2;
      if (length === 0) return;
      const buffer = sound.createBuffer(2, length, SAMPLE_RATE);
      const [left, right] = [buffer.getChannelData(0), buffer.getChannelData(1)];
      for (let i = 0; i < length; i++) {
        left[i] = samples[2 * i] / 32768;
        right[i] = samples[2 * i + 1] / 32768;
      }
      const source = sound.createBufferSource();
      source.buffer = buffer;
      source.connect(gain);
      if (playAt < sound.currentTime || playAt > sound.currentTime + MOST_BEHIND_S) {
        playAt = sound.currentTime + SOUND_DELAY_S;
      }
      source.start(playAt);
      playAt += buffer.duration;
    };

    let waitingForKey = true;
    let turns = 0;
    const decoder = new VideoDecoder({
      output: (frame) => {
        const target = canvas.current;
        const context = target?.getContext("2d");
        if (target && context) {
          const sideways = turns % 2 === 1;
          target.width = sideways ? frame.displayHeight : frame.displayWidth;
          target.height = sideways ? frame.displayWidth : frame.displayHeight;
          context.setTransform(1, 0, 0, 1, 0, 0);
          context.translate(target.width / 2, target.height / 2);
          context.rotate((turns * Math.PI) / 2);
          context.drawImage(frame, -frame.displayWidth / 2, -frame.displayHeight / 2);
          setShowing(true);
        }
        frame.close();
      },
      error: (error) => onError(error.message),
    });
    decoder.configure({ codec: CODEC, optimizeForLatency: true });

    const frames = new Channel<ArrayBuffer>();
    frames.onmessage = (message) => {
      const bytes = new Uint8Array(message);
      if (bytes[0] === 2) {
        play(bytes.subarray(2));
        return;
      }
      const key = bytes[0] === 1;
      // Behind? Skip ahead to the next key frame rather than fall further back.
      if (!key && (waitingForKey || decoder.decodeQueueSize > 2)) {
        waitingForKey = true;
        return;
      }
      waitingForKey = false;
      if (key) turns = bytes[1];
      const chunk = {
        type: key ? "key" : "delta",
        timestamp: performance.now() * 1000,
        data: bytes.subarray(2),
      } as const;
      decoder.decode(new EncodedVideoChunk(chunk));
    };

    start(frames).catch((error) => {
      onError(errorMessage(error));
      onClose();
    });
    const ended = onEnded(onClose);
    return () => {
      decoder.close();
      void sound.close();
      volume.current = null;
      void stop();
      void ended.then((unlisten) => unlisten());
    };
  }, [start, stop, onEnded, onClose, onError]);

  return { canvas, showing, muted, setMuted };
}
