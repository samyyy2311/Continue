// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from "react";
import { Check, Copy, Loader, X } from "lucide-react";
import { renderSVG } from "uqr";
import {
  cancelPairing,
  errorMessage,
  onPairingCompleted,
  onPairingFailed,
  pairFromCode,
  startPairing,
} from "./api.ts";
import type { TrustedPeer } from "./types.ts";

type Mode = "show" | "enter";

// Start and cancel must reach the backend in order: React re-runs effects (retry,
// StrictMode) and a late cancel would otherwise close the newer listener.
let pairingQueue: Promise<unknown> = Promise.resolve();
function queuePairingCall<T>(call: () => Promise<T>): Promise<T> {
  const result = pairingQueue.then(call, call);
  pairingQueue = result.catch(() => undefined);
  return result;
}

type ShowState =
  | { status: "starting" }
  | { status: "waiting"; code: string }
  | { status: "failed"; message: string };

interface PairDialogProps {
  onPaired: (peer: TrustedPeer) => void;
  onClose: () => void;
}

export function PairDialog({ onPaired, onClose }: PairDialogProps) {
  const [mode, setMode] = useState<Mode>("show");

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [onClose]);

  return (
    <div className="scrim" onMouseDown={onClose}>
      <div
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="pair-dialog-title"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <header className="dialog-header">
          <h2 id="pair-dialog-title">Pair a device</h2>
          <button type="button" className="icon-btn" onClick={onClose} aria-label="Close">
            <X size={16} />
          </button>
        </header>

        <div className="segmented" role="tablist" aria-label="Pairing method">
          <button
            type="button"
            role="tab"
            aria-selected={mode === "show"}
            className={mode === "show" ? "active" : ""}
            onClick={() => setMode("show")}
          >
            Show code
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={mode === "enter"}
            className={mode === "enter" ? "active" : ""}
            onClick={() => setMode("enter")}
          >
            Enter code
          </button>
        </div>

        {mode === "show" ? <ShowCode onPaired={onPaired} /> : <EnterCode onPaired={onPaired} />}
      </div>
    </div>
  );
}

function ShowCode({ onPaired }: { onPaired: (peer: TrustedPeer) => void }) {
  const [state, setState] = useState<ShowState>({ status: "starting" });
  const [attempt, setAttempt] = useState(0);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let active = true;
    const unlisteners = Promise.all([
      onPairingCompleted((peer) => active && onPaired(peer)),
      onPairingFailed((message) => active && setState({ status: "failed", message })),
    ]);

    setState({ status: "starting" });
    queuePairingCall(startPairing)
      .then((code) => active && setState({ status: "waiting", code }))
      .catch((error) => active && setState({ status: "failed", message: errorMessage(error) }));

    return () => {
      active = false;
      unlisteners.then((fns) => fns.forEach((unlisten) => unlisten()));
      // Closes the listener so a stale QR can't be used after the dialog is gone.
      queuePairingCall(cancelPairing).catch(() => undefined);
    };
  }, [attempt, onPaired]);

  const copyCode = async (code: string) => {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1600);
  };

  if (state.status === "failed") {
    return (
      <div className="dialog-body">
        <p className="inline-error" role="alert">{state.message}</p>
        <button type="button" className="btn btn-primary" onClick={() => setAttempt((n) => n + 1)}>
          Try again
        </button>
      </div>
    );
  }

  return (
    <div className="pair-show">
      <div className="pair-steps">
        <ol className="steps">
          <li>Open Continue on your phone</li>
          <li>
            <span>
              Tap <strong>Pair a device</strong>
            </span>
          </li>
          <li>Point the camera at this code</li>
        </ol>
        <p className="setting-help">Both devices need to be on the same Wi-Fi.</p>
        <p className="waiting-label" role="status">
          {state.status === "waiting" && (
            <>
              <span className="pulse-dot" aria-hidden="true" />
              Waiting for your device
            </>
          )}
        </p>
        <button
          type="button"
          className="btn btn-quiet"
          disabled={state.status !== "waiting"}
          onClick={() => state.status === "waiting" && copyCode(state.code)}
        >
          {copied ? <Check size={14} /> : <Copy size={14} />}
          {copied ? "Copied" : "Copy as text"}
        </button>
      </div>
      <div className="qr-frame" aria-busy={state.status === "starting"}>
        {state.status === "waiting" ? (
          <div
            className="qr"
            role="img"
            aria-label="Pairing QR code"
            dangerouslySetInnerHTML={{ __html: renderSVG(state.code, { border: 2 }) }}
          />
        ) : (
          <Loader size={18} className="spin faint" aria-label="Preparing code" />
        )}
      </div>
    </div>
  );
}

function EnterCode({ onPaired }: { onPaired: (peer: TrustedPeer) => void }) {
  const [code, setCode] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    setPending(true);
    setError("");
    try {
      onPaired(await pairFromCode(code.trim()));
    } catch (err) {
      setError(errorMessage(err));
      setPending(false);
    }
  };

  return (
    <form className="dialog-body" onSubmit={submit}>
      <label className="field">
        <span className="field-label">Code from your other device</span>
        <textarea
          className="input code"
          rows={3}
          value={code}
          onChange={(e) => setCode(e.target.value)}
          placeholder="Paste it here"
          spellCheck={false}
          autoFocus
        />
      </label>
      {error && <p className="inline-error" role="alert">{error}</p>}
      <div className="dialog-footer">
        <span />
        <button type="submit" className="btn btn-primary" disabled={pending || !code.trim()}>
          {pending ? "Pairing…" : "Pair"}
        </button>
      </div>
    </form>
  );
}
