// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowUpDown,
  Check,
  CircleAlert,
  Copy,
  File,
  FileText,
  Film,
  Home,
  Image,
  Laptop,
  Loader,
  MonitorSmartphone,
  Music,
  Plus,
  Send,
  Settings,
  Smartphone,
  Tablet,
  Upload,
} from "lucide-react";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
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
import { fileNameFromPath, formatBytes, formatPairedDate, formatRelativeTime, shortFingerprint } from "./format.ts";
import { PairDialog } from "./PairDialog.tsx";
import {
  ACCENT_PALETTE,
  type AccentName,
  type DeviceIdentity,
  type Grant,
  type PeerPermission,
  type TransferHistoryItem,
  type TrustedPeer,
} from "./types.ts";

type Page = "home" | "transfers" | "devices" | "settings";

interface Toast {
  message: string;
  tone: "info" | "error";
}

interface SentClip {
  id: string;
  text: string;
  peerName: string;
  timestamp: number;
}

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

function FileIcon({ fileName }: { fileName: string }) {
  const ext = fileName.toLowerCase().split(".").pop() ?? "";
  if (["png", "jpg", "jpeg", "gif", "webp", "heic", "dng"].includes(ext)) return <Image size={18} />;
  if (["mp3", "m4a", "wav", "flac", "ogg"].includes(ext)) return <Music size={18} />;
  if (["mp4", "mov", "mkv", "webm"].includes(ext)) return <Film size={18} />;
  if (["pdf", "doc", "docx", "txt", "md"].includes(ext)) return <FileText size={18} />;
  return <File size={18} />;
}

function StatusDot({ connected }: { connected: boolean }) {
  return <span className={`status-dot ${connected ? "online" : ""}`} aria-hidden="true" />;
}

interface InlineConfirmButtonProps {
  label: string;
  confirmLabel: string;
  onConfirm: () => void;
}

/** Destructive action that needs a second click within a few seconds. */
function InlineConfirmButton({ label, confirmLabel, onConfirm }: InlineConfirmButtonProps) {
  const [confirming, setConfirming] = useState(false);

  useEffect(() => {
    if (!confirming) return;
    const timer = window.setTimeout(() => setConfirming(false), 3000);
    return () => window.clearTimeout(timer);
  }, [confirming]);

  return (
    <button
      type="button"
      className={`btn btn-sm ${confirming ? "btn-danger" : "btn-ghost-danger"}`}
      onClick={() => {
        if (confirming) {
          setConfirming(false);
          onConfirm();
        } else {
          setConfirming(true);
        }
      }}
    >
      {confirming ? confirmLabel : label}
    </button>
  );
}

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
        className="btn btn-secondary btn-sm"
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
        className="input input-sm mono"
        value={endpoint}
        onChange={(e) => setEndpoint(e.target.value)}
        placeholder="192.168.1.20:4433"
        aria-label={`Network address of ${peer.displayName}`}
        spellCheck={false}
      />
      <button type="submit" className="btn btn-primary btn-sm" disabled={pending || !endpoint.trim()}>
        {pending ? "Connecting…" : "Connect"}
      </button>
    </form>
  );
}

