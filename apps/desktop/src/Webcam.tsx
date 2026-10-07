// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useMemo, useState } from "react";
import { ScanFace, SwitchCamera, Video, X } from "lucide-react";
import type { Channel } from "@tauri-apps/api/core";
import {
  callCameraAvailable,
  cameraControl,
  errorMessage,
  onCameraEnded,
  setCallCamera,
  startCamera,
  stopCamera,
} from "./api.ts";
import { useVideo } from "./useVideo.ts";

export function Webcam(props: { peer: string; onClose: () => void; onError: (message: string) => void }) {
  const { peer, onClose, onError } = props;
  const [front, setFront] = useState(true);
  const [framing, setFraming] = useState(false);
  const [callsAvailable, setCallsAvailable] = useState(false);
  const [inCalls, setInCalls] = useState(false);
  const [joining, setJoining] = useState(false);
  useEffect(() => {
    callCameraAvailable()
      .then(setCallsAvailable)
      .catch(() => {});
  }, []);
  const feed = useMemo(
    () => ({
      start: (frames: Channel<ArrayBuffer>) => startCamera(peer, true, frames),
      stop: stopCamera,
      onEnded: onCameraEnded,
    }),
    [peer],
  );
  const { canvas, showing } = useVideo(feed, onClose, onError);
  const steer = (control: Parameters<typeof cameraControl>[0]) =>
    void cameraControl(control).catch((error) => onError(errorMessage(error)));

  return (
    <div className="mirror" role="dialog" aria-label="Webcam">
      <div className="webcam">
        <button type="button" className="icon-btn mirror-close" aria-label="Close" onClick={onClose}>
          <X size={20} />
        </button>
        <canvas ref={canvas} className="webcam-picture" />
        {!showing && <p className="supporting mirror-note">Starting the camera on your phone…</p>}
        {showing && inCalls && (
          <p className="supporting mirror-note">Pick Continue as the camera in your video call app.</p>
        )}
        <div className="mirror-buttons">
          <button
            type="button"
            className="icon-btn"
            aria-label={front ? "Use the back camera" : "Use the front camera"}
            onClick={() => {
              steer({ kind: "front", front: !front });
              setFront(!front);
            }}
          >
            <SwitchCamera size={20} />
          </button>
          <button
            type="button"
            className="icon-btn"
            aria-label="Keep me centred"
            aria-pressed={framing}
            onClick={() => {
              steer({ kind: "framing", on: !framing });
              setFraming(!framing);
            }}
          >
            <ScanFace size={20} />
          </button>
          {callsAvailable && (
            <button
              type="button"
              className="icon-btn"
              aria-label="Use in video calls"
              aria-pressed={inCalls}
              disabled={joining}
              onClick={() => {
                setJoining(true);
                setCallCamera(!inCalls)
                  .then(() => setInCalls(!inCalls))
                  .catch((error) => onError(errorMessage(error)))
                  .finally(() => setJoining(false));
              }}
            >
              <Video size={20} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
