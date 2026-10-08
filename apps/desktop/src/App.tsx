// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowUp,
  Bell,
  BellOff,
  Check,
  CheckCheck,
  CircleAlert,
  ClipboardPaste,
  Clock,
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
  Monitor,
  Plus,
  Search,
  Send,
  Settings,
  ShieldAlert,
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
import { ButtonGroup, ConnectionStatus, ProgressBar, Switch } from "./components.tsx";
import {
  broadcastHandoff,
  cancelIncoming,
  clearHistory,
  clearQueuedTransfers,
  connectToPeer,
  disconnectPeer,
  dismissHandoff,
  reconnectPeer,
  errorMessage,
  getActiveHandoffs,
  getAutostart,
  getDeviceIdentity,
  getDropFolder,
  getHistory,
  getQueuedTransfers,
  getSaveFolder,
  getPermissions,
  getTrustedPeers,
  isLockdownMode,
  listIncoming,
  onHandoffDismissed,
  onHandoffReceived,
  onIncomingEnded,
  onIncomingProgress,
  onLockdownChanged,
  onNotificationPosted,
  onNotificationRemoved,
  onQueuedTransfersChanged,
  openHandoff,
  openLink,
  openReceived,
  queueOfflineFile,
  queueOfflineText,
  ringPeer,
  savePastedFile,
  setAutostart,
  setDropFolder,
  setLockdownMode,
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
import { CatalogDialog } from "./CatalogDialog.tsx";
import { ClipboardHistoryDialog } from "./ClipboardHistoryDialog.tsx";
import { DesktopStreamDialog } from "./DesktopStreamDialog.tsx";
import { HandoffCard } from "./HandoffCard.tsx";
import { NotificationList } from "./Notifications.tsx";
import { PairDialog } from "./PairDialog.tsx";
import {
  ACCENT_PALETTE,
  type AccentName,
  type Activity,
  type DeviceIdentity,
  type HistoryEntry,
  GRANT_OPTIONS,
  type Grant,
  type HandoffItem,
  type HistoryFilter,
  type IncomingTransfer,
  isMac,
  MOD_KEY,
  MOD_SHIFT_KEY,
  type PeerPermission,
  type PhoneNotification,
  PERMISSIONS,
  type QueuedTransfer,
  type Theme,
  type Toast,
  type TrustedPeer,
  type View,
} from "./types.ts";

/** In rail order; Ctrl/Cmd plus 1 to 4 opens each. */
const VIEW_ORDER: View[] = ["transfer", "devices", "history", "settings"];

const ACCENT_KEY = "continue.accent";
const THEME_KEY = "continue.theme";
const PEER_KEY = "continue.peer";
const CLIPBOARD_SYNC_KEY = "continue.clipboardSync";

/** Storage can be unavailable in restrictive environments, so reads and writes do without. */
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
    // See readStored.
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
      return <FileImage size={18} />;
    case "video":
      return <FileVideo size={18} />;
    case "audio":
      return <FileAudio size={18} />;
    case "archive":
      return <FileArchive size={18} />;
    case "code":
      return <FileCode size={18} />;
    case "document":
      return <FileText size={18} />;
    default:
      return <File size={18} />;
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
  const [showCatalogDialog, setShowCatalogDialog] = useState(false);
  const [showClipboardDialog, setShowClipboardDialog] = useState(false);
  const [showDesktopStreamDialog, setShowDesktopStreamDialog] = useState(false);
  const [isRinging, setIsRinging] = useState(false);
  const [activity, setActivity] = useState<Activity[]>([]);
  const [connecting, setConnecting] = useState<string | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);
  const [dragCount, setDragCount] = useState<number | null>(null);
  const [textInput, setTextInput] = useState("");
  const [notifications, setNotifications] = useState<PhoneNotification[]>([]);
  const [handoffs, setHandoffs] = useState<HandoffItem[]>([]);
  const [isLockdown, setIsLockdown] = useState(false);
  const [dropFolder, setDropFolderState] = useState<string | null>(null);
  const [queuedTransfers, setQueuedTransfers] = useState<QueuedTransfer[]>([]);

  const selectedPeer = peers?.find((p) => p.fingerprint === selectedPeerId) ?? peers?.[0] ?? null;

  const showToast = useCallback((message: string, tone: Toast["tone"] = "info") => {
    setToast({ message, tone });
  }, []);

  const showError = useCallback((message: string) => showToast(message, "error"), [showToast]);

  const handleToggleRing = async () => {
    if (!selectedPeer) return;
    try {
      const next = !isRinging;
      const active = await ringPeer(selectedPeer.fingerprint, next);
      setIsRinging(active);
      if (active) {
        showToast(`Ringing ${selectedPeer.displayName}...`);
      }
    } catch (err) {
      showError(errorMessage(err));
    }
  };

  useEffect(() => {
    setIsRinging(false);
  }, [selectedPeerId]);

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

  // A disconnected phone can't say when its notifications go away, so they're dropped.
  useEffect(() => {
    const connected = new Set(peers?.filter((p) => p.isConnected).map((p) => p.fingerprint));
    setNotifications((prev) => prev.filter((n) => connected.has(n.peerId)));
  }, [peers]);

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
    Promise.all([
      getDeviceIdentity(),
      getTrustedPeers(),
      getHistory(),
      getActiveHandoffs(),
      isLockdownMode(),
      getDropFolder(),
    ])
      .then(([loadedIdentity, loadedPeers, history, loadedHandoffs, lockdownState, folderState]) => {
        if (!active) return;
        setIdentity(loadedIdentity);
        setPeers(loadedPeers);
        setHandoffs(loadedHandoffs);
        setIsLockdown(lockdownState);
        setDropFolderState(folderState);
        // Anything sent or coming in since the window opened stays on top.
        setActivity((live) => [...live, ...history.map(fromHistory)]);
      })
      .catch((error) => active && setLoadError(errorMessage(error)));
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (selectedPeer) {
      getQueuedTransfers(selectedPeer.fingerprint).then(setQueuedTransfers).catch(() => {});
    } else {
      setQueuedTransfers([]);
    }
  }, [selectedPeer?.fingerprint]);

  useEffect(() => {
    if (!isTauri()) return;
    const cleanups: (() => void)[] = [];
    let disposed = false;
    // A listener can finish registering after cleanup has run; drop it straight away then.
    const keep = (unlisten: () => void) => {
      if (disposed) unlisten();
      else cleanups.push(unlisten);
    };

    const setupListeners = async () => {
      try {
        const unPeer = await listen<{ fingerprint: string; displayName: string }>(
          "peer-connected",
          (event) => {
            void refreshPeers();
            showToast(`${event.payload.displayName} connected`);
          },
        );
        keep(unPeer);

        const unState = await listen("peer-state-changed", () => void refreshPeers());
        keep(unState);
        // A device says its name just after connecting, and again after a rename.
        const unRenamed = await listen("peer-renamed", () => void refreshPeers());
        keep(unRenamed);
        const unStatus = await listen("peer-status", () => void refreshPeers());
        keep(unStatus);
        const unPosted = await onNotificationPosted((posted) =>
          setNotifications((prev) => [posted, ...prev.filter((n) => n.id !== posted.id)]),
        );
        keep(unPosted);
        const unRemoved = await onNotificationRemoved((id) =>
          setNotifications((prev) => prev.filter((n) => n.id !== id)),
        );
        keep(unRemoved);

        const addRow = (row: Omit<Activity, "id" | "timestamp">) =>
          setActivity((prev) => [{ ...row, id: crypto.randomUUID(), timestamp: Date.now() }, ...prev]);

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
            addRow({
              kind: "file",
              label: event.payload.fileName,
              peerId: event.payload.peerId,
              peerName: event.payload.peerName,
              status: "received",
              path: event.payload.path,
              bytesSent: event.payload.bytesReceived,
              totalBytes: event.payload.bytesReceived,
            });
          },
        );
        keep(unFile);

        const unClip = await listen<{ peerId: string; peerName: string; content: string }>(
          "clipboard-received",
          (event) => {
            // The app has already put it on the clipboard, even if this window is in the background.
            showToast(`Copied text from ${event.payload.peerName}`);
            addRow({
              kind: "text",
              label: event.payload.content,
              peerId: event.payload.peerId,
              peerName: event.payload.peerName,
              status: "received",
            });
          },
        );
        keep(unClip);

        const ended = new Set<string>();
        // An arrival also comes through file-received, which adds the finished row. Listened
        // for before progress, so a file can't get a row whose end goes unheard.
        const unEnded = await onIncomingEnded((transferId) => {
          ended.add(transferId);
          setActivity((prev) => prev.filter((item) => item.id !== `in-${transferId}`));
        });
        keep(unEnded);
        const unProgress = await onIncomingProgress((file) => setActivity((prev) => showIncoming(prev, file)));
        keep(unProgress);
        // Only now, so nothing that starts meanwhile is missed. Events that arrived
        // while this loaded are newer, so they win over the snapshot.
        const incoming = await listIncoming();
        if (!disposed) {
          setActivity((prev) =>
            incoming
              .filter((file) => !ended.has(file.transferId))
              .reduce(
                (list, file) =>
                  list.some((item) => item.id === `in-${file.transferId}`) ? list : showIncoming(list, file),
                prev,
              ),
          );
        }

        const unSynced = await listen<{ peerId: string; peerName: string; text: string; failed: boolean }>(
          "clipboard-synced",
          ({ payload }) =>
            addRow({
              kind: "text",
              label: payload.text,
              peerId: payload.peerId,
              peerName: payload.peerName,
              status: payload.failed ? "failed" : "sent",
            }),
        );
        keep(unSynced);

        const unHandoffRecv = await onHandoffReceived((item) => {
          setHandoffs((prev) => [item, ...prev.filter((h) => h.handoffId !== item.handoffId)]);
          showToast(`Continue from ${item.sourceDeviceId || "device"}`);
        });
        keep(unHandoffRecv);

        const unHandoffDismiss = await onHandoffDismissed((handoffId) => {
          setHandoffs((prev) => prev.filter((h) => h.handoffId !== handoffId));
        });
        keep(unHandoffDismiss);

        const unLockdown = await onLockdownChanged((active) => setIsLockdown(active));
        keep(unLockdown);

        const unQueue = await onQueuedTransfersChanged((peerId) => {
          if (selectedPeerId === peerId) {
            getQueuedTransfers(peerId).then(setQueuedTransfers).catch(() => {});
          }
        });
        keep(unQueue);
      } catch (error) {
        showError(errorMessage(error));
      }
    };

    void setupListeners();
    return () => {
      disposed = true;
      for (const cleanup of cleanups) cleanup();
    };
  }, [refreshPeers, showToast, showError]);

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
        if (selectedPeer?.isConnected) {
          sendFilesRef.current(payload.paths);
        } else if (selectedPeer) {
          const peer = selectedPeer;
          (async () => {
            for (const p of payload.paths) {
              await queueOfflineFile(peer.fingerprint, p);
            }
            const updated = await getQueuedTransfers(peer.fingerprint);
            setQueuedTransfers(updated);
            showToast(`Queued ${payload.paths.length} file(s) for ${peer.displayName}`);
          })();
        }
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [selectedPeer]);

  const chooseFiles = async () => {
    if (!selectedPeer) {
      showError("Pair your phone first.");
      return;
    }
    const picked = await openFileDialog({ multiple: true, directory: false });
    if (picked) {
      const paths = Array.isArray(picked) ? picked : [picked];
      if (selectedPeer.isConnected) {
        sendFiles(paths);
      } else {
        for (const p of paths) {
          await queueOfflineFile(selectedPeer.fingerprint, p);
        }
        const updated = await getQueuedTransfers(selectedPeer.fingerprint);
        setQueuedTransfers(updated);
        showToast(`Queued ${paths.length} file(s) for ${selectedPeer.displayName}`);
      }
    }
  };

  const handleSendText = async () => {
    const text = textInput.trim();
    if (!selectedPeer || !text) return;
    setTextInput("");
    if (selectedPeer.isConnected) {
      const sent = await sendText(selectedPeer, text);
      if (!sent) setTextInput(text);
    } else {
      try {
        await queueOfflineText(selectedPeer.fingerprint, text);
        const updated = await getQueuedTransfers(selectedPeer.fingerprint);
        setQueuedTransfers(updated);
        showToast(`Queued text for ${selectedPeer.displayName}`);
      } catch (error) {
        showError(errorMessage(error));
        setTextInput(text);
      }
    }
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
    if (!selectedPeer) return;
    if (selectedPeer.isConnected) {
      if (files.length === 0) {
        await sendText(selectedPeer, text);
        return;
      }
      for (const file of files) {
        try {
          await sendFile(selectedPeer, await savePastedFile(file, pastedName(file)));
        } catch (error) {
          showError(errorMessage(error));
        }
      }
    } else {
      if (text) {
        await queueOfflineText(selectedPeer.fingerprint, text);
      }
      for (const file of files) {
        try {
          const saved = await savePastedFile(file, pastedName(file));
          await queueOfflineFile(selectedPeer.fingerprint, saved);
        } catch (error) {
          showError(errorMessage(error));
        }
      }
      const updated = await getQueuedTransfers(selectedPeer.fingerprint);
      setQueuedTransfers(updated);
      showToast(`Queued for ${selectedPeer.displayName}`);
    }
  };

  const handleClearQueuedTransfers = async () => {
    if (!selectedPeer) return;
    try {
      await clearQueuedTransfers(selectedPeer.fingerprint);
      setQueuedTransfers([]);
      showToast("Cleared queued transfers");
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const handleToggleLockdown = async () => {
    try {
      const next = !isLockdown;
      await setLockdownMode(next);
      setIsLockdown(next);
      showToast(next ? "Lockdown active: all capabilities paused" : "Lockdown turned off");
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const handleChooseDropFolder = async () => {
    try {
      const picked = await openFileDialog({ directory: true, multiple: false });
      if (typeof picked === "string") {
        await setDropFolder(picked);
        setDropFolderState(picked);
        showToast("Drop folder configured");
      }
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const handleClearDropFolder = async () => {
    try {
      await setDropFolder(null);
      setDropFolderState(null);
      showToast("Drop folder disabled");
    } catch (error) {
      showError(errorMessage(error));
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

  const handleOpenHandoff = async (item: HandoffItem) => {
    try {
      const peerId = item.peerId || selectedPeer?.fingerprint || "";
      await openHandoff(peerId, item.handoffId, item.uri);
      setHandoffs((prev) => prev.filter((h) => h.handoffId !== item.handoffId));
      showToast("Opened in browser");
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const handleDismissHandoff = async (item: HandoffItem) => {
    try {
      const peerId = item.peerId || selectedPeer?.fingerprint || "";
      await dismissHandoff(peerId, item.handoffId);
      setHandoffs((prev) => prev.filter((h) => h.handoffId !== item.handoffId));
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const handleBroadcastHandoff = async (url: string) => {
    const peer = readyPeer();
    if (!peer) return;
    try {
      await broadcastHandoff(peer.fingerprint, url, url, 1, 0);
      showToast(`Sent to ${peer.displayName} to continue reading`);
    } catch (error) {
      showError(errorMessage(error));
    }
  };

  const rowActions: RowActions = {
    retry: retryItem,
    copy: copyToClipboard,
    open: (path, reveal) => openReceived(path, reveal).catch((error) => showError(errorMessage(error))),
    openLink: (url) => openLink(url).catch((error) => showError(errorMessage(error))),
    cancelIncoming: (transferId) => cancelIncoming(transferId).catch((error) => showError(errorMessage(error))),
    handoffLink: handleBroadcastHandoff,
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
      const mod = isMac ? e.metaKey : e.ctrlKey;
      const target = e.target as HTMLElement | null;
      const isInputFocused =
        target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable);

      const shortcutView = VIEW_ORDER[Number(e.key) - 1];
      if (mod && shortcutView) {
        e.preventDefault();
        setView(shortcutView);
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
        <Laptop size={56} strokeWidth={1.25} className="device-icon" />
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
        <div className="rail-top">
          <div className="rail-logo" title="Continue">
            <img src="/icon.svg" alt="Continue" style={{ width: 22, height: 22 }} />
          </div>
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
              onOpenCatalog={() => setShowCatalogDialog(true)}
              onOpenClipboardHistory={() => setShowClipboardDialog(true)}
              onOpenDesktopStream={() => setShowDesktopStreamDialog(true)}
              isRinging={isRinging}
              onToggleRing={handleToggleRing}
              textInput={textInput}
              onTextInputChange={setTextInput}
              onSendText={handleSendText}
              handoffs={handoffs.filter((h) => !h.peerId || !selectedPeer || h.peerId === selectedPeer.fingerprint)}
              onOpenHandoff={handleOpenHandoff}
              onDismissHandoff={handleDismissHandoff}
              onBroadcastHandoff={handleBroadcastHandoff}
              onConnect={(address) => selectedPeer && handleConnect(selectedPeer, address)}
              onReconnect={() => selectedPeer && handleReconnect(selectedPeer)}
              onDisconnect={() => selectedPeer && handleDisconnect(selectedPeer)}
              isConnecting={connecting === selectedPeer?.fingerprint}
              activeTransfers={activeTransfers}
              recentActivity={recentActivity}
              notifications={notifications.filter((n) => n.peerId === selectedPeer?.fingerprint)}
              onError={showError}
              rowActions={rowActions}
              onNavigateHistory={() => setView("history")}
              isLockdown={isLockdown}
              onToggleLockdown={handleToggleLockdown}
              queuedTransfers={queuedTransfers}
              onClearQueuedTransfers={handleClearQueuedTransfers}
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
              onOpenClipboardHistory={() => setShowClipboardDialog(true)}
              onError={showError}
              onOpenFolder={(folder) => rowActions.open(folder, false)}
              isLockdown={isLockdown}
              onToggleLockdown={handleToggleLockdown}
              dropFolder={dropFolder}
              onChooseDropFolder={handleChooseDropFolder}
              onClearDropFolder={handleClearDropFolder}
            />
          )}
        </div>
      </main>

      {dragCount !== null && (
        <div className="drop-scrim" aria-hidden="true">
          <div className="drop-target">
            <Upload size={40} strokeWidth={1.5} className="text-accent" />
            <p className="headline">
              {selectedPeer
                ? selectedPeer.isConnected
                  ? `Drop to send to ${selectedPeer.displayName}`
                  : `Drop to queue for ${selectedPeer.displayName}`
                : "Connect your phone first"}
            </p>
            {selectedPeer && dragCount > 0 && (
              <p className="supporting">{dragCount === 1 ? "1 file" : `${dragCount} files`}</p>
            )}
          </div>
        </div>
      )}

      {showPairDialog && <PairDialog onPaired={handlePaired} onClose={() => setShowPairDialog(false)} />}

      {showCatalogDialog && selectedPeer && (
        <CatalogDialog
          peer={selectedPeer}
          onClose={() => setShowCatalogDialog(false)}
          onError={showError}
        />
      )}

      {showDesktopStreamDialog && selectedPeer && (
        <DesktopStreamDialog
          peer={selectedPeer}
          onClose={() => setShowDesktopStreamDialog(false)}
          onError={showError}
        />
      )}

      {showClipboardDialog && (
        <ClipboardHistoryDialog
          onClose={() => setShowClipboardDialog(false)}
          onError={showError}
        />
      )}

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
  handoffLink?: (url: string) => void;
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
                : "Getting ready"}
            </span>
          </>
        ) : (
          <span className="list-sub">
            {who}, {when}
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
          <>
            <button type="button" className="btn btn-tonal btn-small" onClick={() => actions.openLink(link)}>
              Open link
            </button>
            {actions.handoffLink && (
              <button
                type="button"
                className="btn btn-tonal btn-small"
                onClick={() => actions.handoffLink!(link)}
                title="Continue reading on device"
              >
                Handoff
              </button>
            )}
          </>
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
  onOpenCatalog: () => void;
  onOpenClipboardHistory: () => void;
  onOpenDesktopStream: () => void;
  isRinging: boolean;
  onToggleRing: () => void;
  textInput: string;
  onTextInputChange: (val: string) => void;
  onSendText: () => void;
  handoffs: HandoffItem[];
  onOpenHandoff: (item: HandoffItem) => void;
  onDismissHandoff: (item: HandoffItem) => void;
  onBroadcastHandoff: (url: string) => void;
  onConnect: (address: string) => void;
  onReconnect: () => void;
  onDisconnect: () => void;
  isConnecting: boolean;
  activeTransfers: Activity[];
  recentActivity: Activity[];
  rowActions: RowActions;
  onNavigateHistory: () => void;
  notifications: PhoneNotification[];
  onError: (message: string) => void;
  isLockdown: boolean;
  onToggleLockdown: () => void;
  queuedTransfers: QueuedTransfer[];
  onClearQueuedTransfers: () => void;
}

function HomeView(props: HomeViewProps) {
  const {
    peer,
    peers,
    onSelectPeer,
    onOpenPair,
    onChooseFiles,
    onSendClipboard,
    onOpenCatalog,
    onOpenClipboardHistory,
    onOpenDesktopStream,
    isRinging,
    onToggleRing,
    textInput,
    handoffs,
    onOpenHandoff,
    onDismissHandoff,
    onBroadcastHandoff,
    isLockdown,
    onToggleLockdown,
    queuedTransfers,
    onClearQueuedTransfers,
  } = props;
  const { onTextInputChange, onSendText, onConnect, onReconnect, onDisconnect, isConnecting } = props;
  const { activeTransfers, recentActivity, rowActions, onNavigateHistory, notifications, onError } = props;

  if (!peer) {
    return (
      <div className="empty">
        <Smartphone size={56} strokeWidth={1.25} className="device-icon" />
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
    <div className="home">
      <aside className="home-device">
        <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
          <div className={`device-icon ${online ? "online" : ""}`}>
            <Smartphone size={24} />
          </div>
          <div style={{ display: "flex", flexDirection: "column", minWidth: 0, flex: 1 }}>
            <h2 className="title" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {peer.displayName}
            </h2>
            <ConnectionStatus peer={peer} />
          </div>
        </div>

        {peer.endpoint && (
          <div style={{ display: "flex", alignItems: "center", gap: 6, color: "var(--text-3)" }}>
            <span className="mono" style={{ fontSize: 11 }}>{peer.endpoint}</span>
          </div>
        )}

        <div className="device-actions">
          {online ? (
            <button type="button" className="btn btn-tonal btn-small" onClick={onDisconnect}>
              Disconnect
            </button>
          ) : (
            <ManualConnect
              key={peer.fingerprint}
              initialAddress={peer.endpoint}
              isConnecting={isConnecting}
              onConnect={onConnect}
              onReconnect={onReconnect}
            />
          )}
        </div>

        {peers.length > 1 && (
          <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 4 }}>
            <span className="label">Other devices</span>
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
                  {p.displayName}
                </button>
              ))}
            </div>
          </div>
        )}
      </aside>

      <div className="home-main">
        {isLockdown && (
          <section className="section" aria-label="Lockdown active">
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 16,
                padding: "14px 18px",
                borderRadius: 16,
                background: "color-mix(in srgb, var(--error) 12%, transparent)",
                border: "1px solid color-mix(in srgb, var(--error) 30%, transparent)",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 12, minWidth: 0, flex: 1 }}>
                <ShieldAlert size={22} style={{ color: "var(--error)", flexShrink: 0 }} />
                <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                  <span className="list-title" style={{ fontWeight: 600, fontSize: 14 }}>
                    Lockdown mode active
                  </span>
                  <span className="list-sub wrap" style={{ fontSize: 13 }}>
                    All permissions and transfers are paused.
                  </span>
                </div>
              </div>
              <button
                type="button"
                className="btn btn-tonal btn-small"
                onClick={onToggleLockdown}
              >
                Turn off
              </button>
            </div>
          </section>
        )}

        {queuedTransfers.length > 0 && (
          <section className="section" aria-label="Queued transfers">
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 16,
                padding: "14px 18px",
                borderRadius: 16,
                background: "var(--fill)",
                border: "1px solid var(--line)",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 12, minWidth: 0, flex: 1 }}>
                <Clock size={20} style={{ color: "var(--text-2)", flexShrink: 0 }} />
                <div style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
                  <span className="list-title" style={{ fontWeight: 600, fontSize: 14 }}>
                    {queuedTransfers.length === 1
                      ? "1 item queued"
                      : `${queuedTransfers.length} items queued`}
                  </span>
                  <span className="list-sub" style={{ fontSize: 13 }}>
                    Will send automatically when {peer.displayName} connects.
                  </span>
                </div>
              </div>
              <button
                type="button"
                className="btn btn-tonal btn-small"
                onClick={onClearQueuedTransfers}
              >
                Clear
              </button>
            </div>
          </section>
        )}

        {online && handoffs.length > 0 && (
          <section className="section" aria-label="Continue where you left off">
            <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
              {handoffs.map((item) => (
                <HandoffCard
                  key={item.handoffId}
                  item={item}
                  peerName={peer.displayName}
                  onOpen={onOpenHandoff}
                  onDismiss={onDismissHandoff}
                />
              ))}
            </div>
          </section>
        )}

        <section className="section" aria-label="Transfer and quick send">
          <div className="send-actions">
            <button
              type="button"
              className="btn btn-filled btn-large"
              onClick={onChooseFiles}
              title={`${MOD_KEY}O`}
            >
              <Upload size={18} />
              {online ? "Send files" : `Queue files for ${peer.displayName}`}
            </button>
            {online && (
              <button
                type="button"
                className="btn btn-tonal btn-large"
                onClick={onSendClipboard}
                title={`${MOD_SHIFT_KEY}V`}
              >
                <ClipboardPaste size={18} />
                Send clipboard
              </button>
            )}
          </div>

          <form
            className="composer"
            onSubmit={(e) => {
              e.preventDefault();
              onSendText();
            }}
          >
            <input
              value={textInput}
              onChange={(e) => onTextInputChange(e.target.value)}
              placeholder={online ? `Send text or link to ${peer.displayName}...` : `Queue text for ${peer.displayName}...`}
              aria-label="Text to send"
            />
            {online && linkIn(textInput) && (
              <button
                type="button"
                className="btn btn-tonal btn-small"
                onClick={() => {
                  const link = linkIn(textInput);
                  if (link) {
                    onBroadcastHandoff(link);
                    onTextInputChange("");
                  }
                }}
                title="Send as active browser tab on phone"
              >
                Handoff
              </button>
            )}
            <button type="submit" className="composer-send" disabled={!textInput.trim()} aria-label="Send">
              <ArrowUp size={20} />
            </button>
          </form>
        </section>

        {online && (
          <section className="continuity-hub" aria-label="Continuity features">
            <span className="label">Continuity Suite</span>
            <div className="continuity-deck">
              <button
                type="button"
                className="capability-card"
                onClick={onOpenCatalog}
              >
                <div className="capability-icon">
                  <FolderOpen size={18} />
                </div>
                <div>
                  <div className="capability-title">Browse Device</div>
                  <div className="capability-sub">Photos, docs & explorer mount</div>
                </div>
              </button>

              <button
                type="button"
                className="capability-card"
                onClick={onOpenDesktopStream}
              >
                <div className="capability-icon">
                  <Monitor size={18} />
                </div>
                <div>
                  <div className="capability-title">Desktop Mode</div>
                  <div className="capability-sub">Stream apps & DeX workspace</div>
                </div>
              </button>

              <button
                type="button"
                className="capability-card"
                onClick={onOpenClipboardHistory}
              >
                <div className="capability-icon">
                  <History size={18} />
                </div>
                <div>
                  <div className="capability-title">Clipboard History</div>
                  <div className="capability-sub">Search pinned text & links</div>
                </div>
              </button>

              <button
                type="button"
                className="capability-card"
                onClick={onToggleRing}
              >
                <div className="capability-icon" style={isRinging ? { color: "var(--error)" } : undefined}>
                  {isRinging ? <BellOff size={18} /> : <Bell size={18} />}
                </div>
                <div>
                  <div className="capability-title">{isRinging ? "Stop Alarm" : "Find Device"}</div>
                  <div className="capability-sub">{isRinging ? "Alarm is sounding" : "Play sound to locate"}</div>
                </div>
              </button>
            </div>
          </section>
        )}

        {online && notifications.length > 0 && (
          <section className="section">
            <h2 className="title">Notifications</h2>
            <NotificationList notifications={notifications} onError={onError} />
          </section>
        )}

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
    <section className="device">
      <div className="device-head">
        <Smartphone size={32} strokeWidth={1.5} className={`device-icon ${peer.isConnected ? "online" : ""}`} />
        <div className="list-text">
          <h2 className="headline">{peer.displayName}</h2>
          <ConnectionStatus peer={peer} />
        </div>
        <div className="device-actions">
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
            const meta = PERMISSIONS[perm.capabilityId];
            if (!meta) return null;
            return (
              <li key={perm.capabilityId} className="list-item">
                <div className="list-text">
                  <span className="list-title">{meta.label}</span>
                  <span className="list-sub">{meta.description}</span>
                </div>
                <ButtonGroup
                  label={meta.label}
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

      <div>
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
  onOpenClipboardHistory: () => void;
  onError: (message: string) => void;
  onOpenFolder: (folder: string) => void;
  theme: Theme;
  accent: AccentName;
  onThemeChange: (theme: Theme) => void;
  onAccentChange: (accent: AccentName) => void;
  isLockdown: boolean;
  onToggleLockdown: () => void;
  dropFolder: string | null;
  onChooseDropFolder: () => void;
  onClearDropFolder: () => void;
}

const THEME_OPTIONS = [
  { value: "system", label: "Auto" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
] as const;

function SettingsView(props: SettingsViewProps) {
  const {
    identity,
    theme,
    accent,
    onThemeChange,
    onAccentChange,
    clipboardSync,
    onClipboardSyncChange,
    onOpenClipboardHistory,
    onError,
    isLockdown,
    onToggleLockdown,
    dropFolder,
    onChooseDropFolder,
    onClearDropFolder,
  } = props;
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
        <li className="list-item">
          <div className="list-text">
            <span className="list-title">Drop folder</span>
            <span className="list-sub wrap" title={dropFolder ?? undefined}>
              {dropFolder
                ? dropFolder
                : "Files added to this folder are sent automatically to connected devices."}
            </span>
          </div>
          <div className="list-trailing">
            {dropFolder ? (
              <>
                <button
                  type="button"
                  className="icon-btn"
                  title="Open drop folder"
                  onClick={() => onOpenFolder(dropFolder)}
                >
                  <FolderOpen size={18} />
                </button>
                <button type="button" className="btn btn-tonal btn-small" onClick={onChooseDropFolder}>
                  Change
                </button>
                <button type="button" className="btn btn-text btn-small" onClick={onClearDropFolder}>
                  Disable
                </button>
              </>
            ) : (
              <button type="button" className="btn btn-tonal btn-small" onClick={onChooseDropFolder}>
                Choose folder
              </button>
            )}
          </div>
        </li>
      </ul>

      <h2 className="label">Privacy & security</h2>
      <ul className="list">
        <li className="list-item">
          <div className="list-text">
            <span className="list-title" id="lockdown-mode-label">
              Lockdown mode
            </span>
            <span className="list-sub wrap">
              Pause all incoming and outgoing transfers and capability requests immediately.
            </span>
          </div>
          <Switch labelledBy="lockdown-mode-label" checked={isLockdown} onChange={onToggleLockdown} />
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
        <li className="list-item">
          <div className="list-text">
            <span className="list-title">History</span>
            <span className="list-sub">View, search, and pin saved clipboard snippets</span>
          </div>
          <button type="button" className="btn btn-tonal btn-small" onClick={onOpenClipboardHistory}>
            View history
          </button>
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