export default function App() {
  const [page, setPage] = useState<Page>("home");
  const [accent, setAccent] = useState<AccentName>(loadAccent);
  const [identity, setIdentity] = useState<DeviceIdentity | null>(null);
  const [peers, setPeers] = useState<TrustedPeer[] | null>(null);
  const [loadError, setLoadError] = useState("");
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);
  const [showPairDialog, setShowPairDialog] = useState(false);
  const [transfers, setTransfers] = useState<TransferHistoryItem[]>([]);
  const [sentClips, setSentClips] = useState<SentClip[]>([]);
  const [clipText, setClipText] = useState("");
  const [sendingClip, setSendingClip] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [toast, setToast] = useState<Toast | null>(null);

  const selectedPeer = peers?.find((p) => p.fingerprint === selectedPeerId) ?? peers?.[0] ?? null;

  const showToast = useCallback((message: string, tone: Toast["tone"] = "info") => {
    setToast({ message, tone });
  }, []);
  const showError = useCallback((message: string) => showToast(message, "error"), [showToast]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), toast.tone === "error" ? 6000 : 3000);
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
    Promise.all([getDeviceIdentity(), getTrustedPeers()])
      .then(([loadedIdentity, loadedPeers]) => {
        setIdentity(loadedIdentity);
        setPeers(loadedPeers);
      })
      .catch((error) => setLoadError(errorMessage(error)));
  }, []);

  const sendFiles = async (paths: string[]) => {
    if (!selectedPeer?.isConnected) {
      showError(selectedPeer ? `Connect to ${selectedPeer.displayName} to send files.` : "Pair a device first.");
      return;
    }
    const peer = selectedPeer;
    for (const path of paths) {
      const id = crypto.randomUUID();
      setTransfers((prev) => [
        {
          id,
          fileName: fileNameFromPath(path),
          peerFingerprint: peer.fingerprint,
          status: "in_progress",
          timestamp: Date.now(),
        },
        ...prev,
      ]);
      const update = (patch: Partial<TransferHistoryItem>) =>
        setTransfers((prev) => prev.map((tx) => (tx.id === id ? { ...tx, ...patch } : tx)));
      try {
        update({ status: "completed", bytesSent: await sendFileToPeer(peer.fingerprint, path) });
      } catch (error) {
        update({ status: "failed", error: errorMessage(error) });
      }
    }
  };

  // The webview listener is registered once, so it reads the latest sender through a ref.
  const sendFilesRef = useRef(sendFiles);
  sendFilesRef.current = sendFiles;

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter" || payload.type === "over") {
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

  const sendClip = async (e: React.FormEvent) => {
    e.preventDefault();
    const text = clipText;
    if (!selectedPeer || !text.trim()) return;
    setSendingClip(true);
    try {
      await sendClipboardText(selectedPeer.fingerprint, text);
      setSentClips((prev) =>
        [{ id: crypto.randomUUID(), text, peerName: selectedPeer.displayName, timestamp: Date.now() }, ...prev].slice(
          0,
          5,
        ),
      );
      setClipText("");
    } catch (error) {
      showError(errorMessage(error));
    } finally {
      setSendingClip(false);
    }
  };

  const copyText = async (text: string, confirmation: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showToast(confirmation);
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const handlePaired = useCallback(
    (peer: TrustedPeer) => {
      setShowPairDialog(false);
      setSelectedPeerId(peer.fingerprint);
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

  const peerName = (fingerprint: string) =>
    peers?.find((p) => p.fingerprint === fingerprint)?.displayName ?? "removed device";
  const activeTransfers = transfers.filter((tx) => tx.status === "in_progress").length;

  const navItems: { id: Page; label: string; icon: React.ReactNode }[] = [
    { id: "home", label: "Home", icon: <Home size={17} /> },
    { id: "transfers", label: "Transfers", icon: <ArrowUpDown size={17} /> },
    { id: "devices", label: "Devices", icon: <MonitorSmartphone size={17} /> },
    { id: "settings", label: "Settings", icon: <Settings size={17} /> },
  ];

  const renderTransferRow = (tx: TransferHistoryItem) => (
    <li key={tx.id} className="list-row">
      <span className="row-icon">
        <FileIcon fileName={tx.fileName} />
      </span>
      <span className="row-main">
        <span className="row-title" title={tx.fileName}>
          {tx.fileName}
        </span>
        <span className={`row-sub ${tx.status === "failed" ? "danger-text" : ""}`}>
          {tx.status === "in_progress" && `Sending to ${peerName(tx.peerFingerprint)}…`}
          {tx.status === "completed" &&
            `Sent to ${peerName(tx.peerFingerprint)} · ${formatBytes(tx.bytesSent ?? 0)} · ${formatRelativeTime(tx.timestamp)}`}
          {tx.status === "failed" && tx.error}
        </span>
      </span>
      <span className="row-trailing">
        {tx.status === "in_progress" && <Loader size={16} className="spin" aria-label="Sending" />}
        {tx.status === "completed" && <Check size={16} className="success-text" aria-label="Sent" />}
        {tx.status === "failed" && <CircleAlert size={16} className="danger-text" aria-label="Failed" />}
      </span>
    </li>
  );

  const renderHome = () => {
    if (!selectedPeer) {
      return (
        <div className="empty-hero">
          <span className="empty-hero-icon">
            <MonitorSmartphone size={28} />
          </span>
          <h1>Pair your first device</h1>
          <p>
            Continue connects your phone, tablet and computers directly over your local network. Nothing goes through
            the cloud.
          </p>
          <button type="button" className="btn btn-primary" onClick={() => setShowPairDialog(true)}>
            <Plus size={16} />
            Pair a device
          </button>
        </div>
      );
    }

    const connected = selectedPeer.isConnected;
    return (
      <>
        <header className="page-header">
          <div className="device-heading">
            <span className="device-avatar">
              <DeviceIcon name={selectedPeer.displayName} size={22} />
            </span>
            <div>
              {peers && peers.length > 1 ? (
                <select
                  className="device-select"
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
              ) : (
                <h1>{selectedPeer.displayName}</h1>
              )}
              <p className="status-line">
                <StatusDot connected={connected} />
                {connected ? "Connected" : "Not connected"}
              </p>
            </div>
          </div>
          <ConnectControl
            key={selectedPeer.fingerprint}
            peer={selectedPeer}
            onChanged={refreshPeers}
            onError={showError}
          />
        </header>

        {!connected && (
          <p className="notice">
            Enter the address shown in Continue on {selectedPeer.displayName} and connect to start sending.
          </p>
        )}

        <section className="section" aria-labelledby="send-files-title">
          <h2 id="send-files-title" className="section-title">
            Send files
          </h2>
          <div className={`dropzone ${dragActive ? "active" : ""} ${connected ? "" : "disabled"}`}>
            <Upload size={22} className="dropzone-icon" />
            <p className="dropzone-title">Drop files anywhere in this window</p>
            <p className="muted">They go straight to {selectedPeer.displayName} over your network.</p>
            <button type="button" className="btn btn-secondary" disabled={!connected} onClick={chooseFiles}>
              Choose files…
            </button>
          </div>
        </section>

        <section className="section" aria-labelledby="send-text-title">
          <h2 id="send-text-title" className="section-title">
            Send text
          </h2>
          <form className="clip-form" onSubmit={sendClip}>
            <textarea
              className="input"
              rows={3}
              value={clipText}
              onChange={(e) => setClipText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  e.currentTarget.form?.requestSubmit();
                }
              }}
              placeholder={`Text or a link to put on ${selectedPeer.displayName}'s clipboard`}
              aria-label="Text to send"
              disabled={!connected}
            />
            <div className="clip-form-footer">
              <span className="hint">Enter to send · Shift+Enter for a new line</span>
              <button type="submit" className="btn btn-primary" disabled={!connected || sendingClip || !clipText.trim()}>
                <Send size={15} />
                {sendingClip ? "Sending…" : "Send"}
              </button>
            </div>
          </form>
          {sentClips.length > 0 && (
            <ul className="list" aria-label="Recently sent text">
              {sentClips.map((clip) => (
                <li key={clip.id} className="list-row">
                  <span className="row-main">
                    <span className="row-title">{clip.text}</span>
                    <span className="row-sub">
                      Sent to {clip.peerName} · {formatRelativeTime(clip.timestamp)}
                    </span>
                  </span>
                  <button
                    type="button"
                    className="icon-btn"
                    onClick={() => copyText(clip.text, "Copied to clipboard")}
                    aria-label="Copy text"
                  >
                    <Copy size={15} />
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        {transfers.length > 0 && (
          <section className="section" aria-labelledby="recent-transfers-title">
            <div className="section-heading">
              <h2 id="recent-transfers-title" className="section-title">
                Recent transfers
              </h2>
              <button type="button" className="link-btn" onClick={() => setPage("transfers")}>
                View all
              </button>
            </div>
            <ul className="list">{transfers.slice(0, 3).map(renderTransferRow)}</ul>
          </section>
        )}
      </>
    );
  };

  const renderTransfers = () => (
    <>
      <header className="page-header">
        <div>
          <h1>Transfers</h1>
          <p className="muted">Files sent from this computer since Continue was opened.</p>
        </div>
        {transfers.some((tx) => tx.status !== "in_progress") && (
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            onClick={() => setTransfers((prev) => prev.filter((tx) => tx.status === "in_progress"))}
          >
            Clear finished
          </button>
        )}
      </header>
      {transfers.length === 0 ? (
        <div className="empty-block">
          <ArrowUpDown size={22} />
          <p>No files sent yet. Drop files on this window or use Choose files on Home.</p>
        </div>
      ) : (
        <ul className="list">{transfers.map(renderTransferRow)}</ul>
      )}
    </>
  );

  const renderDevices = () => (
    <>
      <header className="page-header">
        <div>
          <h1>Devices</h1>
          <p className="muted">Devices you have paired. Each one is verified by its own key.</p>
        </div>
        <button type="button" className="btn btn-primary" onClick={() => setShowPairDialog(true)}>
          <Plus size={16} />
          Pair a device
        </button>
      </header>
      {peers?.length === 0 ? (
        <div className="empty-block">
          <MonitorSmartphone size={22} />
          <p>No paired devices yet.</p>
        </div>
      ) : (
        <ul className="device-list">
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
      <header className="page-header">
        <div>
          <h1>Settings</h1>
        </div>
      </header>

      <section className="section" aria-labelledby="appearance-title">
        <h2 id="appearance-title" className="section-title">
          Appearance
        </h2>
        <div className="panel">
          <div className="panel-row">
            <div>
              <p className="row-title">Accent color</p>
              <p className="row-sub">Used for buttons and highlights.</p>
            </div>
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
                >
                  {accent === option.id && <Check size={14} color={option.onBase} />}
                </button>
              ))}
            </div>
          </div>
        </div>
      </section>

      <section className="section" aria-labelledby="this-computer-title">
        <h2 id="this-computer-title" className="section-title">
          This computer
        </h2>
        {identity && (
          <div className="panel">
            <div className="panel-row">
              <div>
                <p className="row-title">Name</p>
                <p className="row-sub">Shown to devices you pair with.</p>
              </div>
              <span>{identity.deviceName}</span>
            </div>
            <div className="panel-row stacked">
              <div className="panel-row-head">
                <div>
                  <p className="row-title">Device fingerprint</p>
                  <p className="row-sub">Compare this with what your other device shows when pairing.</p>
                </div>
                <button
                  type="button"
                  className="btn btn-secondary btn-sm"
                  onClick={() => copyText(identity.fingerprint, "Fingerprint copied")}
                >
                  <Copy size={14} />
                  Copy
                </button>
              </div>
              <code className="key-value">{identity.fingerprint}</code>
            </div>
            <div className="panel-row stacked">
              <p className="row-title">Certificate hash</p>
              <code className="key-value">{identity.spkiHash}</code>
            </div>
          </div>
        )}
      </section>
    </>
  );

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            C
          </span>
          Continue
        </div>
        <nav className="nav" aria-label="Main">
          {navItems.map((item) => (
            <button
              key={item.id}
              type="button"
              className={`nav-item ${page === item.id ? "active" : ""}`}
              aria-current={page === item.id ? "page" : undefined}
              aria-label={item.label}
              title={item.label}
              onClick={() => setPage(item.id)}
            >
              {item.icon}
              <span>{item.label}</span>
              {item.id === "transfers" && activeTransfers > 0 && (
                <span className="nav-count" aria-label={`${activeTransfers} sending`}>
                  {activeTransfers}
                </span>
              )}
            </button>
          ))}
        </nav>
        {identity && (
          <div className="this-device">
            <Laptop size={16} />
            <div>
              <p className="row-title">{identity.deviceName}</p>
              <p className="row-sub mono">{shortFingerprint(identity.fingerprint)}</p>
            </div>
          </div>
        )}
      </aside>

      <main className="content">
        <div className="content-inner">
          {loadError ? (
            <div className="empty-block" role="alert">
              <CircleAlert size={22} />
              <p>Continue couldn't start its local service: {loadError}</p>
            </div>
          ) : peers === null ? null : (
            <>
              {page === "home" && renderHome()}
              {page === "transfers" && renderTransfers()}
              {page === "devices" && renderDevices()}
              {page === "settings" && renderSettings()}
            </>
          )}
        </div>
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

const GRANT_OPTIONS: { value: Grant; label: string }[] = [
  { value: "Allow", label: "Allow" },
  { value: "Ask", label: "Ask" },
  { value: "Deny", label: "Block" },
];

function DeviceRow({ peer, onChanged, onRemove, onError }: DeviceRowProps) {
  const [expanded, setExpanded] = useState(false);
  const [permissions, setPermissions] = useState<PeerPermission[] | null>(null);

  useEffect(() => {
    if (!expanded) return;
    getPermissions(peer.fingerprint)
      .then(setPermissions)
      .catch((error) => onError(errorMessage(error)));
  }, [expanded, peer.fingerprint, onError]);

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
    <li className="device-card">
      <div className="device-card-main">
        <span className="device-avatar">
          <DeviceIcon name={peer.displayName} />
        </span>
        <div className="row-main">
          <span className="row-title">{peer.displayName}</span>
          <span className="row-sub">
            <StatusDot connected={peer.isConnected} />
            {peer.isConnected ? "Connected" : "Not connected"} · Paired {formatPairedDate(peer.pairedAt)}
          </span>
        </div>
        <ConnectControl peer={peer} onChanged={onChanged} onError={onError} />
      </div>
      <div className="device-card-footer">
        <button
          type="button"
          className="link-btn"
          aria-expanded={expanded}
          aria-controls={panelId}
          onClick={() => setExpanded((v) => !v)}
        >
          {expanded ? "Hide permissions" : "Permissions"}
        </button>
        <span className="mono muted small" title={peer.fingerprint}>
          {shortFingerprint(peer.fingerprint)}
        </span>
      </div>
      {expanded && (
        <div id={panelId} className="permissions">
          {permissions?.map((permission) => (
            <div key={permission.capabilityId} className="permission-row">
              <span>{permission.capabilityName}</span>
              <div className="segmented segmented-sm" role="radiogroup" aria-label={permission.capabilityName}>
                {GRANT_OPTIONS.map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    role="radio"
                    aria-checked={permission.grant === option.value}
                    className={permission.grant === option.value ? "active" : ""}
                    onClick={() => changeGrant(permission, option.value)}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </div>
          ))}
          <div className="permission-row">
            <span className="muted">Remove this device and forget its key.</span>
            <InlineConfirmButton label="Remove device" confirmLabel="Click again to remove" onConfirm={onRemove} />
          </div>
        </div>
      )}
    </li>
  );
}
