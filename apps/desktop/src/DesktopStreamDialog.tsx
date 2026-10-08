// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef, useState, useCallback } from "react";
import { Loader, Maximize2, Minimize2, Monitor, RefreshCw, Square, X } from "lucide-react";
import {
  errorMessage,
  onDesktopStreamFrame,
  onDesktopStreamStopped,
  sendDesktopInput,
  startDesktopStream,
  stopDesktopStream,
} from "./api.ts";
import type { DesktopInputEventDto, DesktopStreamInfo, TrustedPeer } from "./types.ts";

interface DesktopStreamDialogProps {
  peer: TrustedPeer;
  onClose: () => void;
  onError: (message: string) => void;
}

const PRESETS = [
  { label: "1080p (FHD)", width: 1920, height: 1080, dpi: 240 },
  { label: "720p (HD)", width: 1280, height: 720, dpi: 160 },
  { label: "Phone Native", width: 1080, height: 2400, dpi: 420 },
];

export function DesktopStreamDialog({ peer, onClose, onError }: DesktopStreamDialogProps) {
  const [session, setSession] = useState<DesktopStreamInfo | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [presetIndex, setPresetIndex] = useState(0);
  const [fps, setFps] = useState(0);
  const [frameData, setFrameData] = useState<string | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const viewportRef = useRef<HTMLDivElement>(null);
  const sessionRef = useRef<DesktopStreamInfo | null>(null);
  sessionRef.current = session;

  const frameCounterRef = useRef(0);
  const lastFpsCalcRef = useRef(Date.now());

  const stopActiveStream = useCallback(async () => {
    const cur = sessionRef.current;
    if (cur) {
      try {
        await stopDesktopStream(cur.sessionId);
      } catch (err) {
        console.error("Failed to stop stream:", err);
      }
    }
  }, []);

  const handleClose = useCallback(async () => {
    await stopActiveStream();
    onClose();
  }, [stopActiveStream, onClose]);

  const startStream = useCallback(async () => {
    setConnecting(true);
    const preset = PRESETS[presetIndex];
    try {
      const info = await startDesktopStream(
        peer.fingerprint,
        undefined,
        preset.width,
        preset.height,
        preset.dpi,
      );
      setSession(info);
      setFrameData(null);
      frameCounterRef.current = 0;
      lastFpsCalcRef.current = Date.now();
    } catch (err) {
      onError(errorMessage(err));
    } finally {
      setConnecting(false);
    }
  }, [peer.fingerprint, presetIndex, onError]);

  useEffect(() => {
    startStream();
    return () => {
      stopActiveStream();
    };
  }, [startStream, stopActiveStream]);

  useEffect(() => {
    let unlistenFrame: (() => void) | null = null;
    let unlistenStopped: (() => void) | null = null;

    onDesktopStreamFrame((frame) => {
      if (sessionRef.current && frame.sessionId === sessionRef.current.sessionId) {
        setFrameData(`data:image/jpeg;base64,${frame.data}`);
        frameCounterRef.current += 1;

        const now = Date.now();
        const elapsed = now - lastFpsCalcRef.current;
        if (elapsed >= 1000) {
          setFps(Math.round((frameCounterRef.current * 1000) / elapsed));
          frameCounterRef.current = 0;
          lastFpsCalcRef.current = now;
        }
      }
    }).then((un) => {
      unlistenFrame = un;
    });

    onDesktopStreamStopped((stoppedId) => {
      if (sessionRef.current && stoppedId === sessionRef.current.sessionId) {
        setSession(null);
        setFrameData(null);
      }
    }).then((un) => {
      unlistenStopped = un;
    });

    return () => {
      if (unlistenFrame) unlistenFrame();
      if (unlistenStopped) unlistenStopped();
    };
  }, []);

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (isFullscreen) {
          setIsFullscreen(false);
        } else {
          handleClose();
        }
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [isFullscreen, handleClose]);

  const mapCoordinates = (clientX: number, clientY: number): { x: number; y: number } | null => {
    const el = viewportRef.current;
    const cur = sessionRef.current;
    if (!el || !cur) return null;
    const rect = el.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return null;

    const relX = Math.max(0, Math.min(clientX - rect.left, rect.width));
    const relY = Math.max(0, Math.min(clientY - rect.top, rect.height));

    const targetWidth = cur.width || 1920;
    const targetHeight = cur.height || 1080;

    return {
      x: Math.round((relX / rect.width) * targetWidth),
      y: Math.round((relY / rect.height) * targetHeight),
    };
  };

  const dispatchInput = (
    eventType: DesktopInputEventDto["eventType"],
    clientX: number,
    clientY: number,
    opts: Partial<DesktopInputEventDto> = {},
  ) => {
    const cur = sessionRef.current;
    if (!cur) return;
    const coords = mapCoordinates(clientX, clientY);
    if (!coords) return;

    sendDesktopInput({
      sessionId: cur.sessionId,
      eventType,
      x: coords.x,
      y: coords.y,
      ...opts,
    }).catch(() => {});
  };

  const handlePointerDown = (e: React.PointerEvent) => {
    viewportRef.current?.focus();
    dispatchInput("pointer_down", e.clientX, e.clientY, { button: e.button });
  };

  const handlePointerUp = (e: React.PointerEvent) => {
    dispatchInput("pointer_up", e.clientX, e.clientY, { button: e.button });
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (e.buttons > 0) {
      dispatchInput("pointer_move", e.clientX, e.clientY, { button: e.button });
    }
  };

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    dispatchInput("scroll", e.clientX, e.clientY, {
      scrollDx: Math.round(e.deltaX),
      scrollDy: Math.round(e.deltaY),
    });
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") return;
    const cur = sessionRef.current;
    if (!cur) return;
    sendDesktopInput({
      sessionId: cur.sessionId,
      eventType: "key_down",
      x: 0,
      y: 0,
      keyCode: e.keyCode,
      keyText: e.key,
    }).catch(() => {});
  };

  const handleKeyUp = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") return;
    const cur = sessionRef.current;
    if (!cur) return;
    sendDesktopInput({
      sessionId: cur.sessionId,
      eventType: "key_up",
      x: 0,
      y: 0,
      keyCode: e.keyCode,
      keyText: e.key,
    }).catch(() => {});
  };

  return (
    <div className="scrim" onMouseDown={handleClose}>
      <div
        ref={containerRef}
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="desktop-stream-title"
        onMouseDown={(e) => e.stopPropagation()}
        style={{
          width: isFullscreen ? "98vw" : "min(960px, 96vw)",
          maxWidth: "none",
          height: isFullscreen ? "96vh" : "auto",
        }}
      >
        <header className="dialog-header">
          <div>
            <h2 id="desktop-stream-title" className="dialog-title" style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Monitor size={22} />
              Desktop Mode: {peer.displayName}
            </h2>
            <p className="dialog-subtitle">
              Interactive Android desktop workspace and app streaming.
            </p>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <button
              type="button"
              className="icon-btn"
              onClick={() => setIsFullscreen(!isFullscreen)}
              aria-label={isFullscreen ? "Exit full size" : "Expand size"}
            >
              {isFullscreen ? <Minimize2 size={18} /> : <Maximize2 size={18} />}
            </button>
            <button type="button" className="icon-btn" onClick={handleClose} aria-label="Close">
              <X size={20} />
            </button>
          </div>
        </header>

        <div className="toolbar" style={{ justifyContent: "space-between", flexWrap: "wrap", gap: 8 }}>
          <div className="chips" role="tablist" aria-label="Resolution presets">
            {PRESETS.map((p, idx) => (
              <button
                key={p.label}
                type="button"
                role="tab"
                aria-selected={presetIndex === idx}
                className="chip"
                onClick={() => {
                  setPresetIndex(idx);
                }}
                disabled={connecting}
              >
                {p.label}
              </button>
            ))}
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
            {session && (
              <span className="mono text-muted">
                {session.width}x{session.height} @ {fps} FPS
              </span>
            )}
            {session ? (
              <button
                type="button"
                className="btn btn-tonal btn-small"
                onClick={stopActiveStream}
                title="Stop current session"
              >
                <Square size={14} /> Stop
              </button>
            ) : (
              <button
                type="button"
                className="btn btn-primary btn-small"
                onClick={startStream}
                disabled={connecting}
              >
                {connecting ? <Loader size={14} className="spin" /> : <RefreshCw size={14} />}
                {connecting ? "Starting..." : "Start Stream"}
              </button>
            )}
          </div>
        </div>

        <div
          ref={viewportRef}
          tabIndex={0}
          onPointerDown={handlePointerDown}
          onPointerUp={handlePointerUp}
          onPointerMove={handlePointerMove}
          onWheel={handleWheel}
          onKeyDown={handleKeyDown}
          onKeyUp={handleKeyUp}
          onContextMenu={(e) => e.preventDefault()}
          style={{
            position: "relative",
            width: "100%",
            aspectRatio: session ? `${session.width} / ${session.height}` : "16 / 9",
            maxHeight: isFullscreen ? "calc(88vh - 120px)" : "60vh",
            backgroundColor: "#0d1117",
            borderRadius: 16,
            overflow: "hidden",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            cursor: session ? "crosshair" : "default",
            outline: "none",
            border: "1px solid var(--line)",
            userSelect: "none",
          }}
        >
          {frameData ? (
            <img
              src={frameData}
              alt="Remote Desktop Workspace"
              style={{
                width: "100%",
                height: "100%",
                objectFit: "contain",
                pointerEvents: "none",
              }}
            />
          ) : connecting ? (
            <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 12, color: "var(--text-2)" }}>
              <Loader size={32} className="spin" />
              <span>Initializing remote desktop display...</span>
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 12, color: "var(--text-2)" }}>
              <Monitor size={36} opacity={0.6} />
              <span>Desktop stream inactive. Click Start to begin.</span>
            </div>
          )}
        </div>

        <footer className="dialog-actions" style={{ justifyContent: "space-between" }}>
          <span className="text-muted" style={{ fontSize: 13 }}>
            Forwarding keyboard, mouse, and touch input directly to remote Android session.
          </span>
          <button type="button" className="btn btn-tonal" onClick={handleClose}>
            Close
          </button>
        </footer>
      </div>
    </div>
  );
}
