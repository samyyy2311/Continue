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
  FolderOpen,
  History,
  Home,
  Info,
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
import { ButtonGroup, DeviceGlyph, ProgressBar, Switch } from "./components.tsx";
import {
  cancelIncoming,
  clearHistory,
  connectToPeer,
  disconnectPeer,
  reconnectPeer,
  errorMessage,
  getAutostart,
  getDeviceIdentity,
  getHistory,
  getSaveFolder,
  getPermissions,
  getTrustedPeers,
  listIncoming,
  onIncomingEnded,
  onIncomingProgress,
  openLink,
  openReceived,
  savePastedFile,
  setAutostart,
  setSaveFolder,
  setClipboardSyncEnabled,
  removeTrustedPeer,
  sendClipboardText,
  sendFileToPeer,
  setPermission,
} from "./api.ts";
import {
  dayLabel,
  fileNameFromPath,
  formatBytes,
  formatRelativeTime,
  getFileCategory,
  linkIn,
} from "./format.ts";
import { PairDialog } from "./PairDialog.tsx";
import {
  ACCENT_PALETTE,
  type AccentName,
  type Activity,
  type DeviceIdentity,
  type HistoryEntry,
  GRANT_OPTIONS,
  type Grant,
  type HistoryFilter,
  type IncomingTransfer,
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
const PEER_KEY = "continue.peer";
const CLIPBOARD_SYNC_KEY = "continue.clipboardSync";

/** Storage can be unavailable in restrictive environments, so reads fall back to nothing. */
function readStored(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function readChoice<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  const stored = readStored(key);
  return allowed.includes(stored as T) ? (stored as T) : fallback;
}

function writeStored(key: string, value: string | null) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    // Storage can be unavailable in restrictive environments.
  }
}

/** Still going: being sent or coming in. */
function isMoving(item: Activity) {
  return item.status === "sending" || item.status === "receiving";
}

function fromIncoming(file: IncomingTransfer): Activity {
  return {
    id: `in-${file.transferId}`,
    kind: "file",
    label: file.fileName,
    peerId: file.peerId,
    peerName: file.peerName,
    status: "receiving",
    timestamp: Date.now(),
    bytesSent: file.received,
    totalBytes: file.total,
    transferId: file.transferId,
  };
}

/** Puts a file that's coming in at the top, or moves its row along. */
function showIncoming(list: Activity[], file: IncomingTransfer): Activity[] {
  const row = fromIncoming(file);
  const existing = list.find((item) => item.id === row.id);
  if (!existing) return [row, ...list];
  return list.map((item) => (item.id === row.id ? { ...row, timestamp: existing.timestamp } : item));
}

function fromHistory(entry: HistoryEntry): Activity {
  return {
    id: `h-${entry.id}`,
    kind: entry.kind,
    label: entry.label,
    peerId: entry.peerId,
    peerName: entry.peerName,
    status: entry.received ? "received" : entry.failed ? "failed" : "sent",
    timestamp: entry.at,
    path: entry.location ?? undefined,
    bytesSent: entry.size,
    totalBytes: entry.size,
  };
}

