// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from "react";
import { Check, Copy, Pin, PinOff, Search, Trash2, X } from "lucide-react";
import {
  clearClipboardHistory,
  deleteClipboardClip,
  errorMessage,
  getClipboardHistory,
  pinClipboardClip,
} from "./api.ts";
import { formatRelativeTime } from "./format.ts";
import type { ClipboardHistoryItem } from "./types.ts";

interface ClipboardHistoryDialogProps {
  onClose: () => void;
  onError: (message: string) => void;
}

export function ClipboardHistoryDialog({ onClose, onError }: ClipboardHistoryDialogProps) {
  const [clips, setClips] = useState<ClipboardHistoryItem[]>([]);
  const [search, setSearch] = useState("");
  const [copiedId, setCopiedId] = useState<number | null>(null);

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [onClose]);

  const refresh = () => {
    getClipboardHistory(100)
      .then(setClips)
      .catch((err) => onError(errorMessage(err)));
  };

  useEffect(() => {
    refresh();
  }, []);

  const handleCopy = async (clip: ClipboardHistoryItem) => {
    try {
      await navigator.clipboard.writeText(clip.content);
      setCopiedId(clip.id);
      window.setTimeout(() => setCopiedId(null), 1500);
    } catch {
      onError("Couldn't copy text to clipboard.");
    }
  };

  const handleTogglePin = async (clip: ClipboardHistoryItem) => {
    try {
      await pinClipboardClip(clip.id, !clip.isPinned);
      setClips((prev) =>
        prev.map((c) => (c.id === clip.id ? { ...c, isPinned: !c.isPinned } : c)),
      );
    } catch (err) {
      onError(errorMessage(err));
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await deleteClipboardClip(id);
      setClips((prev) => prev.filter((c) => c.id !== id));
    } catch (err) {
      onError(errorMessage(err));
    }
  };

  const handleClear = async () => {
    try {
      await clearClipboardHistory();
      refresh();
    } catch (err) {
      onError(errorMessage(err));
    }
  };

  const filtered = clips.filter((c) =>
    c.content.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div className="scrim" onMouseDown={onClose}>
      <div
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="clipboard-history-title"
        onMouseDown={(e) => e.stopPropagation()}
        style={{ maxWidth: 640 }}
      >
        <header className="dialog-header">
          <div>
            <h2 id="clipboard-history-title" className="dialog-title">
              Clipboard History
            </h2>
            <p className="dialog-subtitle">Recent clips synced across your devices.</p>
          </div>
          <button type="button" className="icon-btn" onClick={onClose} aria-label="Close">
            <X size={20} />
          </button>
        </header>

        <div className="toolbar">
          <label className="search" style={{ flex: 1 }}>
            <Search size={18} aria-hidden="true" />
            <input
              type="search"
              placeholder="Search history..."
              aria-label="Search clipboard history"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              style={{ width: "100%" }}
            />
          </label>

          <button
            type="button"
            className="btn btn-text btn-small"
            onClick={handleClear}
            disabled={clips.every((c) => c.isPinned)}
          >
            Clear unpinned
          </button>
        </div>

        {filtered.length === 0 ? (
          <div style={{ textAlign: "center", padding: "40px 0" }}>
            <p className="supporting">
              {search ? `Nothing matches "${search}".` : "No clipboard history yet."}
            </p>
          </div>
        ) : (
          <ul className="list" style={{ maxHeight: 420, overflowY: "auto" }}>
            {filtered.map((clip) => {
              const isCopied = copiedId === clip.id;
              return (
                <li key={clip.id} className="list-item" style={{ alignItems: "flex-start" }}>
                  <div className="list-text" style={{ minWidth: 0, flex: 1 }}>
                    <span
                      className="list-title"
                      style={{
                        whiteSpace: "pre-wrap",
                        wordBreak: "break-word",
                        fontFamily: "var(--font)",
                        fontSize: 14,
                        maxHeight: 72,
                        overflow: "hidden",
                        display: "-webkit-box",
                        WebkitLineClamp: 3,
                        WebkitBoxOrient: "vertical",
                      }}
                    >
                      {clip.content}
                    </span>
                    <span className="list-sub" style={{ marginTop: 4 }}>
                      {clip.originDevice} · {formatRelativeTime(clip.timestampMs)}
                      {clip.isPinned && " · Pinned"}
                    </span>
                  </div>

                  <div className="list-trailing" style={{ flexShrink: 0, gap: 4 }}>
                    <button
                      type="button"
                      className="icon-btn"
                      title={clip.isPinned ? "Unpin clip" : "Pin clip"}
                      onClick={() => handleTogglePin(clip)}
                    >
                      {clip.isPinned ? (
                        <PinOff size={16} style={{ color: "var(--accent)" }} />
                      ) : (
                        <Pin size={16} />
                      )}
                    </button>
                    <button
                      type="button"
                      className="icon-btn"
                      title="Copy clip"
                      onClick={() => handleCopy(clip)}
                    >
                      {isCopied ? <Check size={16} /> : <Copy size={16} />}
                    </button>
                    <button
                      type="button"
                      className="icon-btn"
                      title="Delete clip"
                      onClick={() => handleDelete(clip.id)}
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
