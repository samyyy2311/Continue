// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from "react";
import { BellOff, Phone, PhoneOff } from "lucide-react";
import { answerCall, declineCall, errorMessage, onPhoneCall, sendSms, silenceCall } from "./api.ts";
import type { PhoneCall } from "./types.ts";

const QUICK_REPLIES = ["Can't talk now. I'll call you back.", "On my way.", "In a meeting, text me?"];

export function CallBanner({ onError }: { onError: (message: string) => void }) {
  const [call, setCall] = useState<PhoneCall | null>(null);
  const [since, setSince] = useState(0);
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const unlisten = onPhoneCall((next) => {
      setCall(next.state === "ended" ? null : next);
      if (next.state === "talking") setSince(Date.now());
    });
    return () => void unlisten.then((stop) => stop());
  }, []);

  const talking = call?.state === "talking";
  useEffect(() => {
    if (!talking) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [talking]);

  if (!call) return null;
  const caller = call.name || call.number || "Unknown caller";
  const act = (action: Promise<void>) => action.catch((error) => onError(errorMessage(error)));
  const decline = () => act(declineCall(call.peerId));
  const declineWith = (text: string) =>
    act(declineCall(call.peerId).then(() => (call.number ? sendSms(call.peerId, call.number, text) : undefined)));

  if (talking) {
    const seconds = Math.max(0, Math.floor((now - since) / 1000));
    const length = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
    return (
      <div className="call-pill" role="status">
        <Phone size={16} />
        <span>
          {caller} · {length}
        </span>
        <button type="button" className="call-end" aria-label="Hang up" onClick={decline}>
          <PhoneOff size={16} />
        </button>
      </div>
    );
  }

  return (
    <div className="call-card" role="alertdialog" aria-label={`Call from ${caller}`}>
      <span className="avatar call-avatar" aria-hidden="true">
        {caller.replace(/^\+/, "").charAt(0).toUpperCase()}
      </span>
      <div className="call-text">
        <span className="label">Incoming call</span>
        <span className="title">{caller}</span>
        {call.name && <span className="supporting">{call.number}</span>}
      </div>
      <div className="call-buttons">
        <button
          type="button"
          className="call-button"
          aria-label="Silence"
          title="Silence"
          onClick={() => act(silenceCall(call.peerId))}
        >
          <BellOff size={22} />
        </button>
        <button type="button" className="call-button decline" aria-label="Decline" onClick={decline}>
          <PhoneOff size={22} />
        </button>
        <button
          type="button"
          className="call-button answer"
          aria-label="Answer"
          onClick={() => act(answerCall(call.peerId))}
        >
          <Phone size={22} />
        </button>
      </div>
      {call.number && (
        <div className="call-replies">
          {QUICK_REPLIES.map((text) => (
            <button key={text} type="button" className="chip" onClick={() => declineWith(text)}>
              {text}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
