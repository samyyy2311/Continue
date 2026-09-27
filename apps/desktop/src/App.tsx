// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowUp,
  Check,
  CheckCheck,
  CircleAlert,
  Copy,
  History,
  Loader,
  Paperclip,
  Plus,
  Search,
  Send,
  Settings,
  SlidersHorizontal,
  Type,
  Upload,
} from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import "./App.css";
import {
  connectToPeer,
  disconnectPeer,
  errorMessage,
  getDeviceIdentity,
  getPermissions,
  getTrustedPeers,
  removeTrustedPeer,
  sendClipboardText,
  sendFileToPeer,
  setPermission,
} from "./api.ts";
import { fileNameFromPath, formatBytes, formatPairedDate, formatRelativeTime } from "./format.ts";
import { PairDialog } from "./PairDialog.tsx";
import {
  ACCENT_PALETTE,
  type AccentName,
  type DeviceIdentity,
  type Grant,
  type PeerPermission,
  type TrustedPeer,
} from "./types.ts";

type View = { name: "send" | "history" | "settings" } | { name: "device"; fingerprint: string };
type Theme = "system" | "light" | "dark";
type HistoryFilter = "all" | Activity["kind"];

interface Toast {
  message: string;
  tone: "info" | "error";
}

interface Activity {
  id: string;
  kind: "file" | "text";
  label: string;
  peerId: string;
  peerName: string;
  status: "sending" | "sent" | "failed";
  timestamp: number;
  /** Kept so a failed file can be sent again. */
  path?: string;
  bytesSent?: number;
  totalBytes?: number;
  error?: string;
}

const ACCENT_KEY = "continue.accent";
const THEME_KEY = "continue.theme";
const ENDPOINTS_KEY = "continue.peerEndpoints";

function readStored<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  try {
    const stored = localStorage.getItem(key);
    return allowed.includes(stored as T) ? (stored as T) : fallback;
  } catch {
    return fallback;
  }
}

function writeStored(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage can be unavailable (e.g. blocked site data); the choice just won't survive a restart.
  }
}

// Discovery isn't wired into the desktop yet, so the last address used for each
// peer is remembered locally to avoid retyping it on every connect.
function loadSavedEndpoints(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem(ENDPOINTS_KEY) ?? "{}");
  } catch {
    return {};
  }
}

function saveEndpoint(fingerprint: string, endpoint: string) {
  writeStored(ENDPOINTS_KEY, JSON.stringify({ ...loadSavedEndpoints(), [fingerprint]: endpoint }));
}

const FILE_KINDS: { tone: string; extensions: string[] }[] = [
  { tone: "doc", extensions: ["pdf", "doc", "docx", "txt", "md", "rtf", "xls", "xlsx", "csv"] },
  { tone: "image", extensions: ["png", "jpg", "jpeg", "gif", "webp", "heic", "dng"] },
  { tone: "media", extensions: ["mp3", "m4a", "wav", "flac", "mp4", "mov", "mkv"] },
  { tone: "archive", extensions: ["zip", "rar", "7z", "apk"] },
];

// The backend names capabilities after the protocol; people think in terms of what gets shared.
const PERMISSIONS: Record<string, { label: string; description: string }> = {
  "File Transfer": { label: "Files", description: "Send and receive files" },
  "Clipboard Sync": { label: "Clipboard", description: "Share copied text and links" },
  "Notification Relay": { label: "Notifications", description: "Show phone notifications here" },
};

const GRANT_OPTIONS: { value: Grant; label: string }[] = [
  { value: "Allow", label: "Allow" },
  { value: "Ask", label: "Ask" },
  { value: "Deny", label: "Block" },
];

