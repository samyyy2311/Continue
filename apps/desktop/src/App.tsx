// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowUpRight,
  Check,
  CheckCheck,
  CircleAlert,
  Copy,
  File,
  FileArchive,
  FileAudio,
  FileCode,
  FileImage,
  FileText,
  FileVideo,
  History,
  Laptop,
  Link2,
  Loader,
  Plus,
  Search,
  Send,
  Settings,
  Smartphone,
  Trash2,
  Type,
  Upload,
} from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
import {
  fileNameFromPath,
  formatBytes,
  formatPairedDate,
  formatRelativeTime,
  getFileCategory,
} from "./format.ts";
import { PairDialog } from "./PairDialog.tsx";
import {
  ACCENT_PALETTE,
  type AccentName,
  type Activity,
  type DeviceIdentity,
  GRANT_OPTIONS,
  type Grant,
  type HistoryFilter,
  isMac,
  MOD_KEY,
  MOD_SHIFT_KEY,
  type PeerPermission,
  PERMISSIONS,
  type Theme,
  type Toast,
  type TrustedPeer,
  type View,
} from "./types.ts";

const ACCENT_KEY = "continue.accent";
const THEME_KEY = "continue.theme";
const ACTIVITY_STORAGE_KEY = "continue.activity_log";

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
    // Storage can be unavailable in restrictive environments.
  }
}

function loadSavedActivity(): Activity[] {
  try {
    const data = localStorage.getItem(ACTIVITY_STORAGE_KEY);
    if (!data) return [];
    const parsed = JSON.parse(data) as Activity[];
    return Array.isArray(parsed) ? parsed.slice(0, 100) : [];
  } catch {
    return [];
  }
}

function saveActivity(items: Activity[]) {
  try {
    const completedOnly = items.filter((item) => item.status !== "sending").slice(0, 100);
    localStorage.setItem(ACTIVITY_STORAGE_KEY, JSON.stringify(completedOnly));
  } catch {
    // Storage failure silently ignored.
  }
}

function getFileIcon(name: string) {
  const category = getFileCategory(name);
  switch (category) {
    case "image":
      return <FileImage size={18} className="file-kind-icon icon-image" />;
    case "video":
      return <FileVideo size={18} className="file-kind-icon icon-media" />;
    case "audio":
      return <FileAudio size={18} className="file-kind-icon icon-media" />;
    case "archive":
      return <FileArchive size={18} className="file-kind-icon icon-archive" />;
    case "code":
      return <FileCode size={18} className="file-kind-icon icon-code" />;
    case "document":
      return <FileText size={18} className="file-kind-icon icon-doc" />;
    default:
      return <File size={18} className="file-kind-icon icon-other" />;
  }
}