/** Screenshots paste as a bare "image.png"; give them a name worth keeping. */
function pastedName(file: File) {
  if (file.name && file.name !== "image.png") return file.name;
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  const stamp = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}.${pad(d.getMinutes())}.${pad(d.getSeconds())}`;
  return `Pasted image ${stamp}.png`;
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
    readChoice(ACCENT_KEY, ACCENT_PALETTE.map((a) => a.id), "cobalt"),
  );
  const [theme, setTheme] = useState<Theme>(() => readChoice(THEME_KEY, ["system", "light", "dark"], "system"));
  const [identity, setIdentity] = useState<DeviceIdentity | null>(null);
  const [peers, setPeers] = useState<TrustedPeer[] | null>(null);
  const [loadError, setLoadError] = useState("");
  // The last device picked is picked again next time.
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(() => readStored(PEER_KEY));
  const [showPairDialog, setShowPairDialog] = useState(false);
  const [activity, setActivity] = useState<Activity[]>([]);
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

  useEffect(() => writeStored(PEER_KEY, selectedPeerId), [selectedPeerId]);

  const [clipboardSync, setClipboardSync] = useState(() => readChoice(CLIPBOARD_SYNC_KEY, ["on", "off"], "on") === "on");
  useEffect(() => {
    writeStored(CLIPBOARD_SYNC_KEY, clipboardSync ? "on" : "off");
    if (isTauri()) setClipboardSyncEnabled(clipboardSync).catch(() => {});
  }, [clipboardSync]);

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
      setLoadError("Open Continue from your apps to use it.");
      return;
    }
    let active = true;
    Promise.all([getDeviceIdentity(), getTrustedPeers(), getHistory(), listIncoming()])
      .then(([loadedIdentity, loadedPeers, history, incoming]) => {
        if (!active) return;
        setIdentity(loadedIdentity);
        setPeers(loadedPeers);
        // Anything sent since the window opened stays on top, below files still coming in.
        setActivity((live) => [...incoming.reduce(showIncoming, live), ...history.map(fromHistory)]);
      })
      .catch((error) => active && setLoadError(errorMessage(error)));
    return () => {
      active = false;
    };
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

        const unFile = await listen<{
          peerId: string;
          peerName: string;
          fileName: string;
          path: string;
          bytesReceived: number;
        }>(
          "file-received",
          (event) => {
            showToast(`Received ${event.payload.fileName}`);
            setActivity((prev) => [
              {
                id: `rx-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
                kind: "file",
                label: event.payload.fileName,
                peerId: event.payload.peerId,
                peerName: event.payload.peerName,
                status: "received",
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

        const unClip = await listen<{ peerId: string; peerName: string; content: string }>(
          "clipboard-received",
          (event) => {
            // The app has already put it on the clipboard, even if this window is in the background.
            showToast(`Copied text from ${event.payload.peerName}`);
            setActivity((prev) => [
              {
                id: `rx-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
                kind: "text",
                label: event.payload.content,
                peerId: event.payload.peerId,
                peerName: event.payload.peerName,
                status: "received",
                timestamp: Date.now(),
              },
              ...prev,
            ]);
          },
        );
        cleanups.push(unClip);

        const unProgress = await onIncomingProgress((file) => setActivity((prev) => showIncoming(prev, file)));
        cleanups.push(unProgress);
        // An arrival also comes through file-received, which adds the finished row.
        const unEnded = await onIncomingEnded((transferId) =>
          setActivity((prev) => prev.filter((item) => item.id !== `in-${transferId}`)),
        );
        cleanups.push(unEnded);

        const unSynced = await listen<{ peerId: string; peerName: string; text: string; failed: boolean }>(
          "clipboard-synced",
          ({ payload }) =>
            setActivity((prev) => [
              {
                id: `sync-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
                kind: "text",
                label: payload.text,
                peerId: payload.peerId,
                peerName: payload.peerName,
                status: payload.failed ? "failed" : "sent",
                timestamp: Date.now(),
              },
              ...prev,
            ]),
        );
        cleanups.push(unSynced);
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

  const handleClearHistory = () => {
    setActivity((prev) => prev.filter(isMoving));
    clearHistory().catch((error) => showError(errorMessage(error)));
  };

  const handleReconnect = async (peer: TrustedPeer) => {
    try {
      await reconnectPeer(peer.fingerprint);
      showToast(`Looking for ${peer.displayName}`);
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const handleDisconnect = async (peer: TrustedPeer) => {
    try {
      await disconnectPeer(peer.fingerprint);
      await refreshPeers();
      showToast(`Disconnected from ${peer.displayName}`);
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const handleRemovePeer = async (peer: TrustedPeer) => {
    try {
      await removeTrustedPeer(peer.fingerprint);
      showToast(`Forgot ${peer.displayName}`);
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
    return trackTransfer(
      { kind: "file", label: fileNameFromPath(path), path, peerId: peer.fingerprint, peerName: peer.displayName },
      (update) => sendFileToPeer(peer.fingerprint, path, (progress) => update(progress)),
    );
  };

  const sendText = (peer: TrustedPeer, text: string) => {
    return trackTransfer({ kind: "text", label: text, peerId: peer.fingerprint, peerName: peer.displayName }, () =>
      sendClipboardText(peer.fingerprint, text),
    );
  };

  /** The selected device if it's ready to send to; otherwise says why not. */
  const readyPeer = () => {
    if (!selectedPeer) {
      showError("Pair your phone first.");
      return null;
    }
    if (!selectedPeer.isConnected) {
      showError(`Connect to ${selectedPeer.displayName} first.`);
      return null;
    }
    return selectedPeer;
  };

  const sendFiles = async (paths: string[]) => {
    const peer = readyPeer();
    if (!peer) return;
    for (const path of paths) {
      await sendFile(peer, path);
    }
  };

  const retryItem = (item: Activity) => {
    const peer = peers?.find((p) => p.fingerprint === item.peerId);
    if (!peer) {
      showError(`${item.peerName} isn't paired any more.`);
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
    if (!readyPeer()) return;
    const picked = await openFileDialog({ multiple: true, directory: false });
    if (picked) {
      const paths = Array.isArray(picked) ? picked : [picked];
      sendFiles(paths);
    }
  };

  const handleSendText = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const text = textInput.trim();
    const peer = readyPeer();
    if (!peer || !text) return;
    setTextInput("");
    const sent = await sendText(peer, text);
    if (!sent) setTextInput(text);
  };

  const handleSendClipboard = async () => {
    const peer = readyPeer();
    if (!peer) return;
    try {
      const text = await navigator.clipboard.readText();
      if (!text || !text.trim()) {
        showError("There's nothing copied to send.");
        return;
      }
      await sendText(peer, text.trim());
      showToast(`Sent what you copied to ${peer.displayName}`);
    } catch {
      showError("Couldn't read what you copied. Paste it into the text box instead.");
    }
  };

  /** Pasting on Home sends what was copied: files if there are any, otherwise text. */
  const handlePaste = async (data: DataTransfer) => {
    const files = Array.from(data.files);
    const text = data.getData("text/plain").trim();
    if (files.length === 0 && !text) return;
    const peer = readyPeer();
    if (!peer) return;
    if (files.length === 0) {
      await sendText(peer, text);
      return;
    }
    for (const file of files) {
      try {
        await sendFile(peer, await savePastedFile(file, pastedName(file)));
      } catch (error) {
        showError(errorMessage(error));
      }
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showToast("Copied");
    } catch {
      showError("Couldn't copy that.");
    }
  };

  const rowActions: RowActions = {
    retry: retryItem,
    copy: copyToClipboard,
    open: (path, reveal) => openReceived(path, reveal).catch((error) => showError(errorMessage(error))),
    openLink: (url) => openLink(url).catch((error) => showError(errorMessage(error))),
    cancelIncoming: (transferId) => cancelIncoming(transferId).catch((error) => showError(errorMessage(error))),
  };

  const chooseFilesRef = useRef(chooseFiles);
  chooseFilesRef.current = chooseFiles;

  const handleSendClipboardRef = useRef(handleSendClipboard);
  handleSendClipboardRef.current = handleSendClipboard;

  const handlePasteRef = useRef(handlePaste);
  handlePasteRef.current = handlePaste;
  const viewRef = useRef(view);
  viewRef.current = view;

  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      const typing = e.target instanceof Element && e.target.closest("input, textarea, [contenteditable]");
      if (viewRef.current !== "transfer" || typing || !e.clipboardData || document.querySelector("dialog[open]")) {
        return;
      }
      e.preventDefault();
      void handlePasteRef.current(e.clipboardData);
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, []);

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
      refreshPeers();
    },
    [refreshPeers, showToast],
  );

  if (loadError) {
    return (
      <main className="standalone">
        <DeviceGlyph icon={<Laptop size={28} strokeWidth={1.75} />} size="lg" />
        <h1 className="headline">Continue couldn't start</h1>
        <p className="supporting">{loadError}</p>
        <button type="button" className="btn btn-filled" onClick={() => window.location.reload()}>
          Try again
        </button>
      </main>
    );
  }
  if (peers === null) return null;

  const activeTransfers = activity.filter(isMoving);
  const recentActivity = activity.filter((a) => !isMoving(a)).slice(0, 5);
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
              onReconnect={() => selectedPeer && handleReconnect(selectedPeer)}
              onDisconnect={() => selectedPeer && handleDisconnect(selectedPeer)}
              isConnecting={connecting === selectedPeer?.fingerprint}
              activeTransfers={activeTransfers}
              recentActivity={recentActivity}
              rowActions={rowActions}
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
              onReconnect={handleReconnect}
              onDisconnect={handleDisconnect}
              onUnpair={handleRemovePeer}
              connectingId={connecting}
              onError={showError}
            />
          )}

          {view === "history" && (
            <HistoryView activity={activity} rowActions={rowActions} onClear={handleClearHistory} />
          )}

          {view === "settings" && (
            <SettingsView
              identity={identity}
              theme={theme}
              accent={accent}
              onThemeChange={setTheme}
              onAccentChange={setAccent}
              clipboardSync={clipboardSync}
              onClipboardSyncChange={setClipboardSync}
              onError={showError}
              onOpenFolder={(folder) => rowActions.open(folder, false)}
            />
          )}
        </div>
      </main>

      {dragCount !== null && (
        <div className="drop-scrim" aria-hidden="true">
          <div className="drop-target">
            <DeviceGlyph icon={<Upload size={28} strokeWidth={1.75} />} active={selectedPeer?.isConnected} size="lg" />
            <p className="headline">
              {selectedPeer?.isConnected ? `Drop to send to ${selectedPeer.displayName}` : "Connect your phone first"}
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

/** What a row in Recent or Activity can do. */
interface RowActions {
  retry: (item: Activity) => void;
  copy: (text: string) => void;
  open: (path: string, reveal: boolean) => void;
  openLink: (url: string) => void;
  cancelIncoming: (transferId: string) => void;
}

/** Under a day heading (`underDay`), older rows show the time rather than repeat the day. */
function ActivityRow(props: { item: Activity; actions: RowActions; underDay?: boolean }) {
  const { item, actions, underDay = false } = props;
  const when =
    underDay && dayLabel(item.timestamp) !== "Today"
      ? new Date(item.timestamp).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })
      : formatRelativeTime(item.timestamp);
  const openable = item.status === "received" && item.kind === "file" ? item.path : undefined;
  const link = item.kind === "text" && item.status !== "sending" ? linkIn(item.label) : null;
  const moving = isMoving(item);
  const progress = moving && item.totalBytes ? (item.bytesSent ?? 0) / item.totalBytes : null;
  const incoming = item.status === "received" || item.status === "receiving";
  const who = incoming ? `From ${item.peerName}` : `To ${item.peerName}`;
  return (
    <li className={`list-item ${item.status}`}>
      <span className="list-leading">{item.kind === "file" ? getFileIcon(item.label) : <Type size={18} />}</span>
      <div className="list-text">
        <span className="list-title" title={item.label}>
          {item.label}
        </span>
        {moving ? (
          <>
            <ProgressBar value={progress} label={`${incoming ? "Receiving" : "Sending"} ${item.label}`} />
            <span className="list-sub">
              {item.totalBytes
                ? `${formatBytes(item.bytesSent ?? 0)} of ${formatBytes(item.totalBytes)}`
                : "Getting ready"}{" "}
              · {who}
            </span>
          </>
        ) : (
          <span className="list-sub">
            {who} ·{" "}
            {when}
            {item.kind === "file" && item.bytesSent !== undefined && ` · ${formatBytes(item.bytesSent)}`}
          </span>
        )}
      </div>
      <div className="list-trailing">
        {progress !== null && <span className="status-text">{Math.round(progress * 100)}%</span>}
        {item.status === "receiving" && item.transferId && (
          <button
            type="button"
            className="btn btn-text btn-small"
            onClick={() => item.transferId && actions.cancelIncoming(item.transferId)}
          >
            Cancel
          </button>
        )}
        {item.status === "failed" && (
          <>
            <span className="status-text error" title={item.error}>
              Didn't send
            </span>
            <button type="button" className="btn btn-tonal btn-small" onClick={() => actions.retry(item)}>
              Retry
            </button>
          </>
        )}
        {openable && (
          <>
            <button type="button" className="icon-btn" title="Show in folder" onClick={() => actions.open(openable, true)}>
              <FolderOpen size={18} />
            </button>
            <button type="button" className="btn btn-tonal btn-small" onClick={() => actions.open(openable, false)}>
              Open
            </button>
          </>
        )}
        {item.status === "sent" && <CheckCheck size={18} className="delivered" aria-label="Delivered" />}
        {item.kind === "text" && item.status !== "sending" && (
          <button type="button" className="icon-btn" title="Copy" onClick={() => actions.copy(item.label)}>
            <Copy size={18} />
          </button>
        )}
        {link && (
          <button type="button" className="btn btn-tonal btn-small" onClick={() => actions.openLink(link)}>
            Open link
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
  onReconnect: () => void;
  onDisconnect: () => void;
  isConnecting: boolean;
  activeTransfers: Activity[];
  recentActivity: Activity[];
  rowActions: RowActions;
  onNavigateHistory: () => void;
}

function HomeView(props: HomeViewProps) {
  const { peer, peers, onSelectPeer, onOpenPair, onChooseFiles, onSendClipboard, textInput } = props;
  const { onTextInputChange, onSendText, onConnect, onReconnect, onDisconnect, isConnecting } = props;
  const { activeTransfers, recentActivity, rowActions, onNavigateHistory } = props;

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
            ? "Drop or paste files anywhere in this window to send them."
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
              onReconnect={onReconnect}
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
              <ActivityRow key={item.id} item={item} actions={rowActions} />
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
  onReconnect: (peer: TrustedPeer) => Promise<void>;
  onDisconnect: (peer: TrustedPeer) => Promise<void>;
  onUnpair: (peer: TrustedPeer) => Promise<void>;
  connectingId: string | null;
  onError: (msg: string) => void;
}

function DevicesView(props: DevicesViewProps) {
  const { peers, onSelectPeer, onOpenPair, onConnect, onReconnect, onDisconnect, onUnpair, connectingId, onError } =
    props;
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
            onReconnect={() => onReconnect(peer)}
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
  onReconnect: () => void;
  onDisconnect: () => void;
  onUnpair: () => void;
  onError: (msg: string) => void;
}) {
  const { peer, isConnecting, onSelect, onConnect, onReconnect, onDisconnect, onUnpair, onError } = props;
  const [permissions, setPermissions] = useState<PeerPermission[] | null>(null);
  const [confirmingForget, setConfirmingForget] = useState(false);

  useEffect(() => {
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
            {peer.isConnected ? "Connected" : "Not connected"}
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
            <ManualConnect
              initialAddress={peer.endpoint}
              isConnecting={isConnecting}
              onConnect={onConnect}
              onReconnect={onReconnect}
            />
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
  rowActions: RowActions;
  onClear: () => void;
}

const HISTORY_FILTERS = [
  { value: "all", label: "All" },
  { value: "file", label: "Files" },
  { value: "text", label: "Text" },
  { value: "failed", label: "Failed" },
] as const;

/** Splits newest-first activity into days, keeping the order. */
function byDay(items: Activity[]): [string, Activity[]][] {
  const days: [string, Activity[]][] = [];
  for (const item of items) {
    const day = dayLabel(item.timestamp);
    const last = days[days.length - 1];
    if (last?.[0] === day) last[1].push(item);
    else days.push([day, [item]]);
  }
  return days;
}

function HistoryView(props: HistoryViewProps) {
  const { activity, rowActions, onClear } = props;
  const [filter, setFilter] = useState<HistoryFilter>("all");
  const [search, setSearch] = useState("");
  const [confirmingClear, setConfirmingClear] = useState(false);

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
          <button
            type="button"
            className={`btn ${confirmingClear ? "btn-danger" : "btn-text"}`}
            onClick={() => {
              if (confirmingClear) onClear();
              setConfirmingClear(!confirmingClear);
            }}
            onBlur={() => setConfirmingClear(false)}
          >
            <Trash2 size={18} />
            {confirmingClear ? "Click again to clear all" : "Clear"}
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
        <p className="supporting">{search ? `Nothing matches "${search}".` : "Nothing sent or received yet."}</p>
      ) : (
        byDay(filtered).map(([day, items]) => (
          <section key={day} className="day">
            <h2 className="day-label">{day}</h2>
            <ul className="list">
              {items.map((item) => (
                <ActivityRow key={item.id} item={item} actions={rowActions} underDay />
              ))}
            </ul>
          </section>
        ))
      )}
    </div>
  );
}

interface SettingsViewProps {
  identity: DeviceIdentity | null;
  clipboardSync: boolean;
  onClipboardSyncChange: (on: boolean) => void;
  onError: (message: string) => void;
  onOpenFolder: (folder: string) => void;
  theme: Theme;
  accent: AccentName;
  onThemeChange: (theme: Theme) => void;
  onAccentChange: (accent: AccentName) => void;
}

const THEME_OPTIONS = [
  { value: "system", label: "Auto" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
] as const;

function SettingsView(props: SettingsViewProps) {
  const { identity, theme, accent, onThemeChange, onAccentChange, clipboardSync, onClipboardSyncChange, onError } = props;
  const { onOpenFolder } = props;
  const [appVersion, setAppVersion] = useState("");
  const [startAtLogin, setStartAtLogin] = useState(false);
  const [saveFolder, setSaveFolderShown] = useState("");

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => setAppVersion(""));
    getAutostart().then(setStartAtLogin).catch(() => {});
    getSaveFolder().then(setSaveFolderShown).catch(() => {});
  }, []);

  const chooseSaveFolder = async () => {
    const picked = await openFileDialog({ directory: true, multiple: false, defaultPath: saveFolder || undefined });
    if (typeof picked !== "string") return;
    setSaveFolder(picked)
      .then(setSaveFolderShown)
      .catch((error) => onError(errorMessage(error)));
  };

  const changeStartAtLogin = (on: boolean) => {
    setStartAtLogin(on);
    setAutostart(on).catch((error) => {
      setStartAtLogin(!on);
      onError(errorMessage(error));
    });
  };

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
            <span className="list-sub">Auto matches your computer's setting</span>
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

      <h2 className="label">Running</h2>
      <ul className="list">
        <li className="list-item">
          <div className="list-text">
            <span className="list-title" id="start-at-login-label">
              Open when you log in
            </span>
            <span className="list-sub wrap">
              Continue starts in the tray, ready to receive. Closing the window keeps it there; quit from the tray icon.
            </span>
          </div>
          <Switch labelledBy="start-at-login-label" checked={startAtLogin} onChange={changeStartAtLogin} />
        </li>
      </ul>

      <h2 className="label">Received files</h2>
      <ul className="list">
        <li className="list-item">
          <div className="list-text">
            <span className="list-title">Save to</span>
            <span className="list-sub" title={saveFolder}>
              {saveFolder}
            </span>
          </div>
          <div className="list-trailing">
            <button
              type="button"
              className="icon-btn"
              title="Open folder"
              disabled={!saveFolder}
              onClick={() => onOpenFolder(saveFolder)}
            >
              <FolderOpen size={18} />
            </button>
            <button type="button" className="btn btn-tonal btn-small" onClick={chooseSaveFolder}>
              Change
            </button>
          </div>
        </li>
      </ul>

      <h2 className="label">Clipboard</h2>
      <ul className="list">
        <li className="list-item">
          <div className="list-text">
            <span className="list-title" id="clipboard-sync-label">
              Send what you copy
            </span>
            <span className="list-sub wrap">
              Text you copy here goes to your connected phone straight away. Passwords from password managers are
              left out.
            </span>
          </div>
          <Switch labelledBy="clipboard-sync-label" checked={clipboardSync} onChange={onClipboardSyncChange} />
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
          </ul>
        </>
      )}

      <h2 className="label">About</h2>
      <ul className="list">
        {appVersion && (
          <li className="list-item">
            <span className="list-leading">
              <Info size={20} />
            </span>
            <div className="list-text">
              <span className="list-title">Version</span>
              <span className="list-sub">{appVersion}</span>
            </div>
          </li>
        )}
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
/** Connect normally, or by typing the address when the devices can't find each other. */
function ManualConnect(props: {
  initialAddress?: string;
  isConnecting: boolean;
  onConnect: (address: string) => void;
  onReconnect: () => void;
}) {
  const { initialAddress, isConnecting, onConnect, onReconnect } = props;
  const [open, setOpen] = useState(false);
  const [address, setAddress] = useState(initialAddress ?? "");

  if (!open) {
    return (
      <>
        <button type="button" className="btn btn-filled" onClick={onReconnect}>
          Connect
        </button>
        <button type="button" className="btn btn-text" onClick={() => setOpen(true)}>
          Connect by address
        </button>
      </>
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
