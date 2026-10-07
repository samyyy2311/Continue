// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import { Check, Copy, Loader, X } from "lucide-react";
import { renderSVG } from "uqr";
import {
  cancelPairing,
  errorMessage,
  onPairingCompleted,
  onPairingFailed,
  pairFromCode,
  startPairing,
  confirmPairing,
  onPairingCheck,
} from "./api.ts";
import type { TrustedPeer } from "./types.ts";

type Mode = "show" | "enter";

const MODES: { value: Mode; label: string }[] = [
  { value: "show", label: "Show code" },
  { value: "enter", label: "Enter code" },
];

// Starting and cancelling a code run one at a time, so closing and reopening quickly can't
// cancel the new code.
let pairingQueue: Promise<unknown> = Promise.resolve();
function queuePairingCall<T>(call: () => Promise<T>): Promise<T> {
  const result = pairingQueue.then(call, call);
  pairingQueue = result.catch(() => undefined);
  return result;
}

type ShowState =
  | { status: "starting" }
  | { status: "waiting"; code: string }
  | { status: "checking"; digits: string }
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
            <h2 id="pair-dialog-title" className="dialog-title">
              Pair a device
            </h2>
            <p className="dialog-subtitle">Both need to be on the same Wi-Fi.</p>
          </div>
          <button type="button" className="icon-btn" onClick={onClose} aria-label="Close">
            <X size={20} />
          </button>
        </header>

        <div className="chips" role="tablist" aria-label="How to pair">
          {MODES.map((option) => (
            <button
              key={option.value}
              type="button"
              role="tab"
              aria-selected={mode === option.value}
              className="chip"
              onClick={() => setMode(option.value)}
            >
              {option.label}
            </button>
          ))}
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
      onPairingCheck((digits) => active && setState({ status: "checking", digits })),
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

  if (state.status === "checking") {
    const answer = (accept: boolean) =>
      confirmPairing(accept)
        .then(() => !accept && setAttempt((n) => n + 1))
        .catch((error) => setState({ status: "failed", message: errorMessage(error) }));
    return (
      <div className="section">
        <p className="supporting">Check that your phone shows the same six digits.</p>
        <p className="pair-digits" aria-label={`Code ${state.digits.split("").join(" ")}`}>
          {state.digits.slice(0, 3)} {state.digits.slice(3)}
        </p>
        <div className="dialog-actions">
          <button type="button" className="btn btn-text" onClick={() => void answer(false)}>
            Not mine
          </button>
          <button type="button" className="btn btn-filled" onClick={() => void answer(true)}>
            Pair
          </button>
        </div>
      </div>
    );
  }

  if (state.status === "failed") {
    return (
      <div className="section">
        <p className="error-banner" role="alert">
          {state.message}
        </p>
        <div>
          <button type="button" className="btn btn-filled" onClick={() => setAttempt((n) => n + 1)}>
            Try again
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="pair-show">
      <div className="section">
        <ol className="steps">
          <li>Open Continue on your phone</li>
          <li>Tap Pair</li>
          <li>Point the camera at this code, or pick this computer under Nearby</li>
        </ol>
        <p className="waiting" role="status">
          {state.status === "waiting" ? "Waiting for your phone" : "Getting a code ready"}
        </p>
        <div>
          <button
            type="button"
            className="btn btn-tonal btn-small"
            disabled={state.status !== "waiting"}
            onClick={() => state.status === "waiting" && copyCode(state.code)}
          >
            {copied ? <Check size={16} /> : <Copy size={16} />}
            {copied ? "Copied" : "Copy as text"}
          </button>
        </div>
      </div>

      {state.status === "waiting" ? (
        <div
          className="qr"
          role="img"
          aria-label="Pairing code"
          dangerouslySetInnerHTML={{ __html: renderSVG(state.code, { border: 1 }) }}
        />
      ) : (
        <div className="qr">
          <Loader size={24} className="spin" />
        </div>
      )}
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
      onPaired(await pairFromCode(code.trim()));
    } catch (err) {
      setError(errorMessage(err));
      setPending(false);
    }
  };

  return (
    <form className="section" onSubmit={submit}>
      <p className="supporting">Got a code from your other device? Paste it here.</p>
      <textarea
        className="input-textarea font-mono"
        rows={4}
        value={code}
        onChange={(e) => setCode(e.target.value)}
        placeholder="Pairing code"
        aria-label="Pairing code"
        spellCheck={false}
        autoFocus
      />
      {error && (
        <p className="error-banner" role="alert">
          {error}
        </p>
      )}
      <div className="dialog-actions">
        <button type="submit" className="btn btn-filled" disabled={pending || !code.trim()}>
          {pending && <Loader size={16} className="spin" />}
          {pending ? "Pairing" : "Pair"}
        </button>
      </div>
    </form>
  );
}