export default function App() {
  const [view, setView] = useState<View>("transfer");
  const [accent, setAccent] = useState<AccentName>(() =>
    readStored(ACCENT_KEY, ACCENT_PALETTE.map((a) => a.id), "blue"),
  );
  const [theme, setTheme] = useState<Theme>(() => readStored(THEME_KEY, ["system", "light", "dark"], "system"));
  const [identity, setIdentity] = useState<DeviceIdentity | null>(null);
  const [peers, setPeers] = useState<TrustedPeer[] | null>(null);
  const [loadError, setLoadError] = useState("");
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);
  const [showPairDialog, setShowPairDialog] = useState(false);
  const [activity, setActivity] = useState<Activity[]>(() => loadSavedActivity());
  const [connecting, setConnecting] = useState<string | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);
  const [dragCount, setDragCount] = useState<number | null>(null);
  const [textInput, setTextInput] = useState("");

  const selectedPeer = peers?.find((p) => p.fingerprint === selectedPeerId) ?? peers?.[0] ?? null;

  const showToast = useCallback((message: string, tone: Toast["tone"] = "info") => {
    setToast({ message, tone });
  }, []);

  const showError = useCallback((message: string) => showToast(message, "error"), [showToast]);

  useEffect(() => {
    saveActivity(activity);
  }, [activity]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), toast.tone === "error" ? 5000 : 2600);
    return () => window.clearTimeout(timer);
  }, [toast]);

  useEffect(() => {
    const active = ACCENT_PALETTE.find((a) => a.id === accent) ?? ACCENT_PALETTE[0];
    document.documentElement.style.setProperty("--accent", active.base);
    document.documentElement.style.setProperty("--on-accent", active.onBase);
    writeStored(ACCENT_KEY, accent);
  }, [accent]);

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
    if (!isTauri()) {
      setIdentity({
        deviceName: "Continue-PC",
        fingerprint: "e49a:21fc:87aa:3201:99dc:b174:4410:f029",
        spkiHash: "3f821ac90278ef40182379d4ba728901cb48217f9012e8471209384918234710",
      });
      setPeers((prev) =>
        prev && prev.length > 0
          ? prev
          : [
              {
                displayName: "Pixel 8 Pro",
                fingerprint: "f210:48bc:9901:14a2",
                pairedAt: Math.floor(Date.now() / 1000) - 86400 * 3,
                isConnected: true,
                endpoint: "192.168.1.45:4433",
              },
              {
                displayName: "Galaxy Tab S9",
                fingerprint: "389a:11cc:55bb:8812",
                pairedAt: Math.floor(Date.now() / 1000) - 86400 * 14,
                isConnected: false,
                endpoint: "192.168.1.80:4433",
              },
            ],
      );
      return;
    }
    Promise.all([getDeviceIdentity(), getTrustedPeers()])
      .then(([loadedIdentity, loadedPeers]) => {
        setIdentity(loadedIdentity);
        setPeers(loadedPeers);
      })
      .catch((error) => setLoadError(errorMessage(error)));
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    const cleanups: (() => void)[] = [];

    const setupListeners = async () => {
      try {
        const unPeer = await listen<{ fingerprint: string; displayName: string }>(
          "peer-connected",
          (event) => {
            void refreshPeers();
            showToast(`${event.payload.displayName} connected`);
          },
        );
        cleanups.push(unPeer);

        const unState = await listen("peer-state-changed", () => void refreshPeers());
        cleanups.push(unState);

        const unFile = await listen<{ fileName: string; path: string; bytesReceived: number }>(
          "file-received",
          (event) => {
            showToast(`Received ${event.payload.fileName}`);
            setActivity((prev) => [
              {
                id: `rx-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
                kind: "file",
                label: event.payload.fileName,
                peerId: "remote",
                peerName: "Phone",
                status: "sent",
                timestamp: Date.now(),
                path: event.payload.path,
                bytesSent: event.payload.bytesReceived,
                totalBytes: event.payload.bytesReceived,
              },
              ...prev,
            ]);
          },
        );
        cleanups.push(unFile);

        const unClip = await listen<{ format: string; content: string }>(
          "clipboard-received",
          (event) => {
            showToast("Clipboard synced from device");
            void navigator.clipboard?.writeText?.(event.payload.content).catch(() => {});
            setActivity((prev) => [
              {
                id: `rx-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
                kind: "text",
                label: event.payload.content.slice(0, 50),
                peerId: "remote",
                peerName: "Phone",
                status: "sent",
                timestamp: Date.now(),
              },
              ...prev,
            ]);
          },
        );
        cleanups.push(unClip);
      } catch {
        // Tauri events unsupported in current environment.
      }
    };

    void setupListeners();
    return () => {
      for (const cleanup of cleanups) cleanup();
    };
  }, [refreshPeers, showToast]);

  const handleConnect = async (peer: TrustedPeer, rawAddress: string) => {
    const address = rawAddress.trim();
    if (!address) {
      showError("Enter the device's address, like 192.168.1.20:47470.");
      return;
    }
    setConnecting(peer.fingerprint);
    if (!isTauri()) {
      setTimeout(() => {
        setPeers((prev) =>
          prev?.map((p) =>
            p.fingerprint === peer.fingerprint ? { ...p, isConnected: true, endpoint: address } : p,
          ) ?? null,
        );
        setConnecting(null);
        showToast(`Connected to ${peer.displayName}`);
      }, 500);
      return;
    }
    try {
      await connectToPeer(peer.fingerprint, address);
      await refreshPeers();
      showToast(`Connected to ${peer.displayName}`);
    } catch (error) {
      showError(errorMessage(error));
    } finally {
      setConnecting(null);
    }
  };

  const handleDisconnect = async (peer: TrustedPeer) => {
    if (!isTauri()) {
      setPeers((prev) =>
        prev?.map((p) =>
          p.fingerprint === peer.fingerprint ? { ...p, isConnected: false } : p,
        ) ?? null,
      );
      showToast(`Disconnected ${peer.displayName}`);
      return;
    }
    try {
      await disconnectPeer(peer.fingerprint);
      await refreshPeers();
      showToast(`Disconnected ${peer.displayName}`);
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const handleRemovePeer = async (peer: TrustedPeer) => {
    if (!isTauri()) {
      setPeers((prev) => prev?.filter((p) => p.fingerprint !== peer.fingerprint) ?? null);
      showToast(`Unpaired ${peer.displayName}`);
      if (selectedPeerId === peer.fingerprint) {
        setSelectedPeerId(null);
      }
      return;
    }
    try {
      await removeTrustedPeer(peer.fingerprint);
      showToast(`Unpaired ${peer.displayName}`);
      if (selectedPeerId === peer.fingerprint) {
        setSelectedPeerId(null);
      }
      refreshPeers();
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const trackTransfer = async (
    item: Omit<Activity, "id" | "status" | "timestamp">,
    sendFn: (update: (patch: Partial<Activity>) => void) => Promise<number | void>,
  ) => {
    const id = crypto.randomUUID();
    setActivity((prev) => [{ ...item, id, status: "sending", timestamp: Date.now() }, ...prev]);
    const update = (patch: Partial<Activity>) =>
      setActivity((prev) => prev.map((a) => (a.id === id ? { ...a, ...patch } : a)));
    try {
      const bytesSent = await sendFn(update);
      update({ status: "sent", bytesSent: bytesSent ?? undefined });
      return true;
    } catch (error) {
      update({ status: "failed", error: errorMessage(error) });
      return false;
    }
  };

  const sendFile = (peer: TrustedPeer, path: string) => {
    if (!isTauri()) {
      return trackTransfer(
        { kind: "file", label: fileNameFromPath(path), path, peerId: peer.fingerprint, peerName: peer.displayName },
        async (update) => {
          const total = 4800000;
          update({ bytesSent: 0, totalBytes: total });
          await new Promise((r) => setTimeout(r, 200));
          update({ bytesSent: Math.round(total * 0.45), totalBytes: total });
          await new Promise((r) => setTimeout(r, 200));
          update({ bytesSent: total, totalBytes: total });
          return total;
        },
      );
    }
    return trackTransfer(
      { kind: "file", label: fileNameFromPath(path), path, peerId: peer.fingerprint, peerName: peer.displayName },
      (update) => sendFileToPeer(peer.fingerprint, path, (progress) => update(progress)),
    );
  };

  const sendText = (peer: TrustedPeer, text: string) => {
    if (!isTauri()) {
      return trackTransfer({ kind: "text", label: text, peerId: peer.fingerprint, peerName: peer.displayName }, async () => {
        await new Promise((r) => setTimeout(r, 150));
        return 0;
      });
    }
    return trackTransfer({ kind: "text", label: text, peerId: peer.fingerprint, peerName: peer.displayName }, () =>
      sendClipboardText(peer.fingerprint, text),
    );
  };

  const sendFiles = async (paths: string[]) => {
    if (!selectedPeer) {
      showError("Pair a device first.");
      return;
    }
    if (!selectedPeer.isConnected) {
      showError(`Connect to ${selectedPeer.displayName} first.`);
      return;
    }
    for (const path of paths) {
      await sendFile(selectedPeer, path);
    }
  };

  const retryItem = (item: Activity) => {
    const peer = peers?.find((p) => p.fingerprint === item.peerId);
    if (!peer) {
      showError("Device is no longer paired.");
      return;
    }
    if (!peer.isConnected) {
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

  const sendFilesRef = useRef(sendFiles);
  sendFilesRef.current = sendFiles;

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter") {
        setDragCount(payload.paths.length);
        return;
      }
      if (payload.type === "over") return;
      setDragCount(null);
      if (payload.type === "drop" && payload.paths.length > 0) {
        setView("transfer");
        sendFilesRef.current(payload.paths);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const chooseFiles = async () => {
    if (!selectedPeer) {
      showError("Pair a device before selecting files.");
      return;
    }
    if (!selectedPeer.isConnected) {
      showError(`Connect to ${selectedPeer.displayName} first.`);
      return;
    }
    if (!isTauri()) {
      sendFiles(["annual_financials.pdf", "presentation_deck.key"]);
      return;
    }
    const picked = await openFileDialog({ multiple: true, directory: false });
    if (picked) {
      const paths = Array.isArray(picked) ? picked : [picked];
      sendFiles(paths);
    }
  };

  const handleSendText = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const text = textInput.trim();
    if (!selectedPeer) {
      showError("Pair a device first.");
      return;
    }
    if (!selectedPeer.isConnected) {
      showError(`Connect to ${selectedPeer.displayName} first.`);
      return;
    }
    if (!text) return;
    setTextInput("");
    const sent = await sendText(selectedPeer, text);
    if (!sent) setTextInput(text);
  };

  const handleSendClipboard = async () => {
    if (!selectedPeer) {
      showError("Pair a device first.");
      return;
    }
    if (!selectedPeer.isConnected) {
      showError(`Connect to ${selectedPeer.displayName} first.`);
      return;
    }
    try {
      const text = await navigator.clipboard.readText();
      if (!text || !text.trim()) {
        showError("System clipboard is empty.");
        return;
      }
      await sendText(selectedPeer, text.trim());
      showToast("Clipboard sent to device");
    } catch {
      showError("Unable to access clipboard. Please paste into text field.");
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showToast("Copied to clipboard");
    } catch {
      showError("Failed to copy to clipboard");
    }
  };

  const chooseFilesRef = useRef(chooseFiles);
  chooseFilesRef.current = chooseFiles;

  const handleSendClipboardRef = useRef(handleSendClipboard);
  handleSendClipboardRef.current = handleSendClipboard;

  useEffect(() => {
    const handleGlobalKeydown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setShowPairDialog(false);
        return;
      }

      const mod = isMac ? e.metaKey : e.ctrlKey;
      const target = e.target as HTMLElement | null;
      const isInputFocused =
        target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable);

      if (mod && e.key === "1") {
        e.preventDefault();
        setView("transfer");
      } else if (mod && e.key === "2") {
        e.preventDefault();
        setView("devices");
      } else if (mod && e.key === "3") {
        e.preventDefault();
        setView("history");
      } else if (mod && e.key === "4") {
        e.preventDefault();
        setView("settings");
      } else if (mod && !e.shiftKey && e.key.toLowerCase() === "o") {
        e.preventDefault();
        void chooseFilesRef.current();
      } else if (mod && e.shiftKey && e.key.toLowerCase() === "v") {
        if (!isInputFocused) {
          e.preventDefault();
          void handleSendClipboardRef.current();
        }
      }
    };

    window.addEventListener("keydown", handleGlobalKeydown);
    return () => window.removeEventListener("keydown", handleGlobalKeydown);
  }, []);

  const handlePaired = useCallback(
    (peer: TrustedPeer) => {
      setShowPairDialog(false);
      setSelectedPeerId(peer.fingerprint);
      setPeers((prev) => (prev ? [...prev.filter((p) => p.fingerprint !== peer.fingerprint), peer] : [peer]));
      setView("transfer");
      showToast(`Paired with ${peer.displayName}`);
      if (isTauri()) refreshPeers();
    },
    [refreshPeers, showToast],
  );

  const ready = !loadError && peers !== null;

  if (!ready) {
    return (
      <main className="app-standalone">
        <div className="standalone-card">
          <div className="brand-logo large">
            <img src="/icon.svg" alt="Continue Logo" />
          </div>
          <h1 className="standalone-title">
            {loadError ? "Connection Error" : "Continue Desktop"}
          </h1>
          <p className="standalone-desc">
            {loadError || "This interface runs inside the Continue desktop runtime. Launch with pnpm tauri dev or open the application package."}
          </p>
          {isTauri() && (
            <button type="button" className="btn btn-primary" onClick={() => window.location.reload()}>
              Retry Connection
            </button>
          )}
        </div>
      </main>
    );
  }

  const activeTransfers = activity.filter((a) => a.status === "sending");
  const recentActivity = activity.filter((a) => a.status !== "sending").slice(0, 5);

  return (
    <div className="layout-root">
      {/* Top Navigation Bar */}
      <header className="top-nav-bar" aria-label="Main Navigation">
        <div className="top-nav-left">
          <div className="brand" role="button" tabIndex={0} onClick={() => setView("transfer")}>
            <img src="/icon.svg" alt="" className="brand-icon" />
            <span className="brand-name">Continue</span>
          </div>

          {selectedPeer && (
            <div className="nav-target-badge" title={`Active Device: ${selectedPeer.displayName}`}>
              <Smartphone size={13} className="text-secondary" />
              <span className="nav-target-name">{selectedPeer.displayName}</span>
              <span className={`status-dot-sm ${selectedPeer.isConnected ? "online" : "offline"}`} />
              <span className="nav-target-status">
                {selectedPeer.isConnected ? "Connected" : "Offline"}
              </span>
            </div>
          )}
        </div>

        <nav className="top-nav-tabs" aria-label="Views">
          <button
            type="button"
            className={`top-nav-tab ${view === "transfer" ? "active" : ""}`}
            onClick={() => setView("transfer")}
          >
            <Send size={14} />
            <span>Transfer</span>
            {activeTransfers.length > 0 && (
              <span className="badge-count">{activeTransfers.length}</span>
            )}
          </button>
          <button
            type="button"
            className={`top-nav-tab ${view === "devices" ? "active" : ""}`}
            onClick={() => setView("devices")}
          >
            <Smartphone size={14} />
            <span>Devices</span>
            {peers.length > 0 && <span className="nav-meta-inline">{peers.length}</span>}
          </button>
          <button
            type="button"
            className={`top-nav-tab ${view === "history" ? "active" : ""}`}
            onClick={() => setView("history")}
          >
            <History size={14} />
            <span>Activity</span>
          </button>
          <button
            type="button"
            className={`top-nav-tab ${view === "settings" ? "active" : ""}`}
            onClick={() => setView("settings")}
          >
            <Settings size={14} />
            <span>Settings</span>
          </button>
        </nav>

        <div className="top-nav-right">
          {view === "transfer" && selectedPeer?.isConnected && (
            <button
              type="button"
              className="btn btn-ghost btn-xs"
              onClick={() => handleDisconnect(selectedPeer)}
            >
              Disconnect
            </button>
          )}

          {identity && (
            <span className="host-machine-chip" title={`This Computer: ${identity.fingerprint}`}>
              <Laptop size={12} />
              <span>{identity.deviceName}</span>
            </span>
          )}

          <button
            type="button"
            className="btn btn-primary btn-sm"
            onClick={() => setShowPairDialog(true)}
          >
            <Plus size={14} className="inline-icon" />
            Pair Device
          </button>
        </div>
      </header>

      {/* Main Content Area */}
      <main className="app-workspace">

        {/* Workspace Body */}
        <div className="workspace-scroll-container">
          {view === "transfer" && (
            <TransferView
              peer={selectedPeer}
              peers={peers}
              onSelectPeer={setSelectedPeerId}
              onOpenPair={() => setShowPairDialog(true)}
              onChooseFiles={chooseFiles}
              onSendClipboard={handleSendClipboard}
              textInput={textInput}
              onTextInputChange={setTextInput}
              onSendText={handleSendText}
              onConnect={(address) => selectedPeer && handleConnect(selectedPeer, address)}
              isConnecting={connecting === selectedPeer?.fingerprint}
              activeTransfers={activeTransfers}
              recentActivity={recentActivity}
              onRetry={retryItem}
              onCopy={copyToClipboard}
              onNavigateHistory={() => setView("history")}
            />
          )}

          {view === "devices" && (
            <DevicesView
              peers={peers}
              selectedPeerId={selectedPeerId}
              onSelectPeer={(id) => {
                setSelectedPeerId(id);
                setView("transfer");
              }}
              onOpenPair={() => setShowPairDialog(true)}
              onConnect={handleConnect}
              onDisconnect={handleDisconnect}
              onUnpair={handleRemovePeer}
              connectingId={connecting}
              onError={showError}
              onSuccess={showToast}
            />
          )}

          {view === "history" && (
            <HistoryView
              activity={activity}
              onRetry={retryItem}
              onCopy={copyToClipboard}
              onClear={() => setActivity([])}
            />
          )}

          {view === "settings" && (
            <SettingsView
              identity={identity}
              theme={theme}
              accent={accent}
              onThemeChange={setTheme}
              onAccentChange={setAccent}
              onCopy={copyToClipboard}
            />
          )}
        </div>
      </main>

      {/* Global Drag & Drop Overlay */}
      {dragCount !== null && (
        <div className="drag-drop-curtain" aria-hidden="true">
          <div className="drag-drop-card">
            <Upload size={36} className="text-accent" />
            <p className="drag-title">
              {selectedPeer?.isConnected
                ? `Send to ${selectedPeer.displayName}`
                : "Connect device before sending files"}
            </p>
            <p className="drag-desc">
              {dragCount > 0 ? `${dragCount} file${dragCount > 1 ? "s" : ""} detected` : "Drop anywhere to send"}
            </p>
          </div>
        </div>
      )}

      {/* Pairing Dialog Modal */}
      {showPairDialog && (
        <PairDialog onPaired={handlePaired} onClose={() => setShowPairDialog(false)} />
      )}

      {/* Global Toast Alert */}
      {toast && (
        <div className={`app-toast toast-${toast.tone}`} role={toast.tone === "error" ? "alert" : "status"}>
          {toast.tone === "error" ? <CircleAlert size={16} /> : <Check size={16} />}
          <span>{toast.message}</span>
        </div>
      )}
    </div>
  );
}

// -----------------------------------------------------------------------------
// View 1: Transfer Studio
// -----------------------------------------------------------------------------

interface TransferViewProps {
  peer: TrustedPeer | null;
  peers: TrustedPeer[];
  onSelectPeer: (id: string) => void;
  onOpenPair: () => void;
  onChooseFiles: () => void;
  onSendClipboard: () => void;
  textInput: string;
  onTextInputChange: (val: string) => void;
  onSendText: () => void;
  onConnect: (address: string) => void;
  isConnecting: boolean;
  activeTransfers: Activity[];
  recentActivity: Activity[];
  onRetry: (item: Activity) => void;
  onCopy: (text: string) => void;
  onNavigateHistory: () => void;
}

function TransferView(props: TransferViewProps) {
  const {
    peer,
    peers,
    onSelectPeer,
    onOpenPair,
    onChooseFiles,
    onSendClipboard,
    textInput,
    onTextInputChange,
    onSendText,
    onConnect,
    isConnecting,
    activeTransfers,
    recentActivity,
    onRetry,
    onCopy,
    onNavigateHistory,
  } = props;

  if (!peer) {
    return (
      <div className="empty-state">
        <Smartphone size={36} className="text-muted" />
        <h2 className="page-title">No Devices Paired</h2>
        <p className="empty-text">
          Link your phone or another computer over your local network to send files, text, and clipboard seamlessly.
        </p>
        <button type="button" className="btn btn-primary" onClick={onOpenPair}>
          <Plus size={16} className="inline-icon" />
          Pair New Device
        </button>
      </div>
    );
  }

  const isOnline = peer.isConnected;
  const isUrl = /^https?:\/\//i.test(textInput.trim());

  return (
    <div className="transfer-stage-view">
      {/* 1. Spatial Peer Beacon (Direct AirBeam Node) */}
      <div
        className={`beacon-stage ${!isOnline ? "offline" : ""}`}
        onClick={isOnline ? onChooseFiles : undefined}
        role="button"
        tabIndex={isOnline ? 0 : -1}
        title={isOnline ? `Click or drop files anywhere to beam to ${peer.displayName}` : undefined}
        onKeyDown={(e) => e.key === "Enter" && isOnline && onChooseFiles()}
      >
        <div className="beacon-radar-wrap">
          {isOnline && <span className="beacon-pulse-ring" />}
          <div className="beacon-device-avatar">
            <Smartphone size={32} strokeWidth={1.75} />
          </div>
          <span className={`beacon-status-dot ${isOnline ? "online" : "offline"}`} />
        </div>

        <div className="beacon-meta">
          <div className="beacon-title-row">
            <h2 className="beacon-peer-name">{peer.displayName}</h2>
            <span className={`beacon-status-pill ${isOnline ? "online" : "offline"}`}>
              {isOnline ? "Connected" : "Offline"}
            </span>
          </div>

          <p className="beacon-subtitle">
            {isOnline
              ? `Local network${peer.endpoint ? ` · ${peer.endpoint}` : ""} · Pinned TLS 1.3 · Zero cloud`
              : "Connects automatically when both devices are on the same network"}
          </p>
        </div>

        {/* Peer Switcher Chips if multiple peers exist */}
        {peers.length > 1 && (
          <div className="beacon-peers-strip" onClick={(e) => e.stopPropagation()}>
            {peers.map((p) => {
              const isSelected = p.fingerprint === peer.fingerprint;
              return (
                <button
                  key={p.fingerprint}
                  type="button"
                  className={`beacon-peer-chip ${isSelected ? "active" : ""}`}
                  onClick={() => onSelectPeer(p.fingerprint)}
                >
                  <span className={`status-dot-sm ${p.isConnected ? "online" : "offline"}`} />
                  <span>{p.displayName}</span>
                </button>
              );
            })}
          </div>
        )}

        {!isOnline && (
          <div className="beacon-offline-connect" onClick={(e) => e.stopPropagation()}>
            <ManualConnect
              key={peer.fingerprint}
              initialAddress={peer.endpoint}
              isConnecting={isConnecting}
              onConnect={onConnect}
            />
          </div>
        )}
      </div>

      {/* 2. Unified Command Omnibar */}
      <div className="omnibar-section">
        <div className="transfer-omnibar">
          <button
            type="button"
            className="omnibar-action-btn"
            onClick={onChooseFiles}
            disabled={!isOnline}
            title={`Choose files to beam (${MOD_KEY}O)`}
          >
            <Upload size={14} />
            <span>Files</span>
            <kbd className="omnibar-kbd">{MOD_KEY}O</kbd>
          </button>

          <button
            type="button"
            className="omnibar-action-btn"
            onClick={onSendClipboard}
            disabled={!isOnline}
            title={`Beam clipboard text (${MOD_SHIFT_KEY}V)`}
          >
            <Copy size={13} />
            <span>Clipboard</span>
            <kbd className="omnibar-kbd">{MOD_SHIFT_KEY}V</kbd>
          </button>

          <div className="omnibar-sep" />

          <div className="omnibar-input-container">
            {isUrl && (
              <span className="omnibar-url-pill" title="Web URL detected">
                <Link2 size={11} /> Link
              </span>
            )}
            <input
              type="text"
              className="omnibar-text-input font-mono"
              placeholder={
                isOnline
                  ? `Send note, link, or drop files anywhere...`
                  : "Connect device to send text or files..."
              }
              value={textInput}
              onChange={(e) => onTextInputChange(e.target.value)}
              disabled={!isOnline}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  onSendText();
                }
              }}
            />
            {textInput.trim() && (
              <button
                type="button"
                className="btn btn-primary btn-xs omnibar-submit-btn"
                disabled={!isOnline}
                onClick={onSendText}
                title="Send to device (Enter)"
              >
                <Send size={11} />
              </button>
            )}
          </div>
        </div>

        <div className="omnibar-footer-hints">
          <span>Drop files anywhere on screen</span>
          <span className="hint-sep">·</span>
          <span>Zero cloud</span>
          <span className="hint-sep">·</span>
          <span>End-to-end encrypted</span>
        </div>
      </div>

      {/* 3. Active Stream Progress */}
      {activeTransfers.length > 0 && (
        <div className="active-transfers-stream">
          {activeTransfers.map((item) => {
            const progressPct =
              item.totalBytes && item.totalBytes > 0
                ? Math.min(100, Math.round(((item.bytesSent ?? 0) / item.totalBytes) * 100))
                : null;
            return (
              <div key={item.id} className="active-transfer-card">
                <div className="active-transfer-icon">{getFileIcon(item.label)}</div>
                <div className="active-transfer-detail">
                  <div className="active-transfer-top">
                    <span className="active-transfer-name" title={item.label}>
                      {item.label}
                    </span>
                    <span className="active-transfer-pct">
                      {progressPct !== null ? `${progressPct}%` : "Streaming..."}
                    </span>
                  </div>
                  <div className="active-transfer-track">
                    <div
                      className="active-transfer-fill"
                      style={{ width: `${progressPct ?? 35}%` }}
                    />
                  </div>
                  <div className="active-transfer-bottom">
                    <span>
                      {item.bytesSent !== undefined ? formatBytes(item.bytesSent) : "0 B"}
                      {item.totalBytes ? ` of ${formatBytes(item.totalBytes)}` : ""}
                    </span>
                    <span>→ {item.peerName}</span>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* 4. Recent Activity Stream */}
      {recentActivity.length > 0 && (
        <div className="recent-stream-section">
          <div className="recent-stream-head">
            <span className="recent-stream-title">Recent Transmissions</span>
            <button type="button" className="link-action" onClick={onNavigateHistory}>
              Activity Log <ArrowUpRight size={12} />
            </button>
          </div>

          <div className="recent-stream-list">
            {recentActivity.map((item) => (
              <div key={item.id} className="recent-stream-row">
                <div className="recent-item-icon">
                  {item.kind === "file" ? getFileIcon(item.label) : <Type size={15} />}
                </div>
                <div className="recent-item-info">
                  <span className="recent-item-title" title={item.label}>
                    {item.label}
                  </span>
                  <span className="recent-item-meta">
                    {item.peerName} · {formatRelativeTime(item.timestamp)}
                    {item.bytesSent !== undefined && ` · ${formatBytes(item.bytesSent)}`}
                  </span>
                </div>
                <div className="recent-item-status">
                  {item.status === "sent" ? (
                    <span className="badge-ok" title="Delivered">
                      <CheckCheck size={13} />
                    </span>
                  ) : (
                    <span className="badge-err" title={item.error || "Failed"}>
                      <CircleAlert size={13} />
                    </span>
                  )}
                </div>
                <div className="recent-item-actions">
                  {item.kind === "text" && (
                    <button
                      type="button"
                      className="btn-icon"
                      title="Copy text"
                      onClick={() => onCopy(item.label)}
                    >
                      <Copy size={13} />
                    </button>
                  )}
                  {item.status === "failed" && (
                    <button
                      type="button"
                      className="btn btn-secondary btn-xs"
                      onClick={() => onRetry(item)}
                    >
                      Retry
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// -----------------------------------------------------------------------------
// View 2: Devices Management (Clean, Flat, Linear style)
// -----------------------------------------------------------------------------

interface DevicesViewProps {
  peers: TrustedPeer[];
  selectedPeerId: string | null;
  onSelectPeer: (id: string) => void;
  onOpenPair: () => void;
  onConnect: (peer: TrustedPeer, address: string) => Promise<void>;
  onDisconnect: (peer: TrustedPeer) => Promise<void>;
  onUnpair: (peer: TrustedPeer) => Promise<void>;
  connectingId: string | null;
  onError: (msg: string) => void;
  onSuccess: (msg: string) => void;
}

function DevicesView(props: DevicesViewProps) {
  const {
    peers,
    selectedPeerId,
    onSelectPeer,
    onOpenPair,
    onConnect,
    onDisconnect,
    onUnpair,
    connectingId,
    onError,
  } = props;

  return (
    <div className="devices-page">
      <div className="page-header">
        <div>
          <h2 className="page-title">Devices</h2>
          <p className="page-subtitle">Manage paired phones, connection addresses, and capability permissions</p>
        </div>
        <button type="button" className="btn btn-primary btn-sm" onClick={onOpenPair}>
          <Plus size={14} className="inline-icon" />
          Pair New Device
        </button>
      </div>

      {peers.length === 0 ? (
        <div className="empty-state">
          <Smartphone size={36} className="text-muted" />
          <h3 className="page-title">No Devices Paired</h3>
          <p className="empty-text">
            Pair your phone or second computer to enable mutual TLS continuity and peer-to-peer file transfer.
          </p>
          <button type="button" className="btn btn-primary btn-sm mt-3" onClick={onOpenPair}>
            Pair Device
          </button>
        </div>
      ) : (
        <div className="device-list">
          {peers.map((peer) => (
            <DeviceEntry
              key={peer.fingerprint}
              peer={peer}
              isSelected={peer.fingerprint === selectedPeerId}
              isConnecting={connectingId === peer.fingerprint}
              onSelect={() => onSelectPeer(peer.fingerprint)}
              onConnect={(addr) => onConnect(peer, addr)}
              onDisconnect={() => onDisconnect(peer)}
              onUnpair={() => onUnpair(peer)}
              onError={onError}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function DeviceEntry(props: {
  peer: TrustedPeer;
  isSelected: boolean;
  isConnecting: boolean;
  onSelect: () => void;
  onConnect: (address: string) => void;
  onDisconnect: () => void;
  onUnpair: () => void;
  onError: (msg: string) => void;
}) {
  const { peer, isConnecting, onSelect, onConnect, onDisconnect, onUnpair, onError } = props;
  const [permissions, setPermissions] = useState<PeerPermission[] | null>(null);
  const [unpairingConfirm, setUnpairingConfirm] = useState(false);

  useEffect(() => {
    if (!isTauri()) {
      setPermissions([
        { capabilityId: 1, capabilityName: "File Transfer", grant: "Allow" },
        { capabilityId: 2, capabilityName: "Clipboard Sync", grant: "Allow" },
        { capabilityId: 3, capabilityName: "Notification Relay", grant: "Ask" },
      ]);
      return;
    }
    getPermissions(peer.fingerprint)
      .then(setPermissions)
      .catch((err) => onError(errorMessage(err)));
  }, [peer.fingerprint, onError]);

  const updateGrant = async (permission: PeerPermission, grant: Grant) => {
    if (!isTauri()) {
      setPermissions((prev) =>
        prev ? prev.map((p) => (p.capabilityId === permission.capabilityId ? { ...p, grant } : p)) : null,
      );
      return;
    }
    try {
      await setPermission(peer.fingerprint, permission.capabilityId, grant);
      setPermissions((prev) =>
        prev ? prev.map((p) => (p.capabilityId === permission.capabilityId ? { ...p, grant } : p)) : null,
      );
    } catch (err) {
      onError(errorMessage(err));
    }
  };

  return (
    <div className="device-entry">
      <div className="device-entry-head">
        <div className="device-entry-left">
          <div className="device-type-icon">
            <Smartphone size={20} />
          </div>
          <div className="device-entry-meta">
            <div className="device-entry-title">
              <span>{peer.displayName}</span>
              <span className={`status-pill ${peer.isConnected ? "online" : "offline"}`}>
                <span className="status-dot-sm" />
                {peer.isConnected ? "Connected" : "Offline"}
              </span>
            </div>
            <span className="device-entry-sub">
              Paired {formatPairedDate(peer.pairedAt)} · Fingerprint: {peer.fingerprint.slice(0, 14)}...
            </span>
          </div>
        </div>

        <div className="device-entry-actions">
          {peer.isConnected ? (
            <button type="button" className="btn btn-ghost btn-sm" onClick={onDisconnect}>
              Disconnect
            </button>
          ) : (
            <ManualConnect initialAddress={peer.endpoint} isConnecting={isConnecting} onConnect={onConnect} />
          )}

          <button type="button" className="btn btn-secondary btn-sm" onClick={onSelect}>
            <Send size={13} className="inline-icon" />
            Send Files
          </button>

          <button
            type="button"
            className={`btn btn-sm ${unpairingConfirm ? "btn-danger" : "btn-ghost"}`}
            onClick={() => {
              if (unpairingConfirm) {
                onUnpair();
              } else {
                setUnpairingConfirm(true);
                setTimeout(() => setUnpairingConfirm(false), 3500);
              }
            }}
          >
            <Trash2 size={13} className="inline-icon" />
            {unpairingConfirm ? "Confirm Unpair" : "Unpair"}
          </button>
        </div>
      </div>

      {/* Flat Capabilities list */}
      <div className="capabilities-list">
        {permissions ? (
          permissions.map((perm) => {
            const meta = PERMISSIONS[perm.capabilityName];
            return (
              <div key={perm.capabilityId} className="capability-row">
                <div className="capability-info">
                  <span className="capability-title">{meta?.label ?? perm.capabilityName}</span>
                  <span className="capability-desc">{meta?.description}</span>
                </div>
                <div className="grant-segmented" role="radiogroup">
                  {GRANT_OPTIONS.map((opt) => (
                    <button
                      key={opt.value}
                      type="button"
                      className={`grant-btn ${perm.grant === opt.value ? "selected" : ""}`}
                      onClick={() => updateGrant(perm, opt.value)}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
            );
          })
        ) : (
          <div style={{ padding: "10px 14px", color: "var(--text-muted)", fontSize: "12px" }}>
            <Loader size={13} className="spin inline-icon" /> Loading permissions...
          </div>
        )}
      </div>
    </div>
  );
}

// -----------------------------------------------------------------------------
// View 3: Activity History (Clean, Flat, Non-Boxed Table)
// -----------------------------------------------------------------------------

interface HistoryViewProps {
  activity: Activity[];
  onRetry: (item: Activity) => void;
  onCopy: (text: string) => void;
  onClear: () => void;
}

function HistoryView(props: HistoryViewProps) {
  const { activity, onRetry, onCopy, onClear } = props;
  const [filter, setFilter] = useState<HistoryFilter>("all");
  const [search, setSearch] = useState("");

  const needle = search.trim().toLowerCase();
  const filtered = activity.filter((item) => {
    if (filter === "file" && item.kind !== "file") return false;
    if (filter === "text" && item.kind !== "text") return false;
    if (filter === "failed" && item.status !== "failed") return false;
    if (needle && !item.label.toLowerCase().includes(needle) && !item.peerName.toLowerCase().includes(needle)) {
      return false;
    }
    return true;
  });

  return (
    <div className="activity-page">
      <div className="page-header">
        <div>
          <h2 className="page-title">Activity</h2>
          <p className="page-subtitle">Timeline of files and clipboard items transmitted</p>
        </div>
        {activity.length > 0 && (
          <button type="button" className="btn btn-secondary btn-sm" onClick={onClear}>
            <Trash2 size={13} className="inline-icon" />
            Clear Activity
          </button>
        )}
      </div>

      <div className="filter-bar">
        <div className="filter-pills">
          {(["all", "file", "text", "failed"] as const).map((tab) => (
            <button
              key={tab}
              type="button"
              className={`filter-pill ${filter === tab ? "active" : ""}`}
              onClick={() => setFilter(tab)}
            >
              {tab === "all" && "All"}
              {tab === "file" && "Files"}
              {tab === "text" && "Text & Clipboard"}
              {tab === "failed" && "Failed"}
            </button>
          ))}
        </div>

        <div className="search-field">
          <Search size={14} className="text-muted" />
          <input
            type="search"
            className="search-input"
            placeholder="Search transfers..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
      </div>

      {filtered.length === 0 ? (
        <div className="empty-state">
          <History size={32} className="text-muted" />
          <p className="empty-text">
            {search ? `No items match "${search}".` : "No transfers in this category yet."}
          </p>
        </div>
      ) : (
        <div className="activity-table">
          {filtered.map((item) => (
            <div key={item.id} className="activity-row">
              <div className="activity-icon">
                {item.kind === "file" ? getFileIcon(item.label) : <Type size={15} />}
              </div>

              <div className="activity-main">
                <span className="activity-label" title={item.label}>
                  {item.label}
                </span>
                <span className="activity-meta">
                  {item.peerName} · {formatRelativeTime(item.timestamp)}
                  {item.bytesSent !== undefined && ` · ${formatBytes(item.bytesSent)}`}
                </span>
              </div>

              <div className="activity-status">
                {item.status === "sent" ? (
                  <span className="badge-ok">
                    <CheckCheck size={13} /> Delivered
                  </span>
                ) : item.status === "sending" ? (
                  <span className="text-accent" style={{ fontSize: "11.5px", fontWeight: 550 }}>
                    <Loader size={12} className="spin inline-icon" /> Sending
                  </span>
                ) : (
                  <span className="badge-err" title={item.error}>
                    <CircleAlert size={13} /> Failed
                  </span>
                )}
              </div>

              <div className="activity-actions">
                {item.kind === "text" && (
                  <button
                    type="button"
                    className="btn-icon"
                    title="Copy text"
                    onClick={() => onCopy(item.label)}
                  >
                    <Copy size={13} />
                  </button>
                )}
                {item.status === "failed" && (
                  <button
                    type="button"
                    className="btn btn-secondary btn-xs"
                    onClick={() => onRetry(item)}
                  >
                    Retry
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// -----------------------------------------------------------------------------
// View 4: Settings (Clean, Flat, Linear list)
// -----------------------------------------------------------------------------

interface SettingsViewProps {
  identity: DeviceIdentity | null;
  theme: Theme;
  accent: AccentName;
  onThemeChange: (theme: Theme) => void;
  onAccentChange: (accent: AccentName) => void;
  onCopy: (text: string) => void;
}

function SettingsView(props: SettingsViewProps) {
  const { identity, theme, accent, onThemeChange, onAccentChange, onCopy } = props;
  const [appVersion, setAppVersion] = useState("");

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => setAppVersion(""));
  }, []);

  return (
    <div className="settings-page">
      <div className="page-header">
        <div>
          <h2 className="page-title">Settings</h2>
          <p className="page-subtitle">Preferences, theme appearance, and cryptographic identities</p>
        </div>
      </div>

      {/* Appearance Section */}
      <section className="settings-section">
        <h3 className="settings-heading">Appearance</h3>

        <div className="settings-row">
          <div className="settings-label-wrap">
            <span className="settings-label">Interface Theme</span>
            <span className="settings-desc">Choose dark, light, or follow operating system</span>
          </div>
          <div className="mode-segmented">
            {(["system", "light", "dark"] as const).map((t) => (
              <button
                key={t}
                type="button"
                className={`mode-btn ${theme === t ? "active" : ""}`}
                onClick={() => onThemeChange(t)}
              >
                {t === "system" && "Auto"}
                {t === "light" && "Light"}
                {t === "dark" && "Dark"}
              </button>
            ))}
          </div>
        </div>

        <div className="settings-row">
          <div className="settings-label-wrap">
            <span className="settings-label">Accent Color</span>
            <span className="settings-desc">Interface highlight tone</span>
          </div>
          <div className="swatch-group" role="radiogroup" aria-label="Accent Palette">
            {ACCENT_PALETTE.map((pal) => (
              <button
                key={pal.id}
                type="button"
                className={`swatch-dot ${accent === pal.id ? "active" : ""}`}
                style={{ backgroundColor: pal.base }}
                title={pal.label}
                aria-label={pal.label}
                onClick={() => onAccentChange(pal.id)}
              />
            ))}
          </div>
        </div>
      </section>

      {/* Identity & Keys */}
      {identity && (
        <section className="settings-section">
          <h3 className="settings-heading">Device Identity & Keys</h3>

          <div className="settings-row">
            <div className="settings-label-wrap">
              <span className="settings-label">Computer Name</span>
              <span className="settings-desc">Host name advertised to local network peers</span>
            </div>
            <span style={{ fontWeight: 600 }}>{identity.deviceName}</span>
          </div>

          <div className="settings-row">
            <div className="settings-label-wrap">
              <span className="settings-label">Public Identity Fingerprint</span>
              <span className="settings-desc">Ed25519 identity key hash</span>
            </div>
            <div className="code-field">
              <code>{identity.fingerprint}</code>
              <button
                type="button"
                className="btn-icon"
                title="Copy fingerprint"
                onClick={() => onCopy(identity.fingerprint)}
              >
                <Copy size={13} />
              </button>
            </div>
          </div>

          <div className="settings-row">
            <div className="settings-label-wrap">
              <span className="settings-label">Transport Certificate SPKI</span>
              <span className="settings-desc">Pinned QUIC TLS 1.3 key digest</span>
            </div>
            <div className="code-field">
              <code>{identity.spkiHash.slice(0, 20)}...</code>
              <button
                type="button"
                className="btn-icon"
                title="Copy SPKI hash"
                onClick={() => onCopy(identity.spkiHash)}
              >
                <Copy size={13} />
              </button>
            </div>
          </div>
        </section>
      )}

      {/* About */}
      <section className="settings-section">
        <h3 className="settings-heading">About</h3>
        <div className="settings-row">
          <div className="settings-label-wrap">
            <span className="settings-label">Continue Desktop</span>
            <span className="settings-desc">Version {appVersion || "0.1.0"} · Local-first, private by default</span>
          </div>
          <span className="text-muted" style={{ fontSize: "12px" }}>Open Source</span>
        </div>
      </section>
    </div>
  );
}

/** Paired devices connect on their own; this is the fallback for when they can't find each other. */
function ManualConnect(props: {
  initialAddress?: string;
  isConnecting: boolean;
  onConnect: (address: string) => void;
}) {
  const { initialAddress, isConnecting, onConnect } = props;
  const [open, setOpen] = useState(false);
  const [address, setAddress] = useState(initialAddress ?? "");

  if (!open) {
    return (
      <button type="button" className="btn btn-ghost btn-xs" onClick={() => setOpen(true)}>
        Connect manually
      </button>
    );
  }

  return (
    <div className="inline-connect">
      <input
        type="text"
        className="input-sm font-mono"
        placeholder="192.168.1.20:47470"
        autoFocus
        value={address}
        onChange={(e) => setAddress(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && onConnect(address)}
      />
      <button
        type="button"
        className="btn btn-primary btn-sm"
        disabled={isConnecting || !address.trim()}
        onClick={() => onConnect(address)}
      >
        {isConnecting ? <Loader size={12} className="spin inline-icon" /> : null}
        {isConnecting ? "Connecting" : "Connect"}
      </button>
    </div>
  );
}
