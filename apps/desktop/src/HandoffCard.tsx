// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { ExternalLink, FileText, Globe, MapPin, X } from "lucide-react";
import { formatRelativeTime } from "./format.ts";
import { HandoffType, type HandoffItem } from "./types.ts";

interface HandoffCardProps {
  item: HandoffItem;
  peerName: string;
  onOpen: (item: HandoffItem) => void;
  onDismiss: (item: HandoffItem) => void;
}

export function HandoffCard({ item, peerName, onOpen, onDismiss }: HandoffCardProps) {
  const getIcon = () => {
    switch (item.handoffType) {
      case HandoffType.Document:
        return <FileText size={20} />;
      case HandoffType.Map:
        return <MapPin size={20} />;
      default:
        return <Globe size={20} />;
    }
  };

  const getDomain = (uri: string) => {
    try {
      return new URL(uri).hostname;
    } catch {
      return uri;
    }
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 16,
        padding: "16px 20px",
        borderRadius: 20,
        background: "var(--accent-soft)",
        border: "1px solid color-mix(in srgb, var(--accent) 30%, transparent)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 14, minWidth: 0, flex: 1 }}>
        <div
          style={{
            display: "grid",
            placeItems: "center",
            width: 40,
            height: 40,
            borderRadius: 12,
            background: "var(--accent)",
            color: "var(--on-accent)",
            flexShrink: 0,
          }}
        >
          {getIcon()}
        </div>

        <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
          <span className="list-sub" style={{ fontSize: 12, color: "var(--accent)" }}>
            Continue from {peerName} · {formatRelativeTime(item.timestampMs)}
          </span>
          <span
            className="list-title"
            style={{ fontWeight: 600, fontSize: 15 }}
            title={item.title || item.uri}
          >
            {item.title || getDomain(item.uri)}
          </span>
          <span className="list-sub" style={{ fontSize: 12 }} title={item.uri}>
            {getDomain(item.uri)}
          </span>
        </div>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
        <button
          type="button"
          className="btn btn-filled btn-small"
          onClick={() => onOpen(item)}
          title="Open and continue reading"
        >
          <ExternalLink size={16} />
          Continue
        </button>
        <button
          type="button"
          className="icon-btn"
          onClick={() => onDismiss(item)}
          aria-label="Dismiss handoff"
        >
          <X size={18} />
        </button>
      </div>
    </div>
  );
}
