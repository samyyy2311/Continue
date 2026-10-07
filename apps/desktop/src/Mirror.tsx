// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useMemo, useRef } from "react";
import { ArrowLeft, Circle, Square, Volume2, VolumeX, X } from "lucide-react";
import type { Channel } from "@tauri-apps/api/core";
import { errorMessage, onMirrorEnded, screenInput, startMirror, stopMirror, type ScreenInput } from "./api.ts";
import { useVideo } from "./useVideo.ts";

/** Moves sent more often than this pile up on the phone faster than it can draw them. */
const MOVE_EVERY_MS = 16;
const SCROLL_STEP = 0.25;
const SCROLL_MS = 150;

export function Mirror(props: { peer: string; onClose: () => void; onError: (message: string) => void }) {
  const { peer, onClose, onError } = props;
  const feed = useMemo(
    () => ({
      start: (frames: Channel<ArrayBuffer>) => startMirror(peer, frames),
      stop: stopMirror,
      onEnded: onMirrorEnded,
    }),
    [peer],
  );
  const { canvas, showing, muted, setMuted } = useVideo(feed, onClose, onError);
  const lastMove = useRef(0);

  useEffect(() => {
    const type = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      if (e.key === "Backspace") send({ kind: "backspace" });
      else if (e.key === "Enter") send({ kind: "enter" });
      else if (e.key.length === 1) send({ kind: "text", text: e.key });
      else return;
      e.preventDefault();
    };
    window.addEventListener("keydown", type);
    return () => window.removeEventListener("keydown", type);
  });

  const send = (input: ScreenInput) => void screenInput(input).catch((error) => onError(errorMessage(error)));
  const at = (e: React.PointerEvent | React.WheelEvent) => {
    const box = e.currentTarget.getBoundingClientRect();
    return { x: (e.clientX - box.left) / box.width, y: (e.clientY - box.top) / box.height };
  };

  return (
    <div className="mirror" role="dialog" aria-label="Phone screen">
      <div className="mirror-phone">
        <button type="button" className="icon-btn mirror-close" aria-label="Close" onClick={onClose}>
          <X size={20} />
        </button>
        <canvas
          ref={canvas}
          className={`mirror-screen ${showing ? "" : "waiting"}`}
          onPointerDown={(e) => {
            e.currentTarget.setPointerCapture(e.pointerId);
            lastMove.current = performance.now();
            send({ kind: "down", ...at(e) });
          }}
          onPointerMove={(e) => {
            if (!e.currentTarget.hasPointerCapture(e.pointerId)) return;
            const now = performance.now();
            if (now - lastMove.current < MOVE_EVERY_MS) return;
            lastMove.current = now;
            send({ kind: "move", ...at(e) });
          }}
          onPointerUp={(e) => send({ kind: "up", ...at(e) })}
          onWheel={(e) => {
            const from = at(e);
            const toY = from.y - Math.sign(e.deltaY) * SCROLL_STEP;
            send({ kind: "swipe", fromX: from.x, fromY: from.y, toX: from.x, toY, durationMs: SCROLL_MS });
          }}
        />
        {!showing && <p className="supporting mirror-note">Allow sharing on your phone to see it here.</p>}
        <div className="mirror-buttons">
          <button
            type="button"
            className="icon-btn"
            aria-label={muted ? "Play the phone's sound" : "Mute the phone's sound"}
            aria-pressed={muted}
            onClick={() => setMuted(!muted)}
          >
            {muted ? <VolumeX size={20} /> : <Volume2 size={20} />}
          </button>
          <button type="button" className="icon-btn" aria-label="Back" onClick={() => send({ kind: "back" })}>
            <ArrowLeft size={20} />
          </button>
          <button type="button" className="icon-btn" aria-label="Home" onClick={() => send({ kind: "home" })}>
            <Circle size={18} />
          </button>
          <button type="button" className="icon-btn" aria-label="Recent apps" onClick={() => send({ kind: "recents" })}>
            <Square size={16} />
          </button>
        </div>
      </div>
    </div>
  );
}
