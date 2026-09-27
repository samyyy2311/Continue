// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowUp,
  Check,
  ChevronDown,
  CircleAlert,
  Copy,
  File,
  FileText,
  Film,
  Image,
  Laptop,
  Loader,
  Music,
  Plus,
  Smartphone,
  Tablet,
} from "lucide-react";
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

type Page = "send" | "devices" | "settings";

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

const PAGES: { id: Page; label: string }[] = [
  { id: "send", label: "Send" },
  { id: "devices", label: "Devices" },
  { id: "settings", label: "Settings" },
];

const ACCENT_KEY = "continue.accent";
const ENDPOINTS_KEY = "continue.peerEndpoints";

// Discovery isn't wired into the desktop yet, so the last address typed for each
// peer is remembered locally to avoid retyping it on every connect.
function loadSavedEndpoints(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem(ENDPOINTS_KEY) ?? "{}");
  } catch {
    return {};
  }
}

function saveEndpoint(fingerprint: string, endpoint: string) {
  try {
    localStorage.setItem(ENDPOINTS_KEY, JSON.stringify({ ...loadSavedEndpoints(), [fingerprint]: endpoint }));
  } catch {
    // Storage can be unavailable (e.g. blocked site data); the address just won't be remembered.
  }
}

function loadAccent(): AccentName {
  try {
    const stored = localStorage.getItem(ACCENT_KEY);
    return ACCENT_PALETTE.some((a) => a.id === stored) ? (stored as AccentName) : "blue";
  } catch {
    return "blue";
  }
}

function DeviceIcon({ name, size = 18 }: { name: string; size?: number }) {
  const lower = name.toLowerCase();
  if (lower.includes("tablet") || lower.includes("ipad")) return <Tablet size={size} />;
  if (["pc", "desktop", "mac", "laptop"].some((word) => lower.includes(word))) return <Laptop size={size} />;
  return <Smartphone size={size} />;
}

const FILE_ICONS: { icon: typeof File; extensions: string[] }[] = [
  { icon: Image, extensions: ["png", "jpg", "jpeg", "gif", "webp", "heic", "dng"] },
  { icon: Music, extensions: ["mp3", "m4a", "wav", "flac", "ogg"] },
  { icon: Film, extensions: ["mp4", "mov", "mkv", "webm"] },
  { icon: FileText, extensions: ["pdf", "doc", "docx", "txt", "md"] },
];

function ActivityIcon({ item }: { item: Activity }) {
  const ext = item.label.toLowerCase().split(".").pop() ?? "";
  const Icon =
    item.kind === "text" ? FileText : (FILE_ICONS.find((k) => k.extensions.includes(ext))?.icon ?? File);
  return (
    <span className="activity-icon">
      <Icon size={16} />
    </span>
  );
}

// The backend names capabilities after the protocol; people think in terms of what gets shared.
const PERMISSION_LABELS: Record<string, string> = {
  "File Transfer": "Files",
  "Clipboard Sync": "Clipboard",
  "Notification Relay": "Notifications",
};

const GRANT_OPTIONS: { value: Grant; label: string }[] = [
  { value: "Allow", label: "Allow" },
  { value: "Ask", label: "Ask" },
  { value: "Deny", label: "Block" },
];

interface ConnectControlProps {
  peer: TrustedPeer;
  onChanged: () => void;
  onError: (message: string) => void;
}

