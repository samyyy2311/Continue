// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowUp,
  Bell,
  BellRing,
  Check,
  CheckCheck,
  CircleAlert,
  Clipboard,
  Code,
  Copy,
  Pin,
  PinOff,
  FileUp,
  FolderOpen,
  History,
  Home,
  Info,
  Laptop,
  Loader,
  MessageSquareText,
  MonitorSmartphone,
  Plus,
  Search,
  Send,
  Smartphone,
  Trash2,
  Type,
  Upload,
  Video,
} from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import "./App.css";
import { ButtonGroup, ConnectionStatus, getFileIcon, ProgressBar, Switch } from "./components.tsx";
import {
  cancelIncoming,
  clearHistory,
  connectToPeer,
  disconnectPeer,
  reconnectPeer,
  ringPhone,
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
  onNotificationPosted,
  onNotificationRemoved,
  openLink,
  openReceived,
  savePastedFile,
  setAutostart,
  setSaveFolder,
  setClipboardSyncEnabled,
  removeTrustedPeer,
  sendClipboardText,
  sendFileToPeer,
  setAllowed,
  takeFilesToSend,
  onFilesToSend,
  thumbnail,
  getWaiting,
  sendLater,
  cancelWaiting,
  onWaitingSent,
  getPhoneSide,
  setPhoneSide,
  onPointerUnavailable,
  getSnippets,
  pinSnippet,
  unpinSnippet,
  onSnippetsChanged,
  openDropFolder,
  getLockWhenAway,
  setLockWhenAway,
  onRing,
  stopRinging,
} from "./api.ts";
import {
  byDay,
  dayLabel,
  fileNameFromPath,
  formatBytes,
  formatRelativeTime,
  getFileCategory,
  linkIn,
} from "./format.ts";
import { CallBanner } from "./Calls.tsx";
import { Messages } from "./Messages.tsx";
import { Mirror } from "./Mirror.tsx";
import { Webcam } from "./Webcam.tsx";
import { NotificationList } from "./Notifications.tsx";
import { NowPlaying } from "./NowPlaying.tsx";
import { PhoneFiles } from "./PhoneFiles.tsx";
import { PhoneSearch } from "./PhoneSearch.tsx";
import { PairDialog } from "./PairDialog.tsx";
import { Photos } from "./Photos.tsx";
import { TopBar } from "./TopBar.tsx";
import {
  ACCENT_PALETTE,
  type AccentName,
  type Activity,
  type DeviceIdentity,
  type HistoryEntry,
  type HistoryFilter,
  type IncomingTransfer,
  isMac,
  type PeerPermission,
  type PhoneNotification,
  PERMISSIONS,
  type Theme,
  type Toast,
  type TrustedPeer,
  type View,
  type WaitingItem,
  type PhoneSide,
  type Snippet,
} from "./types.ts";

/** Top bar order, which is also the Ctrl/Cmd+number shortcut order. */
const VIEW_ORDER: View[] = ["transfer", "messages", "files", "notifications", "history", "devices", "settings"];

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

