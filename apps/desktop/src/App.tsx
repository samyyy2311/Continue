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
  Lock,
  MonitorSmartphone,
  Music,
  Plus,
  Send,
  Settings,
  Smartphone,
  Tablet,
  Upload,
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

const FILE_KINDS = [
  { tone: "image", icon: Image, extensions: ["png", "jpg", "jpeg", "gif", "webp", "heic", "dng"] },
  { tone: "audio", icon: Music, extensions: ["mp3", "m4a", "wav", "flac", "ogg"] },
  { tone: "video", icon: Film, extensions: ["mp4", "mov", "mkv", "webm"] },
  { tone: "doc", icon: FileText, extensions: ["pdf", "doc", "docx", "txt", "md"] },
];

function FileTile({ fileName }: { fileName: string }) {
  const ext = fileName.toLowerCase().split(".").pop() ?? "";
  const kind = FILE_KINDS.find((k) => k.extensions.includes(ext));
  const Icon = kind?.icon ?? File;
  return (
    <span className={`file-tile ${kind ? `tone-${kind.tone}` : ""}`}>
      <Icon size={17} />
    </span>
  );
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

interface DeviceStageProps {
  identity: DeviceIdentity | null;
  peer: TrustedPeer | null;
  children: React.ReactNode;
}

/** This computer and the selected device, with the link between them showing connection state. */
function DeviceStage({ identity, peer, children }: DeviceStageProps) {
  const connected = peer?.isConnected ?? false;
  return (
    <section className="stage" aria-label="Connection">
      <div className="stage-devices">
        <div className="stage-device">
          <span className="stage-icon">
            <Laptop size={26} strokeWidth={1.6} />
          </span>
          <span className="stage-name">{identity?.deviceName ?? "This computer"}</span>
          <span className="stage-role">This computer</span>
        </div>

        <div className={`stage-link ${connected ? "connected" : ""}`}>
          <span className="stage-link-line" aria-hidden="true" />
          <span className="stage-link-label">
            {connected ? <Lock size={12} /> : null}
            {peer ? (connected ? "Encrypted link" : "Not connected") : "Not paired"}
          </span>
        </div>

        <div className={`stage-device ${peer ? "" : "placeholder"}`}>
          <span className="stage-icon">
            {peer ? (
              <DeviceIcon name={peer.displayName} size={26} />
            ) : (
              <Smartphone size={26} strokeWidth={1.6} />
            )}
          </span>
          <span className="stage-name">{peer?.displayName ?? "Your phone"}</span>
          <span className="stage-role">
            {peer ? (
              <>
                <StatusDot connected={connected} />
                {connected ? "Connected" : "Offline"}
              </>
            ) : (
              "Waiting to pair"
            )}
          </span>
        </div>
      </div>
      <div className="stage-footer">{children}</div>
    </section>
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
    if (!isTauri()) return;
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
    if (!isTauri()) return;
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
      setPage("home");
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
    { id: "home", label: "Home", icon: <Home size={16} /> },
    { id: "transfers", label: "Transfers", icon: <ArrowUpDown size={16} /> },
    { id: "devices", label: "Devices", icon: <MonitorSmartphone size={16} /> },
    { id: "settings", label: "Settings", icon: <Settings size={16} /> },
  ];

  const renderTransferRow = (tx: TransferHistoryItem) => (
    <li key={tx.id} className="row">
      <FileTile fileName={tx.fileName} />
      <span className="row-main">
        <span className="row-title" title={tx.fileName}>
          {tx.fileName}
        </span>
        <span className={`row-sub ${tx.status === "failed" ? "danger-text" : ""}`}>
          {tx.status === "in_progress" && `Sending to ${peerName(tx.peerFingerprint)}…`}
          {tx.status === "completed" &&
            `${formatBytes(tx.bytesSent ?? 0)} · to ${peerName(tx.peerFingerprint)} · ${formatRelativeTime(tx.timestamp)}`}
          {tx.status === "failed" && tx.error}
        </span>
      </span>
      {tx.status === "in_progress" && <Loader size={16} className="spin muted" aria-label="Sending" />}
      {tx.status === "completed" && (
        <span className="badge badge-success">
          <Check size={12} />
          Sent
        </span>
      )}
      {tx.status === "failed" && <span className="badge badge-danger">Failed</span>}
    </li>
  );

  const renderHome = () => {
    if (!selectedPeer) {
      return (
        <div className="home">
          <DeviceStage identity={identity} peer={null}>
            <div className="onboarding">
              <h1>Pair your first device</h1>
              <p className="muted">
                Continue links your phone and computer directly over your own network. Nothing passes through the
                cloud.
              </p>
              <ol className="steps">
                <li>Open Continue on your phone</li>
                <li>Tap Pair a device</li>
                <li>Scan the code this computer shows</li>
              </ol>
              <button type="button" className="btn btn-primary btn-lg" onClick={() => setShowPairDialog(true)}>
                <Plus size={16} />
                Pair a device
              </button>
            </div>
          </DeviceStage>
        </div>
      );
    }

    const connected = selectedPeer.isConnected;
    return (
      <div className="home">
        <DeviceStage identity={identity} peer={selectedPeer}>
          {connected ? (
            <p className="muted">Files and text you send go straight to {selectedPeer.displayName}.</p>
          ) : (
            <p className="muted">Enter the address shown in Continue on {selectedPeer.displayName} to connect.</p>
          )}
          <ConnectControl
            key={selectedPeer.fingerprint}
            peer={selectedPeer}
            onChanged={refreshPeers}
            onError={showError}
          />
        </DeviceStage>

        <div className="send-grid">
          <section className="card" aria-labelledby="send-files-title">
            <header className="card-header">
              <h2 id="send-files-title">Send files</h2>
            </header>
            <div className={`dropzone ${dragActive ? "active" : ""} ${connected ? "" : "disabled"}`}>
              <span className="dropzone-icon">
                <Upload size={20} />
              </span>
              <p className="dropzone-title">Drop files anywhere</p>
              <p className="muted small">or pick them from this computer</p>
              <button type="button" className="btn btn-secondary btn-sm" disabled={!connected} onClick={chooseFiles}>
                Choose files
              </button>
            </div>
          </section>

          <section className="card" aria-labelledby="send-text-title">
            <header className="card-header">
              <h2 id="send-text-title">Send text</h2>
              <span className="hint">⏎ to send · ⇧⏎ new line</span>
            </header>
            <form className="clip-form" onSubmit={sendClip}>
              <textarea
                className="input"
                rows={4}
                value={clipText}
                onChange={(e) => setClipText(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    e.currentTarget.form?.requestSubmit();
                  }
                }}
                placeholder={`Put text or a link on ${selectedPeer.displayName}'s clipboard`}
                aria-label="Text to send"
                disabled={!connected}
              />
              <button
                type="submit"
                className="btn btn-primary btn-sm"
                disabled={!connected || sendingClip || !clipText.trim()}
              >
                <Send size={14} />
                {sendingClip ? "Sending…" : "Send"}
              </button>
            </form>
          </section>
        </div>

        {(sentClips.length > 0 || transfers.length > 0) && (
          <section className="card" aria-labelledby="recent-title">
            <header className="card-header">
              <h2 id="recent-title">Recent</h2>
              {transfers.length > 0 && (
                <button type="button" className="link-btn" onClick={() => setPage("transfers")}>
                  All transfers
                </button>
              )}
            </header>
            <ul className="rows">
              {transfers.slice(0, 3).map(renderTransferRow)}
              {sentClips.map((clip) => (
                <li key={clip.id} className="row">
                  <span className="file-tile tone-text">
                    <FileText size={17} />
                  </span>
                  <span className="row-main">
                    <span className="row-title">{clip.text}</span>
                    <span className="row-sub">
                      Text · to {clip.peerName} · {formatRelativeTime(clip.timestamp)}
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
          </section>
        )}
      </div>
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
        <div className="empty">
          <span className="empty-icon">
            <ArrowUpDown size={20} />
          </span>
          <p className="empty-title">No transfers yet</p>
          <p className="muted">Drop files on this window to send them to a connected device.</p>
        </div>
      ) : (
        <section className="card">
          <ul className="rows">{transfers.map(renderTransferRow)}</ul>
        </section>
      )}
    </>
  );

  const renderDevices = () => (
    <>
      <header className="page-header">
        <div>
          <h1>Devices</h1>
          <p className="muted">Each paired device is verified by its own key.</p>
        </div>
        <button type="button" className="btn btn-primary btn-sm" onClick={() => setShowPairDialog(true)}>
          <Plus size={15} />
          Pair a device
        </button>
      </header>
      {peers?.length === 0 ? (
        <div className="empty">
          <span className="empty-icon">
            <MonitorSmartphone size={20} />
          </span>
          <p className="empty-title">No paired devices</p>
          <p className="muted">Pair your phone to start sending files and text.</p>
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

      <section className="card" aria-labelledby="appearance-title">
        <header className="card-header">
          <h2 id="appearance-title">Appearance</h2>
        </header>
        <div className="setting">
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
                {accent === option.id && <Check size={13} color={option.onBase} strokeWidth={3} />}
              </button>
            ))}
          </div>
        </div>
      </section>

      {identity && (
        <section className="card" aria-labelledby="this-computer-title">
          <header className="card-header">
            <h2 id="this-computer-title">This computer</h2>
          </header>
          <div className="setting">
            <div>
              <p className="row-title">Name</p>
              <p className="row-sub">Shown to devices you pair with.</p>
            </div>
            <span>{identity.deviceName}</span>
          </div>
          <div className="setting stacked">
            <div className="setting-head">
              <div>
                <p className="row-title">Device fingerprint</p>
                <p className="row-sub">Compare this with what your phone shows while pairing.</p>
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
          <div className="setting stacked">
            <p className="row-title">Certificate hash</p>
            <code className="key-value">{identity.spkiHash}</code>
          </div>
        </section>
      )}
    </>
  );

  const renderUnavailable = () => (
    <div className="empty empty-page" role="alert">
      <span className="empty-icon">
        <CircleAlert size={20} />
      </span>
      <p className="empty-title">Continue's local service isn't running</p>
      {isTauri() ? (
        <p className="muted">{loadError}</p>
      ) : (
        <p className="muted">
          This window is showing the interface on its own. Start the desktop app with <code>pnpm tauri dev</code>.
        </p>
      )}
    </div>
  );

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <img src="/icon.svg" alt="" className="brand-mark" />
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

        {peers && peers.length > 0 && (
          <div className="sidebar-section">
            <div className="sidebar-label">
              <span>Send to</span>
              <button
                type="button"
                className="icon-btn icon-btn-sm"
                onClick={() => setShowPairDialog(true)}
                aria-label="Pair a device"
                title="Pair a device"
              >
                <Plus size={14} />
              </button>
            </div>
            {peers.map((peer) => (
              <button
                key={peer.fingerprint}
                type="button"
                className={`peer-item ${peer.fingerprint === selectedPeer?.fingerprint ? "active" : ""}`}
                aria-pressed={peer.fingerprint === selectedPeer?.fingerprint}
                onClick={() => {
                  setSelectedPeerId(peer.fingerprint);
                  setPage("home");
                }}
              >
                <DeviceIcon name={peer.displayName} size={15} />
                <span className="peer-item-name">{peer.displayName}</span>
                <StatusDot connected={peer.isConnected} />
              </button>
            ))}
          </div>
        )}

        {identity && (
          <div className="this-device" title={identity.fingerprint}>
            <span className="this-device-icon">
              <Laptop size={15} />
            </span>
            <div>
              <p className="row-title">{identity.deviceName}</p>
              <p className="row-sub mono">{shortFingerprint(identity.fingerprint)}</p>
            </div>
          </div>
        )}
      </aside>

      <main className="content">
        <div className="content-inner">
          {!isTauri() || loadError ? (
            renderUnavailable()
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

      {dragActive && selectedPeer?.isConnected && (
        <div className="drop-overlay" aria-hidden="true">
          <div className="drop-overlay-card">
            <Upload size={22} />
            Drop to send to {selectedPeer.displayName}
          </div>
        </div>
      )}

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
    <li className="card device-card">
      <div className="device-card-main">
        <span className="device-avatar">
          <DeviceIcon name={peer.displayName} />
        </span>
        <div className="row-main">
          <span className="row-title">{peer.displayName}</span>
          <span className="row-sub">
            <StatusDot connected={peer.isConnected} />
            {peer.isConnected ? "Connected" : "Offline"} · Paired {formatPairedDate(peer.pairedAt)}
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