function ConnectControl({ peer, onChanged, onError }: ConnectControlProps) {
  const [endpoint, setEndpoint] = useState(() => peer.endpoint ?? loadSavedEndpoints()[peer.fingerprint] ?? "");
  const [pending, setPending] = useState(false);

  const run = async (action: () => Promise<void>) => {
    setPending(true);
    try {
      await action();
      onChanged();
    } catch (error) {
      onError(errorMessage(error));
    } finally {
      setPending(false);
    }
  };

  if (peer.isConnected) {
    return (
      <button
        type="button"
        className="btn btn-quiet"
        disabled={pending}
        onClick={() => run(() => disconnectPeer(peer.fingerprint))}
      >
        Disconnect
      </button>
    );
  }

  return (
    <form
      className="connect-form"
      onSubmit={(e) => {
        e.preventDefault();
        const address = endpoint.trim();
        run(async () => {
          await connectToPeer(peer.fingerprint, address);
          saveEndpoint(peer.fingerprint, address);
        });
      }}
    >
      <input
        className="input"
        value={endpoint}
        onChange={(e) => setEndpoint(e.target.value)}
        placeholder="Address, like 192.168.1.20:4433"
        aria-label={`Address of ${peer.displayName}`}
        spellCheck={false}
      />
      <button type="submit" className="btn btn-primary" disabled={pending || !endpoint.trim()}>
        {pending ? "Connecting…" : "Connect"}
      </button>
    </form>
  );
}

