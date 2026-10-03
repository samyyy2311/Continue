// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowUp,
  Check,
  CheckCheck,
  CircleAlert,
  ClipboardPaste,
  Code,
  Copy,
  File,
  FileArchive,
  FileAudio,
  FileCode,
  FileImage,
  FileText,
  FileVideo,
  History,
  Home,
  Info,
  KeyRound,
  Laptop,
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
import { ButtonGroup, DeviceGlyph, ProgressBar } from "./components.tsx";
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
    readStored(ACCENT_KEY, ACCENT_PALETTE.map((a) => a.id), "cyan"),
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
      <main className="standalone">
        <DeviceGlyph icon={<Laptop size={28} strokeWidth={1.75} />} size="lg" />
        <h1 className="headline">{loadError ? "Continue couldn't start" : "Continue"}</h1>
        <p className="supporting">
          {loadError || "Open this window from the Continue app, or start it with pnpm tauri dev."}
        </p>
        {isTauri() && (
          <button type="button" className="btn btn-filled" onClick={() => window.location.reload()}>
            Try again
          </button>
        )}
      </main>
    );
  }

  const activeTransfers = activity.filter((a) => a.status === "sending");
  const recentActivity = activity.filter((a) => a.status !== "sending").slice(0, 5);
  const destinations = [
    { id: "transfer", label: "Home", icon: <Home size={22} />, badge: activeTransfers.length || null },
    { id: "devices", label: "Devices", icon: <Smartphone size={22} />, badge: null },
    { id: "history", label: "Activity", icon: <History size={22} />, badge: null },
    { id: "settings", label: "Settings", icon: <Settings size={22} />, badge: null },
  ] as const;

  return (
    <div className="shell">
      <nav className="rail" aria-label="Main">
        <img src="/icon.svg" alt="Continue" className="rail-logo" />
        <div className="rail-items">
          {destinations.map((item, index) => (
            <button
              key={item.id}
              type="button"
              className="rail-item"
              aria-current={view === item.id ? "page" : undefined}
              onClick={() => setView(item.id)}
              title={`${item.label} (${MOD_KEY}${index + 1})`}
            >
              <span className="rail-indicator">
                {item.icon}
                {item.badge !== null && <span className="rail-badge">{item.badge}</span>}
              </span>
              <span className="rail-label">{item.label}</span>
            </button>
          ))}
        </div>
      </nav>

      <main className="pane">
        <div className="pane-scroll" key={view}>
          {view === "transfer" && (
            <HomeView
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
              onDisconnect={() => selectedPeer && handleDisconnect(selectedPeer)}
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
            />
          )}

          {view === "history" && (
            <HistoryView activity={activity} onRetry={retryItem} onCopy={copyToClipboard} onClear={() => setActivity([])} />
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

      {dragCount !== null && (
        <div className="drop-scrim" aria-hidden="true">
          <div className="drop-target">
            <DeviceGlyph icon={<Upload size={28} strokeWidth={1.75} />} active={selectedPeer?.isConnected} size="lg" />
            <p className="headline">
              {selectedPeer?.isConnected ? `Drop to send to ${selectedPeer.displayName}` : "Connect a device first"}
            </p>
            {selectedPeer?.isConnected && dragCount > 0 && (
              <p className="supporting">{dragCount === 1 ? "1 file" : `${dragCount} files`}</p>
            )}
          </div>
        </div>
      )}

      {showPairDialog && <PairDialog onPaired={handlePaired} onClose={() => setShowPairDialog(false)} />}

      {toast && (
        <div className={`snackbar ${toast.tone}`} role={toast.tone === "error" ? "alert" : "status"}>
          {toast.tone === "error" ? <CircleAlert size={18} /> : <Check size={18} />}
          <span>{toast.message}</span>
        </div>
      )}
    </div>
  );
}

function ActivityRow(props: { item: Activity; onRetry: (item: Activity) => void; onCopy: (text: string) => void }) {
  const { item, onRetry, onCopy } = props;
  const progress =
    item.status === "sending" && item.totalBytes ? (item.bytesSent ?? 0) / item.totalBytes : null;
  return (
    <li className={`list-item ${item.status}`}>
      <span className="list-leading">{item.kind === "file" ? getFileIcon(item.label) : <Type size={18} />}</span>
      <div className="list-text">
        <span className="list-title" title={item.label}>
          {item.label}
        </span>
        {item.status === "sending" ? (
          <>
            <ProgressBar value={progress} label={`Sending ${item.label}`} />
            <span className="list-sub">
              {item.totalBytes
                ? `${formatBytes(item.bytesSent ?? 0)} of ${formatBytes(item.totalBytes)}`
                : "Getting ready"}{" "}
              · {item.peerName}
            </span>
          </>
        ) : (
          <span className="list-sub">
            {item.peerName} · {formatRelativeTime(item.timestamp)}
            {item.kind === "file" && item.bytesSent !== undefined && ` · ${formatBytes(item.bytesSent)}`}
          </span>
        )}
      </div>
      <div className="list-trailing">
        {item.status === "failed" && (
          <>
            <span className="status-text error" title={item.error}>
              Didn't send
            </span>
            <button type="button" className="btn btn-tonal btn-small" onClick={() => onRetry(item)}>
              Retry
            </button>
          </>
        )}
        {item.status === "sent" && <CheckCheck size={18} className="delivered" aria-label="Delivered" />}
        {item.kind === "text" && item.status !== "sending" && (
          <button type="button" className="icon-btn" title="Copy" onClick={() => onCopy(item.label)}>
            <Copy size={18} />
          </button>
        )}
      </div>
    </li>
  );
}

interface HomeViewProps {
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
  onDisconnect: () => void;
  isConnecting: boolean;
  activeTransfers: Activity[];
  recentActivity: Activity[];
  onRetry: (item: Activity) => void;
  onCopy: (text: string) => void;
  onNavigateHistory: () => void;
}

function HomeView(props: HomeViewProps) {
  const { peer, peers, onSelectPeer, onOpenPair, onChooseFiles, onSendClipboard, textInput } = props;
  const { onTextInputChange, onSendText, onConnect, onDisconnect, isConnecting } = props;
  const { activeTransfers, recentActivity, onRetry, onCopy, onNavigateHistory } = props;

  if (!peer) {
    return (
      <div className="empty">
        <DeviceGlyph icon={<Smartphone size={28} strokeWidth={1.75} />} size="lg" />
        <h1 className="display">Pair your phone</h1>
        <p className="supporting">
          Scan a code once, then send files and text between your phone and this computer over your own network.
        </p>
        <button type="button" className="btn btn-filled btn-large" onClick={onOpenPair}>
          <Plus size={20} />
          Pair a device
        </button>
      </div>
    );
  }

  const online = peer.isConnected;
  const busy = activeTransfers.length + recentActivity.length > 0;

  return (
    <div className="page home">
      {peers.length > 1 && (
        <div className="chips" role="radiogroup" aria-label="Device">
          {peers.map((p) => (
            <button
              key={p.fingerprint}
              type="button"
              role="radio"
              aria-checked={p.fingerprint === peer.fingerprint}
              className="chip"
              onClick={() => onSelectPeer(p.fingerprint)}
            >
              {p.fingerprint === peer.fingerprint && <Check size={16} />}
              {p.displayName}
            </button>
          ))}
        </div>
      )}

      <section className={`hero ${online ? "online" : ""}`}>
        <div className="hero-head">
          <DeviceGlyph icon={<Smartphone size={28} strokeWidth={1.75} />} active={online} size="lg" />
          <div className="hero-text">
            <h1 className="display">{peer.displayName}</h1>
            <span className={`status ${online ? "online" : ""}`}>{online ? "Connected" : "Not connected"}</span>
          </div>
        </div>
        <p className="supporting">
          {online
            ? `On your network${peer.endpoint ? ` at ${peer.endpoint}` : ""}. Drop files anywhere in this window to send them.`
            : "It connects on its own when both devices are on the same Wi-Fi."}
        </p>
        {online ? (
          <div className="hero-actions">
            <button type="button" className="btn btn-filled btn-large" onClick={onChooseFiles} title={`${MOD_KEY}O`}>
              <Upload size={20} />
              Send files
            </button>
            <button
              type="button"
              className="btn btn-tonal btn-large"
              onClick={onSendClipboard}
              title={`${MOD_SHIFT_KEY}V`}
            >
              <ClipboardPaste size={20} />
              Send clipboard
            </button>
            <button type="button" className="btn btn-text" onClick={onDisconnect}>
              Disconnect
            </button>
          </div>
        ) : (
          <div className="hero-actions">
            <ManualConnect
              key={peer.fingerprint}
              initialAddress={peer.endpoint}
              isConnecting={isConnecting}
              onConnect={onConnect}
            />
          </div>
        )}
      </section>

      <form
        className={`composer ${online ? "" : "disabled"}`}
        onSubmit={(e) => {
          e.preventDefault();
          onSendText();
        }}
      >
        <input
          value={textInput}
          onChange={(e) => onTextInputChange(e.target.value)}
          placeholder={online ? `Send text to ${peer.displayName}` : "Connect to send text"}
          aria-label="Text to send"
          disabled={!online}
        />
        <button type="submit" className="composer-send" disabled={!online || !textInput.trim()} aria-label="Send">
          <ArrowUp size={22} />
        </button>
      </form>

      {busy && (
        <section className="section">
          <div className="section-head">
            <h2 className="title">Recent</h2>
            <button type="button" className="btn btn-text" onClick={onNavigateHistory}>
              See all
            </button>
          </div>
          <ul className="list">
            {[...activeTransfers, ...recentActivity].map((item) => (
              <ActivityRow key={item.id} item={item} onRetry={onRetry} onCopy={onCopy} />
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

interface DevicesViewProps {
  peers: TrustedPeer[];
  onSelectPeer: (id: string) => void;
  onOpenPair: () => void;
  onConnect: (peer: TrustedPeer, address: string) => Promise<void>;
  onDisconnect: (peer: TrustedPeer) => Promise<void>;
  onUnpair: (peer: TrustedPeer) => Promise<void>;
  connectingId: string | null;
  onError: (msg: string) => void;
}

function DevicesView(props: DevicesViewProps) {
  const { peers, onSelectPeer, onOpenPair, onConnect, onDisconnect, onUnpair, connectingId, onError } = props;
  return (
    <div className="page">
      <header className="page-head">
        <h1 className="display">Devices</h1>
        <button type="button" className="btn btn-tonal" onClick={onOpenPair}>
          <Plus size={20} />
          Pair a device
        </button>
      </header>
      {peers.length === 0 ? (
        <p className="supporting">Nothing paired yet. Pair your phone to get started.</p>
      ) : (
        peers.map((peer) => (
          <DeviceCard
            key={peer.fingerprint}
            peer={peer}
            isConnecting={connectingId === peer.fingerprint}
            onSelect={() => onSelectPeer(peer.fingerprint)}
            onConnect={(addr) => onConnect(peer, addr)}
            onDisconnect={() => onDisconnect(peer)}
            onUnpair={() => onUnpair(peer)}
            onError={onError}
          />
        ))
      )}
    </div>
  );
}

function DeviceCard(props: {
  peer: TrustedPeer;
  isConnecting: boolean;
  onSelect: () => void;
  onConnect: (address: string) => void;
  onDisconnect: () => void;
  onUnpair: () => void;
  onError: (msg: string) => void;
}) {
  const { peer, isConnecting, onSelect, onConnect, onDisconnect, onUnpair, onError } = props;
  const [permissions, setPermissions] = useState<PeerPermission[] | null>(null);
  const [confirmingForget, setConfirmingForget] = useState(false);

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

  useEffect(() => {
    if (!confirmingForget) return;
    const timer = window.setTimeout(() => setConfirmingForget(false), 3500);
    return () => window.clearTimeout(timer);
  }, [confirmingForget]);

  const updateGrant = async (permission: PeerPermission, grant: Grant) => {
    const apply = () =>
      setPermissions((prev) =>
        prev ? prev.map((p) => (p.capabilityId === permission.capabilityId ? { ...p, grant } : p)) : null,
      );
    if (!isTauri()) {
      apply();
      return;
    }
    try {
      await setPermission(peer.fingerprint, permission.capabilityId, grant);
      apply();
    } catch (err) {
      onError(errorMessage(err));
    }
  };

  return (
    <section className="card">
      <div className="card-head">
        <DeviceGlyph icon={<Smartphone size={24} strokeWidth={1.75} />} active={peer.isConnected} />
        <div className="card-title">
          <h2 className="headline">{peer.displayName}</h2>
          <span className={`status ${peer.isConnected ? "online" : ""}`}>
            {peer.isConnected ? "Connected" : "Not connected"} · paired {formatPairedDate(peer.pairedAt)}
          </span>
        </div>
        <div className="card-actions">
          {peer.isConnected ? (
            <>
              <button type="button" className="btn btn-filled" onClick={onSelect}>
                <Send size={18} />
                Send
              </button>
              <button type="button" className="btn btn-text" onClick={onDisconnect}>
                Disconnect
              </button>
            </>
          ) : (
            <ManualConnect initialAddress={peer.endpoint} isConnecting={isConnecting} onConnect={onConnect} />
          )}
        </div>
      </div>

      <h3 className="label">What {peer.displayName} can do here</h3>
      <ul className="list">
        {permissions ? (
          permissions.map((perm) => {
            const meta = PERMISSIONS[perm.capabilityName];
            return (
              <li key={perm.capabilityId} className="list-item">
                <div className="list-text">
                  <span className="list-title">{meta?.label ?? perm.capabilityName}</span>
                  {meta && <span className="list-sub">{meta.description}</span>}
                </div>
                <ButtonGroup
                  label={meta?.label ?? perm.capabilityName}
                  options={GRANT_OPTIONS}
                  value={perm.grant}
                  onChange={(grant) => updateGrant(perm, grant)}
                />
              </li>
            );
          })
        ) : (
          <li className="list-item">
            <Loader size={18} className="spin" />
            <span className="list-sub">Loading permissions</span>
          </li>
        )}
      </ul>

      <div className="card-foot">
        <button
          type="button"
          className={`btn ${confirmingForget ? "btn-danger" : "btn-text danger"}`}
          onClick={() => (confirmingForget ? onUnpair() : setConfirmingForget(true))}
        >
          <Trash2 size={18} />
          {confirmingForget ? "Click again to forget" : "Forget this device"}
        </button>
      </div>
    </section>
  );
}

interface HistoryViewProps {
  activity: Activity[];
  onRetry: (item: Activity) => void;
  onCopy: (text: string) => void;
  onClear: () => void;
}

const HISTORY_FILTERS = [
  { value: "all", label: "All" },
  { value: "file", label: "Files" },
  { value: "text", label: "Text" },
  { value: "failed", label: "Failed" },
] as const;

function HistoryView(props: HistoryViewProps) {
  const { activity, onRetry, onCopy, onClear } = props;
  const [filter, setFilter] = useState<HistoryFilter>("all");
  const [search, setSearch] = useState("");

  const needle = search.trim().toLowerCase();
  const filtered = activity.filter((item) => {
    if (filter === "file" && item.kind !== "file") return false;
    if (filter === "text" && item.kind !== "text") return false;
    if (filter === "failed" && item.status !== "failed") return false;
    return !needle || item.label.toLowerCase().includes(needle) || item.peerName.toLowerCase().includes(needle);
  });

  return (
    <div className="page">
      <header className="page-head">
        <h1 className="display">Activity</h1>
        {activity.length > 0 && (
          <button type="button" className="btn btn-text" onClick={onClear}>
            <Trash2 size={18} />
            Clear
          </button>
        )}
      </header>

      <div className="toolbar">
        <div className="chips" role="radiogroup" aria-label="Show">
          {HISTORY_FILTERS.map((option) => (
            <button
              key={option.value}
              type="button"
              role="radio"
              aria-checked={filter === option.value}
              className="chip"
              onClick={() => setFilter(option.value)}
            >
              {filter === option.value && <Check size={16} />}
              {option.label}
            </button>
          ))}
        </div>
        <label className="search">
          <Search size={20} />
          <input
            type="search"
            placeholder="Search"
            aria-label="Search activity"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
      </div>

      {filtered.length === 0 ? (
        <p className="supporting">{search ? `Nothing matches "${search}".` : "Nothing here yet."}</p>
      ) : (
        <ul className="list">
          {filtered.map((item) => (
            <ActivityRow key={item.id} item={item} onRetry={onRetry} onCopy={onCopy} />
          ))}
        </ul>
      )}
    </div>
  );
}

interface SettingsViewProps {
  identity: DeviceIdentity | null;
  theme: Theme;
  accent: AccentName;
  onThemeChange: (theme: Theme) => void;
  onAccentChange: (accent: AccentName) => void;
  onCopy: (text: string) => void;
}

const THEME_OPTIONS = [
  { value: "system", label: "Auto" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
] as const;

function SettingsView(props: SettingsViewProps) {
  const { identity, theme, accent, onThemeChange, onAccentChange, onCopy } = props;
  const [appVersion, setAppVersion] = useState("");

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => setAppVersion(""));
  }, []);

  return (
    <div className="page">
      <header className="page-head">
        <h1 className="display">Settings</h1>
      </header>

      <h2 className="label">Appearance</h2>
      <ul className="list">
        <li className="list-item">
          <div className="list-text">
            <span className="list-title">Theme</span>
            <span className="list-sub">Auto follows your system</span>
          </div>
          <ButtonGroup label="Theme" options={THEME_OPTIONS} value={theme} onChange={onThemeChange} />
        </li>
        <li className="list-item">
          <div className="list-text">
            <span className="list-title">Colour</span>
          </div>
          <div className="swatches" role="radiogroup" aria-label="Colour">
            {ACCENT_PALETTE.map((pal) => (
              <button
                key={pal.id}
                type="button"
                role="radio"
                aria-checked={accent === pal.id}
                className="swatch"
                style={{ backgroundColor: pal.base }}
                title={pal.label}
                aria-label={pal.label}
                onClick={() => onAccentChange(pal.id)}
              >
                {accent === pal.id && <Check size={16} style={{ color: pal.onBase }} />}
              </button>
            ))}
          </div>
        </li>
      </ul>

      {identity && (
        <>
          <h2 className="label">This computer</h2>
          <ul className="list">
            <li className="list-item">
              <span className="list-leading">
                <Laptop size={20} />
              </span>
              <div className="list-text">
                <span className="list-title">Name</span>
                <span className="list-sub">{identity.deviceName}</span>
              </div>
            </li>
            <li className="list-item">
              <span className="list-leading">
                <KeyRound size={20} />
              </span>
              <div className="list-text">
                <span className="list-title">Device key</span>
                <span className="list-sub mono">{identity.fingerprint}</span>
              </div>
              <button type="button" className="icon-btn" title="Copy" onClick={() => onCopy(identity.fingerprint)}>
                <Copy size={18} />
              </button>
            </li>
          </ul>
        </>
      )}

      <h2 className="label">About</h2>
      <ul className="list">
        <li className="list-item">
          <span className="list-leading">
            <Info size={20} />
          </span>
          <div className="list-text">
            <span className="list-title">Version</span>
            <span className="list-sub">{appVersion || "0.1.0"}</span>
          </div>
        </li>
        <li className="list-item">
          <span className="list-leading">
            <Code size={20} />
          </span>
          <div className="list-text">
            <span className="list-title">Open source</span>
            <span className="list-sub">No accounts, no cloud. Read the code on GitHub.</span>
          </div>
        </li>
      </ul>
    </div>
  );
}

/** Paired devices connect on their own; this is the fallback for when they can't find each other. */
function ManualConnect(props: { initialAddress?: string; isConnecting: boolean; onConnect: (address: string) => void }) {
  const { initialAddress, isConnecting, onConnect } = props;
  const [open, setOpen] = useState(false);
  const [address, setAddress] = useState(initialAddress ?? "");

  if (!open) {
    return (
      <button type="button" className="btn btn-outlined" onClick={() => setOpen(true)}>
        Connect by address
      </button>
    );
  }

  return (
    <form
      className="address-form"
      onSubmit={(e) => {
        e.preventDefault();
        onConnect(address);
      }}
    >
      <input
        type="text"
        className="field mono"
        placeholder="192.168.1.20:47470"
        aria-label="Device address"
        autoFocus
        value={address}
        onChange={(e) => setAddress(e.target.value)}
      />
      <button type="submit" className="btn btn-filled" disabled={isConnecting || !address.trim()}>
        {isConnecting && <Loader size={18} className="spin" />}
        {isConnecting ? "Connecting" : "Connect"}
      </button>
    </form>
  );
}
