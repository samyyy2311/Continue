// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowUp,
  Check,
  CircleAlert,
  Copy,
  History,
  Laptop,
  Loader,
  Plus,
  Search,
  Send,
  Settings,
  SlidersHorizontal,
  Smartphone,
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
  peerName: string;
  status: "sending" | "sent" | "failed";
  timestamp: number;
  bytesSent?: number;
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
      <span className="tile tile-text" aria-hidden="true">
        <Type size={16} />
      </span>
    );
  }
  const ext = item.label.includes(".") ? (item.label.split(".").pop() ?? "").toLowerCase() : "";
  const tone = FILE_KINDS.find((k) => k.extensions.includes(ext))?.tone ?? "other";
  return (
    <span className={`tile tile-${tone}`} aria-hidden="true">
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
  const [dragActive, setDragActive] = useState(false);
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

  const track = async (item: Omit<Activity, "id" | "status" | "timestamp">, send: () => Promise<number | void>) => {
    const id = crypto.randomUUID();
    setActivity((prev) => [{ ...item, id, status: "sending", timestamp: Date.now() }, ...prev]);
    const update = (patch: Partial<Activity>) =>
      setActivity((prev) => prev.map((a) => (a.id === id ? { ...a, ...patch } : a)));
    try {
      const bytesSent = await send();
      update({ status: "sent", bytesSent: bytesSent ?? undefined });
      return true;
    } catch (error) {
      update({ status: "failed", error: errorMessage(error) });
      return false;
    }
  };

  const sendFiles = async (paths: string[]) => {
    if (!selectedPeer?.isConnected) {
      showError(selectedPeer ? `Connect to ${selectedPeer.displayName} first.` : "Pair a device first.");
      return;
    }
    const peer = selectedPeer;
    for (const path of paths) {
      await track({ kind: "file", label: fileNameFromPath(path), peerName: peer.displayName }, () =>
        sendFileToPeer(peer.fingerprint, path),
      );
    }
  };

  // The webview listener is registered once, so it reads the latest sender through a ref.
  const sendFilesRef = useRef(sendFiles);
  sendFilesRef.current = sendFiles;

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter" || payload.type === "over") {
        setView({ name: "send" });
        setDragActive(true);
        return;
      }
      setDragActive(false);
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
    const sent = await track({ kind: "text", label: text, peerName: selectedPeer.displayName }, () =>
      sendClipboardText(selectedPeer.fingerprint, text),
    );
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
      <div className="welcome-art" aria-hidden="true">
        <span className="welcome-device">
          <Laptop size={30} strokeWidth={1.5} />
        </span>
        <span className="welcome-link" />
        <span className="welcome-device">
          <Smartphone size={28} strokeWidth={1.5} />
        </span>
      </div>
      <h1>{title}</h1>
      <p className="lead">{text}</p>
      {action}
    </div>
  );

  const renderSend = () => {
    if (!selectedPeer) {
      return renderWelcome(
        "Pair your phone",
        "Scan a code once, then send files and text between your phone and this computer. Everything stays on your own network.",
        <button type="button" className="btn btn-primary btn-large" onClick={openPairDialog}>
          Pair a device
        </button>,
      );
    }

    const peer = selectedPeer;
    const recent = activity.slice(0, 4);
    return (
      <div className="page">
        <header className="device-head">
          <span className={`device-badge ${peer.isConnected ? "online" : ""}`} aria-hidden="true">
            <Smartphone size={22} strokeWidth={1.75} />
          </span>
          <div className="device-head-text">
            <h1>{peer.displayName}</h1>
            <p className={`status ${peer.isConnected ? "online" : ""}`}>
              {peer.isConnected ? "Connected" : "Not connected"}
            </p>
          </div>
          {peer.isConnected && (
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

        {peer.isConnected ? (
          <>
            <button type="button" className={`drop ${dragActive ? "active" : ""}`} onClick={chooseFiles}>
              <span className="drop-icon">
                <Upload size={20} />
              </span>
              <span className="drop-title">Drop files to send</span>
              <span className="drop-sub">or click to choose them</span>
            </button>

            <form className="compose" onSubmit={sendNote}>
              <input
                value={note}
                onChange={(e) => setNote(e.target.value)}
                placeholder="Send text to your phone's clipboard"
                aria-label="Text to send"
              />
              <button type="submit" className="compose-send" disabled={!note.trim()} aria-label="Send">
                <ArrowUp size={17} />
              </button>
            </form>
          </>
        ) : (
          <form className="connect" onSubmit={connect}>
            <div>
              <p className="row-title">Connect to start sending</p>
              <p className="row-sub">Open Continue on your phone and enter the address it shows.</p>
            </div>
            <div className="connect-fields">
              <input
                className="input"
                value={endpoint}
                onChange={(e) => setEndpointDrafts((prev) => ({ ...prev, [peer.fingerprint]: e.target.value }))}
                placeholder="192.168.1.20:4433"
                aria-label="Phone address"
                spellCheck={false}
              />
              <button
                type="submit"
                className="btn btn-primary"
                disabled={connecting === peer.fingerprint || !endpoint.trim()}
              >
                {connecting === peer.fingerprint && <Loader size={15} className="spin" />}
                {connecting === peer.fingerprint ? "Connecting" : "Connect"}
              </button>
            </div>
          </form>
        )}

        <section className="section">
          <div className="section-head">
            <h2>Recent</h2>
            {activity.length > recent.length && (
              <button type="button" className="link" onClick={() => setView({ name: "history" })}>
                See all
              </button>
            )}
          </div>
          {recent.length === 0 ? (
            <p className="muted">Files and text you send will show up here.</p>
          ) : (
            <ul className="list">
              {recent.map((item) => (
                <ActivityRow key={item.id} item={item} onCopy={copyText} />
              ))}
            </ul>
          )}
        </section>
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