export default function App() {
  const [page, setPage] = useState<Page>("send");
  const [accent, setAccent] = useState<AccentName>(loadAccent);
  const [identity, setIdentity] = useState<DeviceIdentity | null>(null);
  const [peers, setPeers] = useState<TrustedPeer[] | null>(null);
  const [loadError, setLoadError] = useState("");
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);
  const [showPairDialog, setShowPairDialog] = useState(false);
  const [activity, setActivity] = useState<Activity[]>([]);
  const [note, setNote] = useState("");
  const [dragActive, setDragActive] = useState(false);
  const [toast, setToast] = useState<Toast | null>(null);

  const selectedPeer = peers?.find((p) => p.fingerprint === selectedPeerId) ?? peers?.[0] ?? null;

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
    try {
      localStorage.setItem(ACCENT_KEY, accent);
    } catch {
      // Not persisting the accent only affects the next launch.
    }
  }, [accent]);

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
        setPage("send");
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
      setPage("send");
      showToast(`Paired with ${peer.displayName}`);
      refreshPeers();
    },
    [refreshPeers, showToast],
  );

  const closePairDialog = useCallback(() => setShowPairDialog(false), []);

  const removePeer = async (peer: TrustedPeer) => {
    try {
      await removeTrustedPeer(peer.fingerprint);
      showToast(`Removed ${peer.displayName}`);
      refreshPeers();
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const renderSend = () => {
    if (!selectedPeer) {
      return (
        <section className="welcome">
          <h1>Connect your phone</h1>
          <p className="lead">
            Pair once, then send files and text between your devices. Everything stays on your own network.
          </p>
          <button type="button" className="btn btn-primary btn-large" onClick={() => setShowPairDialog(true)}>
            Pair a device
          </button>
        </section>
      );
    }

    const connected = selectedPeer.isConnected;
    return (
      <>
        <section className="target">
          <p className="eyebrow">Sending to</p>
          {peers && peers.length > 1 ? (
            <h1 className="device-picker">
              {selectedPeer.displayName}
              <ChevronDown size={22} aria-hidden="true" />
              <select
                value={selectedPeer.fingerprint}
                onChange={(e) => setSelectedPeerId(e.target.value)}
                aria-label="Device to send to"
              >
                {peers.map((peer) => (
                  <option key={peer.fingerprint} value={peer.fingerprint}>
                    {peer.displayName}
                  </option>
                ))}
              </select>
            </h1>
          ) : (
            <h1>{selectedPeer.displayName}</h1>
          )}
          <div className="target-status">
            <span className={`status ${connected ? "online" : ""}`}>{connected ? "Connected" : "Not connected"}</span>
            <ConnectControl
              key={selectedPeer.fingerprint}
              peer={selectedPeer}
              onChanged={refreshPeers}
              onError={showError}
            />
          </div>
        </section>

        <button
          type="button"
          className={`drop ${dragActive ? "active" : ""}`}
          disabled={!connected}
          onClick={chooseFiles}
        >
          <span className="drop-icon">
            <ArrowUp size={20} />
          </span>
          <span className="drop-title">{connected ? "Drop files here" : "Connect to send files"}</span>
          {connected && <span className="drop-sub">or click to choose</span>}
        </button>

        <form className="compose" onSubmit={sendNote}>
          <input
            className="compose-input"
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="Send a note or link"
            aria-label="Note or link to send"
            disabled={!connected}
          />
          <button
            type="submit"
            className="compose-send"
            disabled={!connected || !note.trim()}
            aria-label="Send"
          >
            <ArrowUp size={16} />
          </button>
        </form>

        {activity.length > 0 && (
          <section className="activity" aria-label="Recent">
            <div className="section-head">
              <h2>Recent</h2>
              {activity.some((a) => a.status !== "sending") && (
                <button
                  type="button"
                  className="link"
                  onClick={() => setActivity((prev) => prev.filter((a) => a.status === "sending"))}
                >
                  Clear
                </button>
              )}
            </div>
            <ul>
              {activity.map((item) => (
                <li key={item.id} className="activity-row">
                  <ActivityIcon item={item} />
                  <span className="activity-main">
                    <span className="activity-label" title={item.label}>
                      {item.label}
                    </span>
                    <span className={`activity-meta ${item.status === "failed" ? "failed" : ""}`}>
                      {item.status === "sending" && "Sending…"}
                      {item.status === "sent" &&
                        [item.bytesSent !== undefined && formatBytes(item.bytesSent), formatRelativeTime(item.timestamp)]
                          .filter(Boolean)
                          .join(" · ")}
                      {item.status === "failed" && `Didn't send: ${item.error}`}
                    </span>
                  </span>
                  {item.status === "sending" && <Loader size={15} className="spin faint" aria-label="Sending" />}
                  {item.status === "sent" && item.kind === "text" && (
                    <button type="button" className="icon-btn" onClick={() => copyText(item.label)} aria-label="Copy">
                      <Copy size={15} />
                    </button>
                  )}
                </li>
              ))}
            </ul>
          </section>
        )}
      </>
    );
  };

  const renderDevices = () => (
    <>
      <div className="page-head">
        <h1>Devices</h1>
        <button type="button" className="btn btn-primary" onClick={() => setShowPairDialog(true)}>
          <Plus size={16} />
          Pair
        </button>
      </div>
      {peers?.length === 0 ? (
        <p className="lead">Nothing paired yet.</p>
      ) : (
        <ul className="list">
          {peers?.map((peer) => (
            <DeviceRow
              key={peer.fingerprint}
              peer={peer}
              onChanged={refreshPeers}
              onRemove={() => removePeer(peer)}
              onError={showError}
            />
          ))}
        </ul>
      )}
    </>
  );

  const renderSettings = () => (
    <>
      <div className="page-head">
        <h1>Settings</h1>
      </div>
      <ul className="list">
        <li className="setting">
          <span className="setting-label">Accent color</span>
          <div className="swatches" role="radiogroup" aria-label="Accent color">
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
                onClick={() => setAccent(option.id)}
              />
            ))}
          </div>
        </li>
        {identity && (
          <>
            <li className="setting">
              <span className="setting-label">Computer name</span>
              <span className="muted">{identity.deviceName}</span>
            </li>
            <li className="setting setting-stacked">
              <div className="setting-row">
                <span>
                  <span className="setting-label">Security code</span>
                  <span className="setting-help">Check it matches on your phone when you pair.</span>
                </span>
                <button type="button" className="btn btn-quiet" onClick={() => copyText(identity.fingerprint)}>
                  <Copy size={14} />
                  Copy
                </button>
              </div>
              <code className="code">{identity.fingerprint}</code>
            </li>
          </>
        )}
      </ul>
    </>
  );

  const renderUnavailable = () => (
    <section className="welcome" role="alert">
      <h1>Continue isn't running</h1>
      <p className="lead">
        {isTauri() ? loadError : "Open this window from the Continue app, or start it with pnpm tauri dev."}
      </p>
    </section>
  );

  const ready = isTauri() && !loadError && peers !== null;

  return (
    <div className="app">
      <header className="topbar">
        <span className="brand">
          <img src="/icon.svg" alt="" width={20} height={20} />
          Continue
        </span>
        {ready && (
          <nav className="tabs" aria-label="Main">
            {PAGES.map((item) => (
              <button
                key={item.id}
                type="button"
                className="tab"
                aria-current={page === item.id ? "page" : undefined}
                onClick={() => setPage(item.id)}
              >
                {item.label}
              </button>
            ))}
          </nav>
        )}
      </header>

      <main className="content">
        {!isTauri() || loadError
          ? renderUnavailable()
          : ready && (
              <>
                {page === "send" && renderSend()}
                {page === "devices" && renderDevices()}
                {page === "settings" && renderSettings()}
              </>
            )}
      </main>

      {showPairDialog && <PairDialog onPaired={handlePaired} onClose={closePairDialog} />}

      {toast && (
        <div className={`toast ${toast.tone}`} role={toast.tone === "error" ? "alert" : "status"}>
          {toast.tone === "error" ? <CircleAlert size={16} /> : <Check size={16} />}
          <span>{toast.message}</span>
        </div>
      )}
    </div>
  );
}

