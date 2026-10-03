// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import { Check, Copy, Loader, QrCode, Smartphone, X } from "lucide-react";
import { renderSVG } from "uqr";
import { isTauri } from "@tauri-apps/api/core";
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
          <div>
            <h2 id="pair-dialog-title" className="dialog-title">Pair a device</h2>
            <p className="dialog-subtitle">Both devices need to be on the same Wi-Fi.</p>
          </div>
          <button type="button" className="icon-btn" onClick={onClose} aria-label="Close dialog">
            <X size={18} />
          </button>
        </header>

        <div className="segmented-tabs" role="tablist" aria-label="Pairing method">
          <button
            type="button"
            role="tab"
            aria-selected={mode === "show"}
            className={`tab-btn ${mode === "show" ? "active" : ""}`}
            onClick={() => setMode("show")}
          >
            <QrCode size={15} />
            Show code
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={mode === "enter"}
            className={`tab-btn ${mode === "enter" ? "active" : ""}`}
            onClick={() => setMode("enter")}
          >
            <Smartphone size={15} />
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
    if (!isTauri()) {
      setState({ status: "waiting", code: "continue://pair?v=1&addr=192.168.1.50:4433&fp=e49a:21fc:87aa" });
      return;
    }

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
      <div className="dialog-body error-view">
        <p className="error-banner" role="alert">{state.message}</p>
        <button type="button" className="btn btn-primary" onClick={() => setAttempt((n) => n + 1)}>
          Try Again
        </button>
      </div>
    );
  }

  return (
    <div className="pair-show">
      <div className="pair-instructions">
        <ol className="step-list">
          <li>
            <span className="step-number">1</span>
            <span>Open <strong>Continue</strong> on your phone</span>
          </li>
          <li>
            <span className="step-number">2</span>
            <span>Tap <strong>Scan code</strong></span>
          </li>
          <li>
            <span className="step-number">3</span>
            <span>Point the camera at this code</span>
          </li>
        </ol>

        <div className="pair-actions">
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            disabled={state.status !== "waiting"}
            onClick={() => state.status === "waiting" && copyCode(state.code)}
          >
            {copied ? <Check size={14} className="text-success" /> : <Copy size={14} />}
            {copied ? "Copied" : "Copy code as text"}
          </button>
        </div>

        <div className="pair-status-bar" role="status">
          {state.status === "waiting" ? (
            <span className="waiting">
              <span className="waiting-dot" />
              Waiting for your phone
            </span>
          ) : (
            <span className="text-muted">
              <Loader size={14} className="spin inline-icon" /> Getting a code ready
            </span>
          )}
        </div>
      </div>

      <div className="qr-container">
        {state.status === "waiting" ? (
          <div
            className="qr-wrapper"
            role="img"
            aria-label="Pairing QR code"
            dangerouslySetInnerHTML={{ __html: renderSVG(state.code, { border: 2 }) }}
          />
        ) : (
          <div className="qr-loading">
            <Loader size={24} className="spin text-accent" />
            <span>Getting a code ready</span>
          </div>
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
    if (!code.trim() || pending) return;
    setPending(true);
    setError("");
    try {
      if (!isTauri()) {
        onPaired({
          displayName: "Paired Phone",
          fingerprint: "a1b2:c3d4:e5f6:7890",
          pairedAt: Math.floor(Date.now() / 1000),
          isConnected: true,
          endpoint: "192.168.1.55:4433",
        });
        return;
      }
      const peer = await pairFromCode(code.trim());
      onPaired(peer);
    } catch (err) {
      setError(errorMessage(err));
      setPending(false);
    }
  };

  return (
    <form className="enter-code-form" onSubmit={submit}>
      <p className="field-desc">
        Got a code from your other device? Paste it here.
      </p>
      <label className="field-block">
        <span className="field-label">Pairing code</span>
        <textarea
          className="input-textarea font-mono"
          rows={4}
          value={code}
          onChange={(e) => setCode(e.target.value)}
          placeholder="Paste the code"
          spellCheck={false}
          autoFocus
        />
      </label>
      {error && <p className="error-banner" role="alert">{error}</p>}
      <div className="form-actions">
        <button type="submit" className="btn btn-primary" disabled={pending || !code.trim()}>
          {pending && <Loader size={14} className="spin inline-icon" />}
          {pending ? "Pairing" : "Pair"}
        </button>
      </div>
    </form>
  );
}