const THEMES: { value: Theme; label: string }[] = [
  { value: "system", label: "Auto" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

const HISTORY_FILTERS: { value: HistoryFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "file", label: "Files" },
  { value: "text", label: "Text" },
];

function Segmented<T extends string>(props: {
  label: string;
  options: { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
}) {
  return (
    <div className="segmented" role="radiogroup" aria-label={props.label}>
      {props.options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={props.value === option.value}
          onClick={() => props.onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

function ActivityIcon({ item }: { item: Activity }) {
  if (item.kind === "text") {
    return (
      <span className="badge badge-text" aria-hidden="true">
        <Type size={16} />
      </span>
    );
  }
  const ext = item.label.includes(".") ? (item.label.split(".").pop() ?? "").toLowerCase() : "";
  const tone = FILE_KINDS.find((k) => k.extensions.includes(ext))?.tone ?? "other";
  return (
    <span className={`badge badge-${tone}`} aria-hidden="true">
      {ext.slice(0, 4) || "file"}
    </span>
  );
}

function ActivityRow({ item, onCopy }: { item: Activity; onCopy: (text: string) => void }) {
  return (
    <li className="activity-row" title={item.error}>
      <ActivityIcon item={item} />
      <div className="activity-main">
        <span className="activity-label">{item.label}</span>
        <span className="activity-meta">
          {item.peerName} · {formatRelativeTime(item.timestamp)}
        </span>
      </div>
      <span className={`activity-status ${item.status}`}>
        {item.status === "sending" && <Loader size={14} className="spin" />}
        {item.status === "sending" && "Sending"}
        {item.status === "sent" && (item.bytesSent !== undefined ? formatBytes(item.bytesSent) : "Sent")}
        {item.status === "failed" && "Didn't send"}
      </span>
      {item.kind === "text" && (
        <button type="button" className="icon-btn" onClick={() => onCopy(item.label)} aria-label="Copy" title="Copy">
          <Copy size={15} />
        </button>
      )}
    </li>
  );
}

function dayLabel(timestamp: number, now = new Date()): string {
  const day = new Date(timestamp);
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (day.toDateString() === now.toDateString()) return "Today";
  if (day.toDateString() === yesterday.toDateString()) return "Yesterday";
  return day.toLocaleDateString(undefined, { weekday: "long", month: "short", day: "numeric" });
}

function timeLabel(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}

function ItemStatus({ item, onRetry }: { item: Activity; onRetry: (item: Activity) => void }) {
  if (item.status === "failed") {
    return (
      <span className="item-status failed" title={item.error}>
        Didn't send ·{" "}
        <button type="button" onClick={() => onRetry(item)}>
          Try again
        </button>
      </span>
    );
  }
  return (
    <span className="item-status">
      {timeLabel(item.timestamp)}
      {item.status === "sending" ? " · Sending" : <CheckCheck size={14} aria-label="Delivered" />}
    </span>
  );
}

function Thread(props: {
  items: Activity[];
  onCopy: (text: string) => void;
  onRetry: (item: Activity) => void;
  children?: React.ReactNode;
}) {
  const { items, onCopy, onRetry, children } = props;
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [items.length]);

  return (
    <div className="thread">
      {items.length === 0 && !children && (
        <p className="thread-empty">Nothing sent yet. Type below, or drag files anywhere onto this window.</p>
      )}
      {items.map((item, index) => {
        const label = dayLabel(item.timestamp);
        const newDay = index === 0 || dayLabel(items[index - 1].timestamp) !== label;
        return (
          <React.Fragment key={item.id}>
            {newDay && <p className="day">{label}</p>}
            {item.kind === "text" ? (
              <div className={`bubble ${item.status}`}>
                <p className="bubble-text">{item.label}</p>
                <div className="bubble-foot">
                  <button type="button" className="bubble-copy" onClick={() => onCopy(item.label)}>
                    <Copy size={13} />
                    Copy
                  </button>
                  <ItemStatus item={item} onRetry={onRetry} />
                </div>
              </div>
            ) : (
              <div className={`file-card ${item.status}`}>
                <ActivityIcon item={item} />
                <div className="file-card-main">
                  <p className="file-card-name">{item.label}</p>
                  {item.status === "sending" && item.totalBytes ? (
                    <>
                      <div className="progress" role="progressbar" aria-valuenow={item.bytesSent} aria-valuemax={item.totalBytes}>
                        <span style={{ width: `${((item.bytesSent ?? 0) / item.totalBytes) * 100}%` }} />
                      </div>
                      <p className="file-card-meta">
                        {formatBytes(item.bytesSent ?? 0)} of {formatBytes(item.totalBytes)}
                      </p>
                    </>
                  ) : (
                    <p className="file-card-meta">
                      {item.status === "sending" && "Preparing"}
                      {item.status === "sent" && item.bytesSent !== undefined && formatBytes(item.bytesSent)}
                    </p>
                  )}
                </div>
                <ItemStatus item={item} onRetry={onRetry} />
              </div>
            )}
          </React.Fragment>
        );
      })}
      {children}
      <div ref={endRef} />
    </div>
  );
}

function Row({ title, text, children }: { title: string; text?: string; children?: React.ReactNode }) {
  return (
    <div className="row">
      <div className="row-text">
        <p className="row-title">{title}</p>
        {text && <p className="row-sub">{text}</p>}
      </div>
      {children}
    </div>
  );
}

export default function App() {
  const [view, setView] = useState<View>({ name: "send" });
  const [accent, setAccent] = useState<AccentName>(() =>
    readStored(ACCENT_KEY, ACCENT_PALETTE.map((a) => a.id), "blue"),
  );
  const [theme, setTheme] = useState<Theme>(() => readStored(THEME_KEY, ["system", "light", "dark"], "system"));
  const [identity, setIdentity] = useState<DeviceIdentity | null>(null);
  const [peers, setPeers] = useState<TrustedPeer[] | null>(null);
  const [loadError, setLoadError] = useState("");
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);
  const [showPairDialog, setShowPairDialog] = useState(false);
  const [activity, setActivity] = useState<Activity[]>([]);
  const [note, setNote] = useState("");
  const [endpointDrafts, setEndpointDrafts] = useState<Record<string, string>>({});
  const [dragCount, setDragCount] = useState<number | null>(null);
  const [connecting, setConnecting] = useState<string | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);

  const selectedPeer = peers?.find((p) => p.fingerprint === selectedPeerId) ?? peers?.[0] ?? null;
  const endpoint = selectedPeer
    ? (endpointDrafts[selectedPeer.fingerprint] ??
      selectedPeer.endpoint ??
      loadSavedEndpoints()[selectedPeer.fingerprint] ??
      "")
    : "";

  const showToast = useCallback((message: string, tone: Toast["tone"] = "info") => {
    setToast({ message, tone });
  }, []);
  const showError = useCallback((message: string) => showToast(message, "error"), [showToast]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), toast.tone === "error" ? 6000 : 2500);
    return () => window.clearTimeout(timer);
  }, [toast]);

  useEffect(() => {
    const active = ACCENT_PALETTE.find((a) => a.id === accent) ?? ACCENT_PALETTE[0];
    document.documentElement.style.setProperty("--accent", active.base);
    document.documentElement.style.setProperty("--on-accent", active.onBase);
    writeStored(ACCENT_KEY, accent);
  }, [accent]);

  // The stylesheet only knows light and dark, so "Auto" is resolved here and kept in sync with the OS.
  useEffect(() => {
    writeStored(THEME_KEY, theme);
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const resolved = theme === "system" ? (query.matches ? "dark" : "light") : theme;
      document.documentElement.dataset.theme = resolved;
    };
    apply();
    query.addEventListener("change", apply);
    return () => query.removeEventListener("change", apply);
  }, [theme]);

  const refreshPeers = useCallback(async () => {
    try {
      setPeers(await getTrustedPeers());
    } catch (error) {
      showError(errorMessage(error));
    }
  }, [showError]);

  useEffect(() => {
    if (!isTauri()) return;
    Promise.all([getDeviceIdentity(), getTrustedPeers()])
      .then(([loadedIdentity, loadedPeers]) => {
        setIdentity(loadedIdentity);
        setPeers(loadedPeers);
      })
      .catch((error) => setLoadError(errorMessage(error)));
  }, []);

  const connect = async (e: React.FormEvent) => {
    e.preventDefault();
    const address = endpoint.trim();
    if (!selectedPeer || !address) return;
    setConnecting(selectedPeer.fingerprint);
    try {
      await connectToPeer(selectedPeer.fingerprint, address);
      saveEndpoint(selectedPeer.fingerprint, address);
      await refreshPeers();
    } catch (error) {
      showError(errorMessage(error));
    } finally {
      setConnecting(null);
    }
  };

  const disconnect = async (peer: TrustedPeer) => {
    try {
      await disconnectPeer(peer.fingerprint);
      await refreshPeers();
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const removePeer = async (peer: TrustedPeer) => {
    try {
      await removeTrustedPeer(peer.fingerprint);
      showToast(`Unpaired ${peer.displayName}`);
      setView({ name: "send" });
      refreshPeers();
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const track = async (
    item: Omit<Activity, "id" | "status" | "timestamp">,
    send: (update: (patch: Partial<Activity>) => void) => Promise<number | void>,
  ) => {
    const id = crypto.randomUUID();
    setActivity((prev) => [{ ...item, id, status: "sending", timestamp: Date.now() }, ...prev]);
    const update = (patch: Partial<Activity>) =>
      setActivity((prev) => prev.map((a) => (a.id === id ? { ...a, ...patch } : a)));
    try {
      const bytesSent = await send(update);
      update({ status: "sent", bytesSent: bytesSent ?? undefined });
      return true;
    } catch (error) {
      update({ status: "failed", error: errorMessage(error) });
      return false;
    }
  };

  const sendFile = (peer: TrustedPeer, path: string) =>
    track(
      { kind: "file", label: fileNameFromPath(path), path, peerId: peer.fingerprint, peerName: peer.displayName },
      (update) => sendFileToPeer(peer.fingerprint, path, (progress) => update(progress)),
    );

  const sendText = (peer: TrustedPeer, text: string) =>
    track({ kind: "text", label: text, peerId: peer.fingerprint, peerName: peer.displayName }, () =>
      sendClipboardText(peer.fingerprint, text),
    );

  const sendFiles = async (paths: string[]) => {
    if (!selectedPeer?.isConnected) {
      showError(selectedPeer ? `Connect to ${selectedPeer.displayName} first.` : "Pair a device first.");
      return;
    }
    for (const path of paths) {
      await sendFile(selectedPeer, path);
    }
  };

  const retry = (item: Activity) => {
    const peer = peers?.find((p) => p.fingerprint === item.peerId);
    if (!peer?.isConnected) {
      showError(`Connect to ${item.peerName} first.`);
      return;
    }
    setActivity((prev) => prev.filter((a) => a.id !== item.id));
    if (item.kind === "file" && item.path) {
      sendFile(peer, item.path);
    } else {
      sendText(peer, item.label);
    }
  };

  // The webview listener is registered once, so it reads the latest sender through a ref.
  const sendFilesRef = useRef(sendFiles);
  sendFilesRef.current = sendFiles;

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter") {
        setView({ name: "send" });
        setDragCount(payload.paths.length);
        return;
      }
      if (payload.type === "over") return;
      setDragCount(null);
      if (payload.type === "drop" && payload.paths.length > 0) {
        sendFilesRef.current(payload.paths);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const chooseFiles = async () => {
    const picked = await openFileDialog({ multiple: true, directory: false });
    if (picked) sendFiles(picked);
  };

  const sendNote = async (e: React.FormEvent) => {
    e.preventDefault();
    const text = note.trim();
    if (!selectedPeer || !text) return;
    setNote("");
    const sent = await sendText(selectedPeer, text);
    if (!sent) setNote(text);
  };

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showToast("Copied");
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const handlePaired = useCallback(
    (peer: TrustedPeer) => {
      setShowPairDialog(false);
      setSelectedPeerId(peer.fingerprint);
      setView({ name: "send" });
      showToast(`Paired with ${peer.displayName}`);
      refreshPeers();
    },
    [refreshPeers, showToast],
  );

  const closePairDialog = useCallback(() => setShowPairDialog(false), []);
  const openPairDialog = () => setShowPairDialog(true);

  const renderWelcome = (title: string, text: string, action?: React.ReactNode) => (
    <div className="welcome">
      <h1>{title}</h1>
      <p className="lead">{text}</p>
      {action}
    </div>
  );

  const renderSend = () => {
    if (!selectedPeer) {
      return renderWelcome(
        "Your phone, one step away",
        "Pair once by scanning a code. After that, anything you send here lands on your phone, over your own Wi-Fi.",
        <button type="button" className="btn btn-primary btn-large" onClick={openPairDialog}>
          Pair a device
        </button>,
      );
    }

    const peer = selectedPeer;
    const online = peer.isConnected;
    const busy = connecting === peer.fingerprint;
    const items = activity.filter((a) => a.peerId === peer.fingerprint).reverse();

    return (
      <div className="conversation">
        <header className="conversation-head">
          <span className={`avatar ${online ? "online" : ""}`} aria-hidden="true">
            {peer.displayName.slice(0, 1).toUpperCase()}
          </span>
          <div className="conversation-title">
            <h1>{peer.displayName}</h1>
            <p className={`presence ${online ? "online" : ""}`}>{online ? "Connected" : "Offline"}</p>
          </div>
          {online && (
            <button type="button" className="btn btn-quiet" onClick={() => disconnect(peer)}>
              Disconnect
            </button>
          )}
          <button
            type="button"
            className="icon-btn"
            onClick={() => setView({ name: "device", fingerprint: peer.fingerprint })}
            aria-label={`${peer.displayName} settings`}
            title="Device settings"
          >
            <SlidersHorizontal size={17} />
          </button>
        </header>

        <Thread items={items} onCopy={copyText} onRetry={retry}>
          {!online && (
            <form className="notice" onSubmit={connect}>
              <p>
                <strong>{peer.displayName} is offline.</strong> Open Continue on it and enter the address it shows.
              </p>
              <div className="notice-row">
                <input
                  className="input address"
                  value={endpoint}
                  onChange={(e) => setEndpointDrafts((prev) => ({ ...prev, [peer.fingerprint]: e.target.value }))}
                  placeholder="192.168.1.20:4433"
                  aria-label="Phone address"
                  spellCheck={false}
                />
                <button type="submit" className="btn btn-primary" disabled={busy || !endpoint.trim()}>
                  {busy && <Loader size={16} className="spin" />}
                  {busy ? "Connecting" : "Connect"}
                </button>
              </div>
            </form>
          )}
        </Thread>

        <form className="composer" onSubmit={sendNote}>
          <button
            type="button"
            className="icon-btn"
            onClick={chooseFiles}
            disabled={!online}
            aria-label="Send files"
            title="Send files"
          >
            <Paperclip size={19} />
          </button>
          <input
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder={online ? `Send text to ${peer.displayName}` : "Connect to send"}
            aria-label="Text to send"
            disabled={!online}
          />
          <button type="submit" className="send-btn" disabled={!online || !note.trim()} aria-label="Send">
            <ArrowUp size={18} />
          </button>
        </form>

        {dragCount !== null && (
          <div className="drop-overlay" aria-hidden="true">
            <div className="drop-target">
              <Upload size={26} />
              <p className="drop-title">
                {online ? `Drop to send to ${peer.displayName}` : `Connect to ${peer.displayName} first`}
              </p>
              {online && dragCount > 0 && (
                <p className="drop-sub">{dragCount === 1 ? "1 file" : `${dragCount} files`}</p>
              )}
            </div>
          </div>
        )}
      </div>
    );
  };

  const devicePeer = view.name === "device" ? peers?.find((p) => p.fingerprint === view.fingerprint) : undefined;
  const ready = isTauri() && !loadError && peers !== null;

  if (!ready) {
    return (
      <main className="app app-bare">
        {(!isTauri() || loadError) &&
          renderWelcome(
            "Continue isn't running",
            isTauri() ? loadError : "Open this window from the Continue app, or start it with pnpm tauri dev.",
          )}
      </main>
    );
  }

  const navItems = [
    { view: "send", label: "Send", icon: <Send size={17} /> },
    { view: "history", label: "History", icon: <History size={17} /> },
    { view: "settings", label: "Settings", icon: <Settings size={17} /> },
  ] as const;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <img src="/icon.svg" alt="" />
          Continue
        </div>

        <nav className="nav" aria-label="Main">
          {navItems.map((item) => (
            <button
              key={item.view}
              type="button"
              className="nav-item"
              aria-current={view.name === item.view ? "page" : undefined}
              onClick={() => setView({ name: item.view })}
            >
              {item.icon}
              {item.label}
            </button>
          ))}
        </nav>

        <div className="sidebar-devices">
          <h2 className="sidebar-label">Devices</h2>
          {peers.map((peer) => (
            <button
              key={peer.fingerprint}
              type="button"
              className="nav-item"
              aria-current={view.name === "send" && peer === selectedPeer ? "true" : undefined}
              onClick={() => {
                setSelectedPeerId(peer.fingerprint);
                setView({ name: "send" });
              }}
            >
              <span className={`dot ${peer.isConnected ? "online" : ""}`} aria-hidden="true" />
              <span className="nav-item-text">{peer.displayName}</span>
            </button>
          ))}
          <button type="button" className="nav-item quiet" onClick={openPairDialog}>
            <Plus size={17} />
            Pair a device
          </button>
        </div>
      </aside>

      <main className="content">
        {view.name === "send" && renderSend()}
        {view.name === "history" && <HistoryPage activity={activity} onCopy={copyText} />}
        {view.name === "settings" && (
          <SettingsPage
            identity={identity}
            theme={theme}
            accent={accent}
            onThemeChange={setTheme}
            onAccentChange={setAccent}
            onCopy={copyText}
          />
        )}
        {view.name === "device" && devicePeer && (
          <DevicePage
            peer={devicePeer}
            onBack={() => setView({ name: "send" })}
            onUnpair={() => removePeer(devicePeer)}
            onError={showError}
          />
        )}
      </main>

      {showPairDialog && <PairDialog onPaired={handlePaired} onClose={closePairDialog} />}

      {toast && (
        <div className={`toast ${toast.tone}`} role={toast.tone === "error" ? "alert" : "status"}>
          {toast.tone === "error" ? <CircleAlert size={17} /> : <Check size={17} />}
          <span>{toast.message}</span>
        </div>
      )}
    </div>
  );
}

function HistoryPage({ activity, onCopy }: { activity: Activity[]; onCopy: (text: string) => void }) {
  const [filter, setFilter] = useState<HistoryFilter>("all");
  const [query, setQuery] = useState("");
  const needle = query.trim().toLowerCase();
  const items = activity.filter(
    (a) => (filter === "all" || a.kind === filter) && a.label.toLowerCase().includes(needle),
  );

  return (
    <div className="page">
      <header className="page-head">
        <h1>History</h1>
        <p className="muted">Everything sent since Continue was opened.</p>
      </header>

      <div className="toolbar">
        <Segmented label="Show" options={HISTORY_FILTERS} value={filter} onChange={setFilter} />
        <label className="search">
          <Search size={15} />
          <input
            type="search"
            value={query}
            placeholder="Search"
            aria-label="Search history"
            onChange={(e) => setQuery(e.target.value)}
          />
        </label>
      </div>

      {items.length === 0 ? (
        <p className="muted">{needle ? `Nothing matches “${query.trim()}”.` : "Nothing sent yet."}</p>
      ) : (
        <ul className="list">
          {items.map((item) => (
            <ActivityRow key={item.id} item={item} onCopy={onCopy} />
          ))}
        </ul>
      )}
    </div>
  );
}

interface SettingsPageProps {
  identity: DeviceIdentity | null;
  theme: Theme;
  accent: AccentName;
  onThemeChange: (theme: Theme) => void;
  onAccentChange: (accent: AccentName) => void;
  onCopy: (text: string) => void;
}

function SettingsPage({ identity, theme, accent, onThemeChange, onAccentChange, onCopy }: SettingsPageProps) {
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion).catch(() => setVersion(""));
  }, []);

  return (
    <div className="page">
      <header className="page-head">
        <h1>Settings</h1>
      </header>

      <section className="section">
        <h2>Appearance</h2>
        <div className="panel">
          <Row title="Theme" text="Auto matches your system.">
            <Segmented label="Theme" options={THEMES} value={theme} onChange={onThemeChange} />
          </Row>
          <Row title="Accent colour">
            <div className="swatches" role="radiogroup" aria-label="Accent colour">
              {ACCENT_PALETTE.map((option) => (
                <button
                  key={option.id}
                  type="button"
                  role="radio"
                  aria-checked={accent === option.id}
                  aria-label={option.label}
                  title={option.label}
                  className="swatch"
                  style={{ backgroundColor: option.base }}
                  onClick={() => onAccentChange(option.id)}
                />
              ))}
            </div>
          </Row>
        </div>
      </section>

      {identity && (
        <section className="section">
          <h2>This computer</h2>
          <div className="panel">
            <Row title="Name" text="How your phone sees this computer.">
              <span className="row-value">{identity.deviceName}</span>
            </Row>
            <Row title="Security code" text="When pairing, check your phone shows the same code.">
              <button type="button" className="btn btn-quiet" onClick={() => onCopy(identity.fingerprint)}>
                <Copy size={15} />
                Copy
              </button>
            </Row>
            <code className="code">{identity.fingerprint}</code>
          </div>
        </section>
      )}

      <section className="section">
        <h2>About</h2>
        <div className="panel">
          <Row title={`Continue${version ? ` ${version}` : ""}`} text="Open source. Works on your own network, with no account." />
        </div>
      </section>
    </div>
  );
}