interface DeviceRowProps {
  peer: TrustedPeer;
  onChanged: () => void;
  onRemove: () => void;
  onError: (message: string) => void;
}

function DeviceRow({ peer, onChanged, onRemove, onError }: DeviceRowProps) {
  const [expanded, setExpanded] = useState(false);
  const [permissions, setPermissions] = useState<PeerPermission[] | null>(null);
  const [confirmingRemove, setConfirmingRemove] = useState(false);

  useEffect(() => {
    if (!expanded) return;
    getPermissions(peer.fingerprint)
      .then(setPermissions)
      .catch((error) => onError(errorMessage(error)));
  }, [expanded, peer.fingerprint, onError]);

  useEffect(() => {
    if (!confirmingRemove) return;
    const timer = window.setTimeout(() => setConfirmingRemove(false), 3000);
    return () => window.clearTimeout(timer);
  }, [confirmingRemove]);

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

  const panelId = `permissions-${peer.fingerprint}`;

  return (
    <li className="device">
      <div className="device-row">
        <span className="device-icon">
          <DeviceIcon name={peer.displayName} />
        </span>
        <span className="device-main">
          <span className="device-name">{peer.displayName}</span>
          <span className={`status ${peer.isConnected ? "online" : ""}`}>
            {peer.isConnected ? "Connected" : `Added ${formatPairedDate(peer.pairedAt)}`}
          </span>
        </span>
        <button
          type="button"
          className="btn btn-quiet"
          aria-expanded={expanded}
          aria-controls={panelId}
          onClick={() => setExpanded((v) => !v)}
        >
          {expanded ? "Done" : "Manage"}
        </button>
      </div>
      {expanded && (
        <div id={panelId} className="device-panel">
          <ConnectControl peer={peer} onChanged={onChanged} onError={onError} />
          {permissions?.map((permission) => (
            <div key={permission.capabilityId} className="permission">
              <span>{PERMISSION_LABELS[permission.capabilityName] ?? permission.capabilityName}</span>
              <div className="segmented" role="radiogroup" aria-label={permission.capabilityName}>
                {GRANT_OPTIONS.map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    role="radio"
                    aria-checked={permission.grant === option.value}
                    onClick={() => changeGrant(permission, option.value)}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </div>
          ))}
          <button
            type="button"
            className={`btn ${confirmingRemove ? "btn-danger" : "btn-quiet danger"}`}
            onClick={() => {
              if (confirmingRemove) {
                onRemove();
              } else {
                setConfirmingRemove(true);
              }
            }}
          >
            {confirmingRemove ? "Click again to remove" : "Remove device"}
          </button>
        </div>
      )}
    </li>
  );
}