function fromWaiting(item: WaitingItem): Activity {
  const file = item.kind === "file";
  return {
    id: `w-${item.id}`,
    kind: item.kind,
    label: file ? fileNameFromPath(item.content) : item.content,
    path: file ? item.content : undefined,
    peerId: item.peerId,
    peerName: item.peerName,
    status: "waiting",
    timestamp: Date.now(),
  };
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

export default function App() {
  const [view, setView] = useState<View>("transfer");
  const [accent, setAccent] = useState<AccentName>(() =>
    readChoice(
      ACCENT_KEY,
      ACCENT_PALETTE.map((a) => a.id),
      "cobalt",
    ),
  );
  const [theme, setTheme] = useState<Theme>(() => readChoice(THEME_KEY, ["system", "light", "dark"], "system"));
  const [identity, setIdentity] = useState<DeviceIdentity | null>(null);
  const [peers, setPeers] = useState<TrustedPeer[] | null>(null);
  const [loadError, setLoadError] = useState("");
  // The last device picked is picked again next time.
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(() => readStored(PEER_KEY));
  const [showPairDialog, setShowPairDialog] = useState(false);
  const [activity, setActivity] = useState<Activity[]>([]);
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const [ringing, setRinging] = useState(false);
  const [connecting, setConnecting] = useState<string | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);
  const [dragCount, setDragCount] = useState<number | null>(null);
  const [textInput, setTextInput] = useState("");
  const [notifications, setNotifications] = useState<PhoneNotification[]>([]);
  const [maximized, setMaximized] = useState(false);

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

  // A disconnected phone can't say when its notifications go away, so they're dropped.
  useEffect(() => {
    const connected = new Set(peers?.filter((p) => p.isConnected).map((p) => p.fingerprint));
    setNotifications((prev) => prev.filter((n) => connected.has(n.peerId)));
  }, [peers]);

  const [clipboardSync, setClipboardSync] = useState(
    () => readChoice(CLIPBOARD_SYNC_KEY, ["on", "off"], "on") === "on",
  );
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
    Promise.all([getDeviceIdentity(), getTrustedPeers(), getHistory(), getWaiting()])
      .then(([loadedIdentity, loadedPeers, history, waiting]) => {
        if (!active) return;
        setIdentity(loadedIdentity);
        setPeers(loadedPeers);
        // Anything sent or coming in since the window opened stays on top.
        setActivity((live) => [...live, ...waiting.map(fromWaiting), ...history.map(fromHistory)]);
      })
      .catch((error) => active && setLoadError(errorMessage(error)));
    return () => {
      active = false;
    };
  }, []);

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

        const unWaiting = await onWaitingSent((id, sent) =>
          setActivity((prev) =>
            prev.map((a) => (a.id === `w-${id}` ? { ...a, status: sent ? "sent" : "failed", timestamp: Date.now() } : a)),
          ),
        );
        keep(unWaiting);
        const unRing = await onRing(setRinging);
        keep(unRing);
        const loadSnippets = () => void getSnippets().then(setSnippets).catch(() => {});
        loadSnippets();
        const unSnippets = await onSnippetsChanged(loadSnippets);
        keep(unSnippets);
        const unPointer = await onPointerUnavailable(() =>
          showError("To use your mouse on the phone, turn on Continue under Accessibility in the phone's settings."),
        );
        keep(unPointer);

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

  const sendWhenConnected = async (peer: TrustedPeer, kind: Activity["kind"], content: string) => {
    try {
      const id = await sendLater(peer.fingerprint, kind === "text", content);
      const item = { id, peerId: peer.fingerprint, peerName: peer.displayName, kind, content };
      setActivity((prev) => [fromWaiting(item), ...prev]);
      showToast(`Sends when ${peer.displayName} connects`);
      return true;
    } catch (error) {
      showError(errorMessage(error));
      return false;
    }
  };

  const sendFile = (peer: TrustedPeer, path: string) => {
    if (!peer.isConnected) return sendWhenConnected(peer, "file", path);
    return trackTransfer(
      { kind: "file", label: fileNameFromPath(path), path, peerId: peer.fingerprint, peerName: peer.displayName },
      (update) => sendFileToPeer(peer.fingerprint, path, (progress) => update(progress)),
    );
  };

  const sendText = (peer: TrustedPeer, text: string) => {
    if (!peer.isConnected) return sendWhenConnected(peer, "text", text);
    return trackTransfer({ kind: "text", label: text, peerId: peer.fingerprint, peerName: peer.displayName }, () =>
      sendClipboardText(peer.fingerprint, text),
    );
  };

  /** The selected device, or says to pair one. Sends to it wait if it isn't connected. */
  const readyPeer = () => {
    if (!selectedPeer) showError("Pair your phone first.");
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
    setActivity((prev) => prev.filter((a) => a.id !== item.id));
    if (item.kind === "file" && item.path) {
      sendFile(peer, item.path);
    } else {
      sendText(peer, item.label);
    }
  };

  useEffect(() => {
    if (!isTauri()) return;
    const win = getCurrentWindow();
    const check = () => void win.isMaximized().then(setMaximized);
    check();
    const unlisten = win.onResized(check);
    return () => void unlisten.then((fn) => fn());
  }, []);

  const sendFilesRef = useRef(sendFiles);
  sendFilesRef.current = sendFiles;

  // Files from Explorer's Send to menu wait until the phone is connected.
  const phoneReady = !!selectedPeer?.isConnected;
  useEffect(() => {
    if (!isTauri() || !phoneReady) return;
    const send = () =>
      void takeFilesToSend().then((paths) => {
        if (paths.length === 0) return;
        setView("transfer");
        sendFilesRef.current(paths);
      });
    send();
    const unlisten = onFilesToSend(send);
    return () => void unlisten.then((stop) => stop());
  }, [phoneReady]);

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

  const handleSendText = async () => {
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
      const sent = await sendText(peer, text.trim());
      if (sent && peer.isConnected) showToast(`Sent what you copied to ${peer.displayName}`);
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
    pin: (text) =>
      pinSnippet(text)
        .then((pinned) => setSnippets((prev) => [pinned, ...prev.filter((s) => s.id !== pinned.id)]))
        .catch((error) => showError(errorMessage(error))),
    unpin: (id) =>
      unpinSnippet(id)
        .then(() => setSnippets((prev) => prev.filter((s) => s.id !== id)))
        .catch((error) => showError(errorMessage(error))),
    cancelWaiting: (item) =>
      cancelWaiting(Number(item.id.slice(2)))
        .then(() => setActivity((prev) => prev.filter((a) => a.id !== item.id)))
        .catch((error) => showError(errorMessage(error))),
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
  const phoneNotifications = notifications.filter((n) => n.peerId === selectedPeer?.fingerprint);
  const tabs = [
    { id: "transfer", label: "Home", icon: <Home size={18} />, badge: activeTransfers.length },
    { id: "messages", label: "Messages", icon: <MessageSquareText size={18} /> },
    { id: "files", label: "Files", icon: <FolderOpen size={18} /> },
    { id: "notifications", label: "Notifications", icon: <Bell size={18} />, badge: phoneNotifications.length },
    { id: "history", label: "Activity", icon: <History size={18} /> },
    { id: "devices", label: "Devices", icon: <Smartphone size={18} /> },
  ] as const;

  return (
    <div className="shell">
      <TopBar tabs={[...tabs]} view={view} onView={setView} maximized={maximized} />
      <CallBanner onError={showError} />

      <main className="pane">
        <div className="pane-scroll" key={view}>
          {view === "transfer" && (
            <HomeView
              peer={selectedPeer}
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
              notificationCount={phoneNotifications.length}
              onOpenNotifications={() => setView("notifications")}
              clipboardSync={clipboardSync}
              onError={showError}
            />
          )}

          {view === "messages" && (
            <div className="page page-wide">
              <header className="page-head">
                <h1 className="display">Messages</h1>
              </header>
              {selectedPeer?.isConnected ? (
                <Messages key={selectedPeer.fingerprint} peer={selectedPeer.fingerprint} onError={showError} />
              ) : (
                <p className="supporting">Connect your phone to read and send its texts here.</p>
              )}
            </div>
          )}

          {view === "files" && (
            <div className="page">
              <header className="page-head">
                <h1 className="display">Files</h1>
              </header>
              {selectedPeer?.isConnected ? (
                <PhoneFiles key={selectedPeer.fingerprint} peer={selectedPeer.fingerprint} onError={showError} />
              ) : (
                <p className="supporting">Connect your phone to browse its files here.</p>
              )}
            </div>
          )}

          {view === "search" && (
            <div className="page">
              <header className="page-head">
                <h1 className="display">Search</h1>
              </header>
              {selectedPeer?.isConnected ? (
                <PhoneSearch key={selectedPeer.fingerprint} peer={selectedPeer.fingerprint} onError={showError} />
              ) : (
                <p className="supporting">Connect your phone to search its files, texts and contacts.</p>
              )}
            </div>
          )}

          {view === "notifications" && (
            <div className="page">
              <header className="page-head">
                <h1 className="display">Notifications</h1>
              </header>
              {phoneNotifications.length > 0 ? (
                <NotificationList notifications={phoneNotifications} onError={showError} />
              ) : (
                <p className="supporting">
                  Notifications from your phone show up here. Turn them on in the phone app, under Settings.
                </p>
              )}
            </div>
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
            <HistoryView activity={activity} snippets={snippets} rowActions={rowActions} onClear={handleClearHistory} />
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
            <Upload size={40} strokeWidth={1.5} className="text-accent" />
            <p className="headline">
              {!selectedPeer
                ? "Pair your phone first"
                : selectedPeer.isConnected
                  ? `Drop to send to ${selectedPeer.displayName}`
                  : `Drop to send when ${selectedPeer.displayName} connects`}
            </p>
            {selectedPeer && dragCount > 0 && (
              <p className="supporting">{dragCount === 1 ? "1 file" : `${dragCount} files`}</p>
            )}
          </div>
        </div>
      )}

      {ringing && (
        <div className="ring-banner" role="alert">
          <BellRing size={22} />
          <span className="title">Your phone is looking for this computer</span>
          <button type="button" className="btn btn-filled" onClick={() => void stopRinging()}>
            Stop
          </button>
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
  cancelWaiting: (item: Activity) => void;
  pin: (text: string) => void;
  unpin: (id: string) => void;
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
  const preview = useThumbnail(openable && getFileCategory(item.label) === "image" ? openable : undefined);
  return (
    <li className={`list-item ${item.status}`}>
      {preview ? (
        <img className="list-thumb" src={preview} alt="" />
      ) : (
        <span className="list-leading">{item.kind === "file" ? getFileIcon(item.label) : <Type size={18} />}</span>
      )}
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
        {item.status === "waiting" && (
          <>
            <span className="status-text">Sends when connected</span>
            <button type="button" className="btn btn-text btn-small" onClick={() => actions.cancelWaiting(item)}>
              Cancel
            </button>
          </>
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
          <>
            <button
              type="button"
              className="icon-btn"
              title="Pin on every device"
              onClick={() => actions.pin(item.label)}
            >
              <Pin size={18} />
            </button>
            <button type="button" className="icon-btn" title="Copy" onClick={() => actions.copy(item.label)}>
              <Copy size={18} />
            </button>
          </>
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
  notificationCount: number;
  onOpenNotifications: () => void;
  clipboardSync: boolean;
  onError: (message: string) => void;
}

function HomeView(props: HomeViewProps) {
  const { peer, onOpenPair, onChooseFiles, onSendClipboard, textInput } = props;
  const { onTextInputChange, onSendText, onConnect, onReconnect, onDisconnect, isConnecting } = props;
  const { activeTransfers, recentActivity, rowActions, onNavigateHistory } = props;
  const { notificationCount, onOpenNotifications, clipboardSync, onError } = props;
  const [writing, setWriting] = useState(false);
  const [mirroring, setMirroring] = useState(false);
  const closeMirror = useCallback(() => setMirroring(false), []);
  const [filming, setFilming] = useState(false);
  const closeWebcam = useCallback(() => setFilming(false), []);

  if (!peer) {
    return (
      <div className="empty">
        <PhoneFrame active={false} />
        <h1 className="display">Pair your phone</h1>
        <p className="supporting">Scan a code once, then send files and text between your phone and this computer.</p>
        <button type="button" className="btn btn-filled btn-large" onClick={onOpenPair}>
          <Plus size={20} />
          Pair a device
        </button>
      </div>
    );
  }

  const online = peer.isConnected;
  const recent = [...activeTransfers, ...recentActivity].slice(0, 5);

  return (
    <div className="home">
      <aside className="device-panel">
        <PhoneFrame active={online} peer={peer} />
        <h1 className="headline">{peer.displayName}</h1>
        <ConnectionStatus peer={peer} />
        {online ? (
          <>
            <RingButton peer={peer.fingerprint} onError={onError} />
            <button type="button" className="btn btn-text btn-small home-disconnect" onClick={onDisconnect}>
              Disconnect
            </button>
          </>
        ) : (
          <div className="device-actions">
            <ManualConnect
              key={peer.fingerprint}
              initialAddress={peer.endpoint}
              isConnecting={isConnecting}
              onConnect={onConnect}
              onReconnect={onReconnect}
            />
          </div>
        )}
      </aside>

      <div className="home-main">
        <div className="tiles">
          <FeatureTile
            icon={<FileUp />}
            label="Send files"
            state={online ? "Or drop them here" : "Sends when it connects"}
            onClick={onChooseFiles}
          />
          <FeatureTile
            icon={<MessageSquareText />}
            label="Send text"
            state={online ? "A note or a link" : "Sends when it connects"}
            onClick={() => setWriting(!writing)}
          />
          <FeatureTile
            icon={<Clipboard />}
            label="Clipboard"
            state={clipboardSync ? "Sync on" : "Sync off"}
            onClick={onSendClipboard}
          />
          <FeatureTile
            icon={<MonitorSmartphone />}
            label="Phone screen"
            state="See and control it"
            disabled={!online}
            onClick={() => setMirroring(true)}
          />
          <FeatureTile
            icon={<Video />}
            label="Webcam"
            state="Use the phone's camera"
            disabled={!online}
            onClick={() => setFilming(true)}
          />
          <FeatureTile
            icon={<Bell />}
            label="Notifications"
            state={notificationCount ? `${notificationCount} new` : "None"}
            onClick={onOpenNotifications}
          />
        </div>

        {writing && (
          <form
            className="composer"
            onSubmit={(e) => {
              e.preventDefault();
              onSendText();
            }}
          >
            <input
              autoFocus
              value={textInput}
              onChange={(e) => onTextInputChange(e.target.value)}
              onKeyDown={(e) => e.key === "Escape" && setWriting(false)}
              placeholder="Type a note or paste a link"
              aria-label="Text to send"
            />
            <button type="submit" className="composer-send" disabled={!textInput.trim()} aria-label="Send">
              <ArrowUp size={20} />
            </button>
          </form>
        )}

        {online && <NowPlaying key={`media-${peer.fingerprint}`} peer={peer.fingerprint} onError={onError} />}
        {online && <Photos key={peer.fingerprint} peer={peer.fingerprint} onError={onError} />}
        {online && mirroring && <Mirror peer={peer.fingerprint} onClose={closeMirror} onError={onError} />}
        {online && filming && <Webcam peer={peer.fingerprint} onClose={closeWebcam} onError={onError} />}

        <section className="section">
          <div className="section-head">
            <h2 className="label">Recent</h2>
            {recent.length > 0 && (
              <button type="button" className="btn btn-text btn-small" onClick={onNavigateHistory}>
                See all
              </button>
            )}
          </div>
          {recent.length > 0 ? (
            <ul className="list">
              {recent.map((item) => (
                <ActivityRow key={item.id} item={item} actions={rowActions} />
              ))}
            </ul>
          ) : (
            <p className="supporting">Files and text you send or receive show up here.</p>
          )}
        </section>
      </div>
    </div>
  );
}

function FeatureTile(props: {
  icon: React.ReactElement;
  label: string;
  state: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  const { icon, label, state, disabled, onClick } = props;
  return (
    <button type="button" className="tile" disabled={disabled} onClick={onClick}>
      <span className="tile-icon">{icon}</span>
      <span className="tile-label">{label}</span>
      <span className="tile-state">{state}</span>
    </button>
  );
}

function PhoneFrame({ active, peer }: { active: boolean; peer?: TrustedPeer }) {
  const screen = peer?.wallpaper
    ? { background: `center / cover url(${peer.wallpaper})` }
    : peer?.wallpaperColor
      ? { background: peer.wallpaperColor }
      : undefined;
  return (
    <div className={`phone ${active ? "active" : ""}`} aria-hidden="true">
      <span className="phone-screen" style={active ? screen : undefined}>
        <span className="phone-camera" />
      </span>
    </div>
  );
}

const thumbnails = new Map<string, Promise<string | null>>();

/** The phone stops by itself after 30 seconds, so the button does too. */
const RING_FOR_MS = 30_000;

function RingButton({ peer, onError }: { peer: string; onError: (message: string) => void }) {
  const [ringing, setRinging] = useState(false);
  useEffect(() => {
    if (!ringing) return;
    const timer = window.setTimeout(() => setRinging(false), RING_FOR_MS);
    return () => window.clearTimeout(timer);
  }, [ringing]);
  const toggle = () => {
    ringPhone(peer, !ringing).then(
      () => setRinging(!ringing),
      (error) => onError(errorMessage(error)),
    );
  };
  return (
    <button type="button" className="btn btn-tonal btn-small" onClick={toggle}>
      <BellRing size={16} />
      {ringing ? "Stop ringing" : "Ring phone"}
    </button>
  );
}

/** Cached per path. */
function useThumbnail(path: string | undefined) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    if (!path || !isTauri()) return;
    if (!thumbnails.has(path))
      thumbnails.set(
        path,
        thumbnail(path).catch(() => null),
      );
    let active = true;
    void thumbnails.get(path)?.then((url) => active && setSrc(url));
    return () => {
      active = false;
    };
  }, [path]);
  return src;
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

  const updateAllowed = async (permission: PeerPermission, allowed: boolean) => {
    try {
      await setAllowed(peer.fingerprint, permission.capabilityId, allowed);
      setPermissions((prev) =>
        prev ? prev.map((p) => (p.capabilityId === permission.capabilityId ? { ...p, allowed } : p)) : null,
      );
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

      <p className="supporting wrap">
        Pairing words: <strong>{peer.words}</strong>. {peer.displayName} shows the same four under this computer
        in its Devices page; if they differ, forget this device and pair again.
      </p>

      <h3 className="label">What {peer.displayName} can do here</h3>
      <ul className="list">
        {permissions ? (
          permissions.map((perm) => {
            const meta = PERMISSIONS[perm.capabilityId];
            if (!meta) return null;
            return (
              <li key={perm.capabilityId} className="list-item">
                <div className="list-text">
                  <span className="list-title" id={`allow-${perm.capabilityId}`}>
                    {meta.label}
                  </span>
                  <span className="list-sub">{meta.description}</span>
                </div>
                <Switch
                  labelledBy={`allow-${perm.capabilityId}`}
                  checked={perm.allowed}
                  onChange={(allowed) => updateAllowed(perm, allowed)}
                />
              </li>
            );
          })
        ) : (
          <li className="list-item">
            <Loader size={18} className="spin" />
            <span className="list-sub">Loading</span>
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
  snippets: Snippet[];
  rowActions: RowActions;
  onClear: () => void;
}

/** Pinned clips, on every paired device. Pin text from its row in Activity. */
function PinnedList(props: { snippets: Snippet[]; search: string; actions: RowActions }) {
  const { snippets, search, actions } = props;
  const shown = snippets.filter((snippet) => !search || snippet.text.toLowerCase().includes(search));
  if (shown.length === 0) {
    return (
      <p className="supporting">
        {search ? "No pinned text matches." : "Pin text from Activity to keep it on every device."}
      </p>
    );
  }
  return (
    <ul className="list">
      {shown.map((snippet) => (
        <li key={snippet.id} className="list-item">
          <span className="list-leading">
            <Pin size={18} />
          </span>
          <div className="list-text">
            <span className="list-title" title={snippet.text}>
              {snippet.text}
            </span>
          </div>
          <div className="list-trailing">
            <button type="button" className="icon-btn" title="Unpin" onClick={() => actions.unpin(snippet.id)}>
              <PinOff size={18} />
            </button>
            <button type="button" className="icon-btn" title="Copy" onClick={() => actions.copy(snippet.text)}>
              <Copy size={18} />
            </button>
          </div>
        </li>
      ))}
    </ul>
  );
}

const HISTORY_FILTERS = [
  { value: "all", label: "All" },
  { value: "file", label: "Files" },
  { value: "text", label: "Text" },
  { value: "failed", label: "Failed" },
  { value: "pinned", label: "Pinned" },
] as const;

function HistoryView(props: HistoryViewProps) {
  const { activity, snippets, rowActions, onClear } = props;
  const [filter, setFilter] = useState<HistoryFilter>("all");
  const [device, setDevice] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [confirmingClear, setConfirmingClear] = useState(false);

  const devices = [...new Map(activity.map((item) => [item.peerId, item.peerName])).entries()];
  const needle = search.trim().toLowerCase();
  const filtered = activity.filter((item) => {
    if (device && item.peerId !== device) return false;
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
        {devices.length > 1 && (
          <div className="chips" role="radiogroup" aria-label="Device">
            {[[null, "Every device"] as const, ...devices].map(([id, name]) => (
              <button
                key={id ?? "all"}
                type="button"
                role="radio"
                aria-checked={device === id}
                className="chip"
                onClick={() => setDevice(id)}
              >
                {device === id && <Check size={16} />}
                {name}
              </button>
            ))}
          </div>
        )}
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

      {filter === "pinned" ? (
        <PinnedList snippets={snippets} search={needle} actions={rowActions} />
      ) : filtered.length === 0 ? (
        <p className="supporting">{search ? `Nothing matches "${search}".` : "Nothing sent or received yet."}</p>
      ) : (
        byDay(filtered, (item) => item.timestamp).map(([day, items]) => (
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

const PHONE_SIDE_OPTIONS = [
  { value: "off", label: "Off" },
  { value: "left", label: "Left" },
  { value: "right", label: "Right" },
] as const;

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
  const [phoneSide, setPhoneSideShown] = useState<PhoneSide | null>(null);
  const [lockWhenAway, setLockWhenAwayShown] = useState<boolean | null>(null);

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => setAppVersion(""));
    getPhoneSide()
      .then(setPhoneSideShown)
      .catch(() => {});
    getLockWhenAway()
      .then(setLockWhenAwayShown)
      .catch(() => {});
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

  const changePhoneSide = (side: PhoneSide) => {
    setPhoneSideShown(side);
    setPhoneSide(side).catch((error) => onError(errorMessage(error)));
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

      {phoneSide && (
        <>
          <h2 className="label">Mouse and keyboard</h2>
          <ul className="list">
            <li className="list-item">
              <div className="list-text">
                <span className="list-title">Phone sits on the</span>
                <span className="list-sub wrap">
                  Push the pointer past that edge of your screens to use it on the phone. Press Esc to bring it back.
                </span>
              </div>
              <ButtonGroup
                label="Phone sits on the"
                options={PHONE_SIDE_OPTIONS}
                value={phoneSide}
                onChange={changePhoneSide}
              />
            </li>
          </ul>
        </>
      )}

      {lockWhenAway !== null && (
        <>
          <h2 className="label">Locking</h2>
          <ul className="list">
            <li className="list-item">
              <div className="list-text">
                <span className="list-title" id="lock-when-away-label">
                  Lock when your phone moves away
                </span>
                <span className="list-sub wrap">
                  Uses Bluetooth. Turn on &quot;Let your computer see you&apos;re nearby&quot; in the phone app&apos;s
                  settings too.
                </span>
              </div>
              <Switch
                labelledBy="lock-when-away-label"
                checked={lockWhenAway}
                onChange={(on) => {
                  setLockWhenAwayShown(on);
                  setLockWhenAway(on).catch((error) => onError(errorMessage(error)));
                }}
              />
            </li>
          </ul>
        </>
      )}

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

      <h2 className="label">Drop folder</h2>
      <ul className="list">
        <li className="list-item">
          <div className="list-text">
            <span className="list-title">Continue Drop, in Documents</span>
            <span className="list-sub wrap">
              Put files here and they go to your phone, or wait until it connects. Sent ones move into Sent.
            </span>
          </div>
          <button
            type="button"
            className="btn btn-tonal btn-small"
            onClick={() => openDropFolder().catch((error) => onError(errorMessage(error)))}
          >
            Open
          </button>
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