interface DevicePageProps {
  peer: TrustedPeer;
  onBack: () => void;
  onUnpair: () => void;
  onError: (message: string) => void;
}

function DevicePage({ peer, onBack, onUnpair, onError }: DevicePageProps) {
  const [permissions, setPermissions] = useState<PeerPermission[] | null>(null);
  const [confirmingUnpair, setConfirmingUnpair] = useState(false);

  useEffect(() => {
    getPermissions(peer.fingerprint)
      .then(setPermissions)
      .catch((error) => onError(errorMessage(error)));
  }, [peer.fingerprint, onError]);

  useEffect(() => {
    if (!confirmingUnpair) return;
    const timer = window.setTimeout(() => setConfirmingUnpair(false), 3000);
    return () => window.clearTimeout(timer);
  }, [confirmingUnpair]);

  const changeGrant = async (permission: PeerPermission, grant: Grant) => {
    try {
      await setPermission(peer.fingerprint, permission.capabilityId, grant);
      setPermissions((prev) =>
        prev?.map((p) => (p.capabilityId === permission.capabilityId ? { ...p, grant } : p)) ?? null,
      );
    } catch (error) {
      onError(errorMessage(error));
    }
  };

  return (
    <div className="page">
      <header className="page-head">
        <button type="button" className="back" onClick={onBack}>
          <ArrowLeft size={16} />
          Back
        </button>
        <h1>{peer.displayName}</h1>
        <p className="muted">Paired on {formatPairedDate(peer.pairedAt)}</p>
      </header>

      <section className="section">
        <h2>What it can do</h2>
        <div className="panel">
          {permissions?.map((permission) => {
            const copy = PERMISSIONS[permission.capabilityName];
            const label = copy?.label ?? permission.capabilityName;
            return (
              <Row key={permission.capabilityId} title={label} text={copy?.description}>
                <Segmented
                  label={label}
                  options={GRANT_OPTIONS}
                  value={permission.grant}
                  onChange={(grant) => changeGrant(permission, grant)}
                />
              </Row>
            );
          })}
        </div>
      </section>

      <section className="section">
        <h2>Pairing</h2>
        <div className="panel">
          <Row title="Unpair" text="You'll need to scan a new code to use it again.">
            <button
              type="button"
              className={`btn ${confirmingUnpair ? "btn-danger" : "btn-quiet danger"}`}
              onClick={() => (confirmingUnpair ? onUnpair() : setConfirmingUnpair(true))}
            >
              {confirmingUnpair ? "Click again to unpair" : "Unpair"}
            </button>
          </Row>
        </div>
      </section>
    </div>
  );
}
