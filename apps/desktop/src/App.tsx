// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState, useCallback, useRef } from "react";
import {
  Share2,
  Smartphone,
  Tablet,
  Laptop,
  Home,
  ArrowDownUp,
  Folder,
  Shield,
  Settings,
  Search,
  X,
  Minus,
  Square,
  Upload,
  Check,
  Copy,
  Plus,
  Play,
  Pause,
  FileText,
  Image,
  Music,
  Film,
  ArrowDownLeft,
  ArrowUpRight,
  QrCode,
  Bell,
  Clipboard,
  ChevronDown,
  Trash2,
} from "lucide-react";
import "./App.css";
import {
  type DeviceIdentity,
  type TrustedPeer,
  type TransferHistoryItem,
  type NotificationItem,
  type RemoteFileItem,
  type AccentName,
  ACCENT_PALETTE,
} from "./types.ts";

async function fetchIdentity(): Promise<DeviceIdentity> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<DeviceIdentity>("get_device_identity");
  } catch {
    return {
      deviceName: "Desktop PC",
      fingerprint: "cont1q8f7e2a9d4c6b8a1e3f5a7b9c1d3e5f7a9b1c3d",
      spkiHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    };
  }
}

async function fetchPeers(): Promise<TrustedPeer[]> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    return await invoke<TrustedPeer[]>("get_trusted_peers");
  } catch {
    return [
      {
        fingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
        displayName: "Pixel 8 Pro",
        pairedAt: 1726920000,
        isConnected: true,
        endpoint: "192.168.1.105:4433",
      },
      {
        fingerprint: "cont1q2f4e6d8c0b2a4f6e8d0c2b4a6f8e0d2c4b6a",
        displayName: "Tablet Air",
        pairedAt: 1726921000,
        isConnected: false,
        endpoint: "192.168.1.112:4433",
      },
    ];
  }
}

type PageDestination = "home" | "transfers" | "files" | "devices" | "settings";
type TransferFilter = "all" | "outgoing" | "incoming";
type FileCategoryFilter = "all" | "images" | "videos" | "documents" | "audio";

function Odometer({ value }: { value: number | string }) {
  const chars = String(value).split("");
  return (
    <span className="odometer-wrap">
      {chars.map((char, i) => {
        const isDigit = /\d/.test(char);
        if (!isDigit) {
          return <span key={i} className="odometer-char">{char}</span>;
        }
        const digit = parseInt(char, 10);
        return (
          <span key={i} className="odometer-col">
            <span
              className="odometer-track"
              style={{ transform: `translateY(-${digit * 10}%)` }}
            >
              {[0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map((n) => (
                <span key={n} className="odometer-num">{n}</span>
              ))}
            </span>
          </span>
        );
      })}
    </span>
  );
}

interface InlineConfirmButtonProps {
  label: string;
  confirmLabel?: string;
  className?: string;
  onConfirm: () => void;
  icon?: React.ReactNode;
}

function InlineConfirmButton({
  label,
  confirmLabel = "Confirm?",
  className = "cat-pill-btn danger cat-pill-btn-xs",
  onConfirm,
  icon,
}: InlineConfirmButtonProps) {
  const [confirming, setConfirming] = useState(false);
  const timerRef = useRef<number | null>(null);

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (confirming) {
      if (timerRef.current) window.clearTimeout(timerRef.current);
      setConfirming(false);
      onConfirm();
    } else {
      setConfirming(true);
      timerRef.current = window.setTimeout(() => {
        setConfirming(false);
      }, 3000);
    }
  };

  useEffect(() => {
    return () => {
      if (timerRef.current) window.clearTimeout(timerRef.current);
    };
  }, []);

  return (
    <button
      type="button"
      className={`${className} ${confirming ? "confirming" : ""}`}
      onClick={handleClick}
      title={confirming ? "Click again to confirm" : label}
    >
      {icon}
      <span>{confirming ? confirmLabel : label}</span>
    </button>
  );
}

interface SegmentedOtpInputProps {
  length?: number;
  value: string;
  onChange: (val: string) => void;
}

function SegmentedOtpInput({ length = 6, value, onChange }: SegmentedOtpInputProps) {
  const inputRefs = useRef<(HTMLInputElement | null)[]>([]);

  const handleKeyDown = (index: number, e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Backspace") {
      e.preventDefault();
      const currentDigits = value.split("");
      if (currentDigits[index]) {
        currentDigits.splice(index, 1);
      } else if (index > 0) {
        currentDigits.splice(index - 1, 1);
        inputRefs.current[index - 1]?.focus();
      }
      onChange(currentDigits.join(""));
    } else if (e.key === "ArrowLeft" && index > 0) {
      inputRefs.current[index - 1]?.focus();
    } else if (e.key === "ArrowRight" && index < length - 1) {
      inputRefs.current[index + 1]?.focus();
    }
  };

  const handleChange = (index: number, e: React.ChangeEvent<HTMLInputElement>) => {
    const char = e.target.value.slice(-1);
    if (!char) return;
    const currentDigits = value.padEnd(length, " ").slice(0, length).split("");
    currentDigits[index] = char;
    const nextVal = currentDigits.join("").trim();
    onChange(nextVal);
    if (index < length - 1 && char) {
      inputRefs.current[index + 1]?.focus();
    }
  };

  const handlePaste = (e: React.ClipboardEvent<HTMLInputElement>) => {
    e.preventDefault();
    const pasted = e.clipboardData.getData("text").trim();
    if (pasted) {
      onChange(pasted.slice(0, length));
      const targetIndex = Math.min(pasted.length, length - 1);
      inputRefs.current[targetIndex]?.focus();
    }
  };

  return (
    <div className="otp-container" onPaste={handlePaste}>
      {Array.from({ length }).map((_, index) => {
        const char = value[index] || "";
        const isFilled = Boolean(char);
        const isCurrent = value.length === index || (index === length - 1 && value.length >= length);
        return (
          <div
            key={index}
            className={`otp-slot ${isFilled ? "filled" : ""} ${isCurrent ? "current" : ""}`}
            onClick={() => inputRefs.current[index]?.focus()}
          >
            <input
              ref={(el) => { inputRefs.current[index] = el; }}
              type="text"
              inputMode="text"
              maxLength={1}
              value={char}
              onChange={(e) => handleChange(index, e)}
              onKeyDown={(e) => handleKeyDown(index, e)}
              className="otp-hidden-input"
            />
            <span className="otp-slot-char">{char}</span>
          </div>
        );
      })}
    </div>
  );
}

export default function App() {
  const [currentPage, setCurrentPage] = useState<PageDestination>("home");
  const [accentName, setAccentName] = useState<AccentName>("recordRed");
  const [searchQuery, setSearchQuery] = useState("");
  const [transferFilter, setTransferFilter] = useState<TransferFilter>("all");
  const [fileCategory, setFileCategory] = useState<FileCategoryFilter>("all");
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);

  const [selectedPeerId, setSelectedPeerId] = useState<string>(
    "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a"
  );
  const [identity, setIdentity] = useState<DeviceIdentity>({
    deviceName: "Desktop PC",
    fingerprint: "cont1q8f7e2a9d4c6b8a1e3f5a7b9c1d3e5f7a9b1c3d",
    spkiHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  });

  const [peers, setPeers] = useState<TrustedPeer[]>([]);
  const [showPairDialog, setShowPairDialog] = useState(false);
  const [showPeerDropdown, setShowPeerDropdown] = useState(false);
  const [pairingPayload, setPairingPayload] = useState("");
  const [isUriMode, setIsUriMode] = useState(false);
  const [copiedFingerprint, setCopiedFingerprint] = useState(false);
  const [toastMessage, setToastMessage] = useState("");
  const [isDraggingOver, setIsDraggingOver] = useState(false);
  const [copiedClipId, setCopiedClipId] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Transfer streaming progress
  const [streamProgress, setStreamProgress] = useState(0);
  const [isStreamPaused, setIsStreamPaused] = useState(false);
  const [showActiveStream, setShowActiveStream] = useState(false);

  // Settings state (CassetteCat style)
  const [allowFileTransfer, setAllowFileTransfer] = useState(true);
  const [allowClipboardSync, setAllowClipboardSync] = useState(true);
  const [allowNotifications, setAllowNotifications] = useState(true);
  const [autoAcceptSmall, setAutoAcceptSmall] = useState(true);
  const [downloadPath, setDownloadPath] = useState("C:\\Users\\ladsa\\Downloads\\Continue");

  // Transfer history
  const [transfers, setTransfers] = useState<TransferHistoryItem[]>([
    {
      id: "tx-1",
      fileName: "presentation_deck_v2.pdf",
      fileSize: 35651584,
      direction: "outgoing",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 120000,
    },
    {
      id: "tx-2",
      fileName: "PXL_20250522_194512_RAW.dng",
      fileSize: 50646220,
      direction: "incoming",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 1080000,
    },
    {
      id: "tx-3",
      fileName: "voice_sync_briefing.m4a",
      fileSize: 8493465,
      direction: "incoming",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 3600000,
    },
    {
      id: "tx-4",
      fileName: "client_assets_archive.zip",
      fileSize: 28490100,
      direction: "outgoing",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 10800000,
    },
  ]);

  // Notifications
  const [notifications, setNotifications] = useState<NotificationItem[]>([
    {
      id: "notif-1",
      appName: "Messages",
      title: "Sarah",
      body: "Sent you the project update document for review.",
      timestamp: Date.now() - 180000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
    {
      id: "notif-2",
      appName: "Calendar",
      title: "Design Sync",
      body: "Starting in 10 minutes in Room 302",
      timestamp: Date.now() - 600000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
    {
      id: "notif-3",
      appName: "GitHub",
      title: "Pull Request #42",
      body: "Merged: feat(transport): optimize QUIC packet acknowledgment",
      timestamp: Date.now() - 1200000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
  ]);

  // Clipboard
  const [clipboardInput, setClipboardInput] = useState("");
  const [syncedClips, setSyncedClips] = useState([
    {
      id: "clip-1",
      text: "https://github.com/samyyy2311/Continue",
      time: "2 minutes ago",
      device: "Pixel 8 Pro",
    },
    {
      id: "clip-2",
      text: "cargo test --workspace --all-targets",
      time: "14 minutes ago",
      device: "Desktop PC",
    },
    {
      id: "clip-3",
      text: "cont1q8f7e2a9d4c6b8a1e3f5a7b9c1d3e5f7a9b1c3d",
      time: "1 hour ago",
      device: "Tablet Air",
    },
  ]);

  // Phone remote files
  const [phoneFiles] = useState<RemoteFileItem[]>([
    {
      id: "rf-1",
      name: "_Data Structures for Data Science.pdf",
      folder: "Documents",
      category: "documents",
      size: 4823400,
      modifiedAt: Date.now() - 86400000,
    },
    {
      id: "rf-2",
      name: "Admit Card - MDS.pdf",
      folder: "Documents",
      category: "documents",
      size: 1204000,
      modifiedAt: Date.now() - 172800000,
    },
    {
      id: "rf-3",
      name: "LINEAR ALGEBRA.pdf",
      folder: "Documents",
      category: "documents",
      size: 8930000,
      modifiedAt: Date.now() - 259200000,
    },
    {
      id: "rf-4",
      name: "Samarth_Lad_Resume.pdf",
      folder: "Documents",
      category: "documents",
      size: 320000,
      modifiedAt: Date.now() - 345600000,
    },
    {
      id: "rf-5",
      name: "217850257839948.pdf",
      folder: "Documents",
      category: "documents",
      size: 1540000,
      modifiedAt: Date.now() - 432000000,
    },
    {
      id: "rf-6",
      name: "Screenshot_20260921-140210.png",
      folder: "DCIM",
      category: "images",
      size: 375808,
      modifiedAt: Date.now() - 7200000,
    },
    {
      id: "rf-7",
      name: "Continue_Backup_2026.zip",
      folder: "Download",
      category: "other",
      size: 51700,
      modifiedAt: Date.now() - 14400000,
    },
    {
      id: "rf-8",
      name: "Continue-v0.4.0.apk",
      folder: "Download",
      category: "documents",
      size: 6658457,
      modifiedAt: Date.now() - 28800000,
    },
  ]);

  // Synchronize CSS variables when accent changes
  useEffect(() => {
    const active = ACCENT_PALETTE.find((a) => a.id === accentName) || ACCENT_PALETTE[0];
    document.documentElement.style.setProperty("--accent-base", active.base);
    document.documentElement.style.setProperty("--accent-hover", active.hover);
  }, [accentName]);

  useEffect(() => {
    fetchIdentity().then(setIdentity);
    fetchPeers().then((loadedPeers) => {
      setPeers(loadedPeers);
      if (loadedPeers.length > 0) {
        setSelectedPeerId(loadedPeers[0].fingerprint);
      }
    });
  }, []);

  useEffect(() => {
    if (isStreamPaused || !showActiveStream) return;
    const timer = setInterval(() => {
      setStreamProgress((prev) => (prev >= 100 ? 64.2 : +(prev + 0.4).toFixed(1)));
    }, 1000);
    return () => clearInterval(timer);
  }, [isStreamPaused, showActiveStream]);

  const showToast = useCallback((msg: string) => {
    setToastMessage(msg);
    setTimeout(() => setToastMessage(""), 2600);
  }, []);

  const handleCopyFingerprint = () => {
    navigator.clipboard.writeText(identity.fingerprint);
    setCopiedFingerprint(true);
    showToast("Identity fingerprint copied");
    setTimeout(() => setCopiedFingerprint(false), 2000);
  };

  const handleCopyText = (text: string, id: string) => {
    navigator.clipboard.writeText(text);
    setCopiedClipId(id);
    showToast("Copied to clipboard");
    setTimeout(() => setCopiedClipId(null), 1800);
  };

  const handleDisconnectPeer = async (fingerprint: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("remove_trusted_peer", { fingerprint });
    } catch {
      // Fallback
    }
    setPeers((prev) => {
      const remaining = prev.filter((p) => p.fingerprint !== fingerprint);
      if (selectedPeerId === fingerprint && remaining.length > 0) {
        setSelectedPeerId(remaining[0].fingerprint);
      }
      return remaining;
    });
    showToast("Device disconnected");
  };

  const handlePairSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!pairingPayload.trim()) return;

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const peer = await invoke<TrustedPeer>("pair_from_qr", {
        qrPayload: pairingPayload.trim(),
      });
      setPeers((prev) => [...prev, peer]);
      setSelectedPeerId(peer.fingerprint);
      showToast(`Connected to ${peer.displayName}`);
    } catch {
      const dummy: TrustedPeer = {
        fingerprint: "cont1q" + Math.random().toString(36).substring(2, 14),
        displayName: "New Mobile Device",
        pairedAt: Math.floor(Date.now() / 1000),
        isConnected: true,
        endpoint: "192.168.1.115:4433",
      };
      setPeers((prev) => [...prev, dummy]);
      setSelectedPeerId(dummy.fingerprint);
      showToast("Connected via local network");
    }

    setPairingPayload("");
    setShowPairDialog(false);
  };

  const handleSendClipboard = async () => {
    if (!clipboardInput.trim()) return;
    const currentPeer = peers.find((p) => p.fingerprint === selectedPeerId) || peers[0];
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      if (currentPeer) {
        await invoke("send_clipboard_text", {
          peerFingerprint: currentPeer.fingerprint,
          text: clipboardInput,
        });
      }
    } catch {
      // Fallback
    }
    const newClip = {
      id: "clip-" + Date.now(),
      text: clipboardInput,
      time: "Just now",
      device: "Desktop PC",
    };
    setSyncedClips((prev) => [newClip, ...prev]);
    showToast(`Clip sent to ${currentPeer?.displayName || "device"}`);
    setClipboardInput("");
  };

  const handleSendFilesPicked = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;
    const currentPeer = peers.find((p) => p.fingerprint === selectedPeerId);
    const peerName = currentPeer?.displayName || "Pixel 8 Pro";

    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const newTx: TransferHistoryItem = {
        id: "tx-" + Date.now() + "-" + i,
        fileName: file.name,
        fileSize: file.size,
        direction: "outgoing",
        peerFingerprint: currentPeer?.fingerprint || "cont1q9a8b",
        status: "completed",
        timestamp: Date.now(),
      };
      setTransfers((prev) => [newTx, ...prev]);
    }
    setShowActiveStream(true);
    setStreamProgress(15);
    showToast(`Streaming ${files[0].name} to ${peerName}`);
  };

  const handleTriggerSendFile = () => {
    if (fileInputRef.current) {
      fileInputRef.current.click();
    }
  };

  const handleDismissNotification = (id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
    showToast("Notification dismissed");
  };

  const handleClearAllNotifications = () => {
    setNotifications([]);
    showToast("All notifications cleared");
  };

  const handleDeleteClip = (id: string) => {
    setSyncedClips((prev) => prev.filter((c) => c.id !== id));
    showToast("Clip removed");
  };

  const getDeviceIcon = (name: string) => {
    const lower = name.toLowerCase();
    if (lower.includes("tablet") || lower.includes("ipad")) {
      return <Tablet size={16} />;
    }
    if (
      lower.includes("pc") ||
      lower.includes("desktop") ||
      lower.includes("mac") ||
      lower.includes("laptop")
    ) {
      return <Laptop size={16} />;
    }
    return <Smartphone size={16} />;
  };

  const handleMinimizeWindow = async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().minimize();
    } catch {
      // Fallback
    }
  };

  const handleMaximizeWindow = async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().toggleMaximize();
    } catch {
      // Fallback
    }
  };

  const handleCloseWindow = async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch {
      // Fallback
    }
  };

  const currentPeer = peers.find((p) => p.fingerprint === selectedPeerId) || peers[0] || {
    fingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    displayName: "Pixel 8 Pro",
    pairedAt: 1726920000,
    isConnected: true,
    endpoint: "192.168.1.105:4433",
  };

  const filteredTransfers = transfers.filter((tx) => {
    const matchesFilter =
      transferFilter === "all" ||
      (transferFilter === "outgoing" && tx.direction === "outgoing") ||
      (transferFilter === "incoming" && tx.direction === "incoming");
    const matchesSearch =
      !searchQuery.trim() ||
      tx.fileName.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesFilter && matchesSearch;
  });

  const filteredPhoneFiles = phoneFiles.filter((file) => {
    const matchesCategory =
      fileCategory === "all" || file.category === fileCategory;
    const matchesFolder =
      !selectedFolder || file.folder === selectedFolder;
    const matchesSearch =
      !searchQuery.trim() ||
      file.name.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesFolder && matchesSearch;
  });

  const filteredClips = syncedClips.filter((clip) =>
    !searchQuery.trim() || clip.text.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  };

  const getFileIcon = (fileName: string) => {
    const lower = fileName.toLowerCase();
    if (lower.endsWith(".pdf")) {
      return <FileText size={18} className="file-icon-pdf" />;
    }
    if (lower.endsWith(".doc") || lower.endsWith(".docx") || lower.endsWith(".txt")) {
      return <FileText size={18} className="file-icon-doc" />;
    }
    if (
      lower.endsWith(".png") ||
      lower.endsWith(".jpg") ||
      lower.endsWith(".jpeg") ||
      lower.endsWith(".dng")
    ) {
      return <Image size={18} className="file-icon-img" />;
    }
    if (lower.endsWith(".m4a") || lower.endsWith(".mp3") || lower.endsWith(".wav")) {
      return <Music size={18} className="file-icon-audio" />;
    }
    if (lower.endsWith(".mp4") || lower.endsWith(".mov") || lower.endsWith(".mkv")) {
      return <Film size={18} className="file-icon-video" />;
    }
    return <FileText size={18} className="file-icon-generic" />;
  };

  return (
    <div className="continue-app">
      <input
        type="file"
        ref={fileInputRef}
        onChange={handleSendFilesPicked}
        style={{ display: "none" }}
        multiple
      />

      {/* Unified Frameless Topbar (Height 50px) */}
      <header className="app-topbar" data-tauri-drag-region>
        {/* Left: Brand Logo & Title */}
        <div
          className="topbar-brand"
          onClick={() => setCurrentPage("home")}
          title="Continue - Device Continuity"
        >
          <div className="brand-logo-mark">
            <Share2 size={15} strokeWidth={2.4} />
          </div>
          <span className="brand-title-label">Continue</span>
        </div>

        {/* Center: Horizontal Navigation Tabs */}
        <nav className="topbar-nav-tabs">
          <button
            className={`nav-tab-btn ${currentPage === "home" ? "active" : ""}`}
            onClick={() => setCurrentPage("home")}
            title="Home"
          >
            <Home size={15} className="tab-icon" />
            <span>Home</span>
          </button>
          <button
            className={`nav-tab-btn ${currentPage === "transfers" ? "active" : ""}`}
            onClick={() => setCurrentPage("transfers")}
            title="Transfers"
          >
            <ArrowDownUp size={15} className="tab-icon" />
            <span>Transfers</span>
          </button>
          <button
            className={`nav-tab-btn ${currentPage === "files" ? "active" : ""}`}
            onClick={() => setCurrentPage("files")}
            title="Files"
          >
            <Folder size={15} className="tab-icon" />
            <span>Files</span>
          </button>
          <button
            className={`nav-tab-btn ${currentPage === "devices" ? "active" : ""}`}
            onClick={() => setCurrentPage("devices")}
            title="Devices"
          >
            <Shield size={15} className="tab-icon" />
            <span>Devices</span>
          </button>
          <button
            className={`nav-tab-btn ${currentPage === "settings" ? "active" : ""}`}
            onClick={() => setCurrentPage("settings")}
            title="Settings"
          >
            <Settings size={15} className="tab-icon" />
            <span>Settings</span>
          </button>
        </nav>

        {/* Right Area: Search + Connected Device Pill + Window Controls */}
        <div className="topbar-right-area">
          <div className={`modern-search-bar topbar-search ${searchQuery ? "has-value" : ""}`}>
            <Search size={13} className="search-icon-trigger" />
            <input
              type="text"
              className="search-input-field"
              placeholder="Search..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
            {searchQuery && (
              <button
                className="search-clear-action"
                onClick={() => setSearchQuery("")}
                title="Clear"
              >
                <X size={12} />
              </button>
            )}
          </div>

          <div
            className="header-peer-pill"
            onClick={() => setCurrentPage("devices")}
            title={`Connected Device: ${currentPeer.displayName}`}
          >
            <span className="peer-live-indicator" />
            <Smartphone size={13} />
            <span className="peer-pill-name">{currentPeer.displayName}</span>
          </div>

          <div className="window-action-buttons">
            <button
              className="win-action-btn"
              onClick={handleMinimizeWindow}
              title="Minimize"
            >
              <Minus size={13} />
            </button>
            <button
              className="win-action-btn"
              onClick={handleMaximizeWindow}
              title="Maximize"
            >
              <Square size={10} />
            </button>
            <button
              className="win-action-btn win-close"
              onClick={handleCloseWindow}
              title="Close"
            >
              <X size={13} />
            </button>
          </div>
        </div>
      </header>

      {/* Main Full-Width Content Stage */}
      <main className="content-stage-scroll">
          {/* VIEW 1: HOME PAGE */}
          {currentPage === "home" && (
            <div className="home-studio-layout">
              {/* Refined Page Header & Device Status */}
              <div className="home-hero-header">
                <div className="home-hero-titles">
                  <h1 className="home-hero-title">Connected to {currentPeer.displayName}</h1>
                  <p className="home-hero-subtitle">
                    Send files, copy text across devices, and view phone alerts over your local network.
                  </p>
                </div>
                <div className="home-device-selector">
                  <button
                    className="cat-pill-btn"
                    onClick={() => setShowPeerDropdown((p) => !p)}
                  >
                    <span>Switch device</span>
                    <ChevronDown size={11} />
                  </button>
                  {showPeerDropdown && (
                    <>
                      <div
                        className="dropdown-backdrop"
                        onClick={() => setShowPeerDropdown(false)}
                      />
                      <div className="peer-select-menu">
                        {peers.map((peer) => (
                          <button
                            key={peer.fingerprint}
                            className={`peer-select-item ${
                              peer.fingerprint === selectedPeerId ? "active" : ""
                            }`}
                            onClick={() => {
                              setSelectedPeerId(peer.fingerprint);
                              setShowPeerDropdown(false);
                              showToast(`Switched to ${peer.displayName}`);
                            }}
                          >
                            <span className="peer-select-label">
                              <span className="peer-select-icon">{getDeviceIcon(peer.displayName)}</span>
                              <span>{peer.displayName}</span>
                            </span>
                            {peer.isConnected && <span className="peer-online-dot" />}
                          </button>
                        ))}
                        <div className="peer-menu-divider" />
                        <button
                          className="peer-select-item action"
                          onClick={() => {
                            setShowPeerDropdown(false);
                            setShowPairDialog(true);
                          }}
                        >
                          <Plus size={12} />
                          <span>Pair new device</span>
                        </button>
                      </div>
                    </>
                  )}
                </div>
              </div>

              {/* Frameless Hero Dropzone */}
              <div
                className={`frameless-dropzone ${isDraggingOver ? "dragging" : ""}`}
                onDragOver={(e) => {
                  e.preventDefault();
                  setIsDraggingOver(true);
                }}
                onDragLeave={() => setIsDraggingOver(false)}
                onDrop={(e) => {
                  e.preventDefault();
                  setIsDraggingOver(false);
                  if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
                    handleSendFilesPicked({
                      target: { files: e.dataTransfer.files },
                    } as unknown as React.ChangeEvent<HTMLInputElement>);
                  } else {
                    handleTriggerSendFile();
                  }
                }}
                onMouseMove={(e) => {
                  const rect = e.currentTarget.getBoundingClientRect();
                  e.currentTarget.style.setProperty("--spot-x", `${e.clientX - rect.left}px`);
                  e.currentTarget.style.setProperty("--spot-y", `${e.clientY - rect.top}px`);
                }}
                onClick={handleTriggerSendFile}
              >
                <div className="dropzone-icon-well">
                  <Upload size={20} />
                </div>
                <div className="dropzone-label">
                  <span className="dropzone-title">
                    Drop files to send to {currentPeer.displayName}
                  </span>
                  <span className="dropzone-hint">
                    Fast Wi-Fi transfer for files of any size
                  </span>
                </div>
                <div className="dropzone-actions-row">
                  <button
                    type="button"
                    className="cat-btn cat-btn-primary"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleTriggerSendFile();
                    }}
                  >
                    <Upload size={13} />
                    <span>Choose Files</span>
                  </button>
                  <button
                    type="button"
                    className="cat-btn cat-btn-secondary"
                    onClick={(e) => {
                      e.stopPropagation();
                      setCurrentPage("files");
                    }}
                  >
                    <Folder size={13} />
                    <span>Browse Phone Files</span>
                  </button>
                </div>
              </div>

              {/* Active Transfer Stream (only when a file is actually being transferred) */}
              {showActiveStream && (
                <div className="active-transfer-strip">
                  <div className="transfer-meta-row">
                    <div className="transfer-file-title">
                      <FileText size={14} className="transfer-file-icon" />
                      <span className="file-name">recording_session_2025_4k.mov</span>
                    </div>
                    <div className="transfer-controls">
                      <span className="transfer-speed">
                        <Odometer value={114} /> MB/s
                      </span>
                      <span className="transfer-pct">
                        <Odometer value={streamProgress} />%
                      </span>
                      <button
                        className="ghost-icon-btn"
                        onClick={() => setIsStreamPaused((p) => !p)}
                        title={isStreamPaused ? "Resume" : "Pause"}
                      >
                        {isStreamPaused ? <Play size={11} /> : <Pause size={11} />}
                      </button>
                      <button
                        className="ghost-icon-btn"
                        onClick={() => setShowActiveStream(false)}
                        title="Cancel"
                      >
                        <X size={11} />
                      </button>
                    </div>
                  </div>
                  <div className="transfer-bar">
                    <div
                      className="transfer-bar-fill"
                      style={{ width: `${streamProgress}%` }}
                    />
                  </div>
                </div>
              )}

              {/* Activity Section: Balanced Two-Card Dock */}
              <div className="home-activity-grid">
                {/* Left Card: Clipboard */}
                <div className="activity-card">
                  <div className="activity-card-header">
                    <div className="stream-title-wrap">
                      <Clipboard size={14} className="stream-icon" />
                      <span className="stream-title">Clipboard</span>
                      <span className="stream-count-pill">
                        <Odometer value={filteredClips.length} />
                      </span>
                    </div>
                  </div>

                  <form
                    onSubmit={(e) => {
                      e.preventDefault();
                      handleSendClipboard();
                    }}
                    className="stream-input-wrap"
                  >
                    <input
                      type="text"
                      className="stream-pill-input"
                      placeholder="Type or paste to send to phone..."
                      value={clipboardInput}
                      onChange={(e) => setClipboardInput(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          e.preventDefault();
                          handleSendClipboard();
                        }
                      }}
                    />
                    <button
                      type="submit"
                      className="stream-pill-submit"
                      title="Send to phone (Enter)"
                    >
                      <ArrowUpRight size={13} />
                    </button>
                  </form>

                  {filteredClips.length === 0 ? (
                    <div className="activity-empty-state">
                      <Clipboard size={22} className="activity-empty-icon" />
                      <span className="activity-empty-title">No copied items yet</span>
                      <span className="activity-empty-desc">
                        Text or links you copy on your phone or paste above will sync here.
                      </span>
                    </div>
                  ) : (
                    <div className="stream-feed-list">
                      {filteredClips.map((clip) => {
                        const isUrl =
                          clip.text.startsWith("http://") ||
                          clip.text.startsWith("https://");
                        return (
                          <div key={clip.id} className="stream-row clip-row">
                            <div className="stream-row-content">
                              <span className={`stream-text ${isUrl ? "url" : ""}`}>
                                {clip.text}
                              </span>
                              <span className="stream-sub">
                                {clip.device}, {clip.time}
                              </span>
                            </div>
                            <div className="stream-row-actions">
                              <button
                                className={`row-copy-btn ${
                                  copiedClipId === clip.id ? "copied" : ""
                                }`}
                                onClick={() => handleCopyText(clip.text, clip.id)}
                                title="Copy"
                              >
                                {copiedClipId === clip.id ? (
                                  <Check size={11} />
                                ) : (
                                  <Copy size={11} />
                                )}
                              </button>
                              <button
                                className="row-del-btn"
                                onClick={() => handleDeleteClip(clip.id)}
                                title="Delete"
                              >
                                <Trash2 size={11} />
                              </button>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>

                {/* Right Card: Phone Notifications */}
                <div className="activity-card">
                  <div className="activity-card-header">
                    <div className="stream-title-wrap">
                      <Bell
                        size={14}
                        className={`stream-icon ${
                          notifications.length > 0 ? "bell-spring-alert" : ""
                        }`}
                      />
                      <span className="stream-title">Notifications</span>
                      {notifications.length > 0 && (
                        <span className="stream-count-pill">
                          <Odometer value={notifications.length} />
                        </span>
                      )}
                    </div>
                    {notifications.length > 0 && (
                      <InlineConfirmButton
                        label="Clear"
                        confirmLabel="Clear all?"
                        onConfirm={handleClearAllNotifications}
                      />
                    )}
                  </div>

                  {notifications.length === 0 ? (
                    <div className="activity-empty-state">
                      <Bell size={22} className="activity-empty-icon" />
                      <span className="activity-empty-title">All caught up</span>
                      <span className="activity-empty-desc">
                        Incoming alerts from your phone will show up here in real time.
                      </span>
                    </div>
                  ) : (
                    <div className="stream-feed-list">
                      {notifications.map((notif) => (
                        <div key={notif.id} className="stream-row notif-row">
                          <div className="stream-row-content">
                            <div className="notif-meta">
                              <span className="notif-app">{notif.appName}</span>
                              <span className="notif-time">Just now</span>
                            </div>
                            <div className="notif-title">{notif.title}</div>
                            <div className="notif-body">{notif.body}</div>
                          </div>
                          <button
                            className="row-del-btn"
                            onClick={() => handleDismissNotification(notif.id)}
                            title="Dismiss"
                          >
                            <X size={11} />
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* VIEW 2: TRANSFERS PAGE */}
          {currentPage === "transfers" && (
            <div className="page-content-stack">
              <div className="cat-section-header">
                <div className="header-titles">
                  <h3 className="section-title">Transfer History</h3>
                  <span className="section-subtitle">
                    Files sent and received with your connected devices
                  </span>
                </div>
                <div className="segmented-filter-row">
                  <button
                    className={`seg-btn ${transferFilter === "all" ? "active" : ""}`}
                    onClick={() => setTransferFilter("all")}
                  >
                    All
                  </button>
                  <button
                    className={`seg-btn ${transferFilter === "outgoing" ? "active" : ""}`}
                    onClick={() => setTransferFilter("outgoing")}
                  >
                    Sent
                  </button>
                  <button
                    className={`seg-btn ${transferFilter === "incoming" ? "active" : ""}`}
                    onClick={() => setTransferFilter("incoming")}
                  >
                    Received
                  </button>
                </div>
              </div>

              <div className="setting-card">
                {filteredTransfers.length === 0 ? (
                  <div className="empty-state-card">
                    <ArrowDownUp size={28} className="empty-icon" />
                    <span>No transfers found</span>
                  </div>
                ) : (
                  <div className="transfers-table-stack">
                    {filteredTransfers.map((tx) => (
                      <div key={tx.id} className="transfer-row-item">
                        <div className="transfer-item-left">
                          <div className="transfer-icon-box">
                            {getFileIcon(tx.fileName)}
                          </div>
                          <div className="transfer-info-column">
                            <span className="transfer-filename">{tx.fileName}</span>
                            <span className="transfer-meta-mono">
                              {formatBytes(tx.fileSize)},{" "}
                              {tx.direction === "outgoing"
                                ? `Sent to ${currentPeer.displayName}`
                                : `Received from ${currentPeer.displayName}`}
                            </span>
                          </div>
                        </div>

                        <div className="transfer-item-right">
                          <span className={`direction-pill ${tx.direction}`}>
                            {tx.direction === "outgoing" ? (
                              <ArrowUpRight size={11} />
                            ) : (
                              <ArrowDownLeft size={11} />
                            )}
                            {tx.direction === "outgoing" ? "Sent" : "Received"}
                          </span>
                          <button
                            className="cat-btn cat-btn-sm cat-btn-outline"
                            onClick={() => showToast(`Opened location for ${tx.fileName}`)}
                          >
                            Open
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* VIEW 3: FILES PAGE (FolderCard & Document Grid) */}
          {currentPage === "files" && (
            <div className="page-content-stack">
              {/* Phone Storage Folders */}
              <div className="cat-section-header">
                <div className="header-titles">
                  <h3 className="section-title">Phone Storage</h3>
                  <span className="section-subtitle">
                    Browse folders and files on {currentPeer.displayName}
                  </span>
                </div>
                {selectedFolder && (
                  <button
                    className="cat-link-btn"
                    onClick={() => setSelectedFolder(null)}
                  >
                    Clear filter ({selectedFolder})
                  </button>
                )}
              </div>

              {/* Folder Cards Grid (CassetteCat FolderCard Style) */}
              <div className="folder-cards-grid">
                {[
                  { name: "Documents", count: 5, category: "documents" },
                  { name: "DCIM", count: 1, category: "images" },
                  { name: "Download", count: 2, category: "other" },
                  { name: "Movies", count: 0, category: "videos" },
                  { name: "Music", count: 0, category: "audio" },
                ].map((f) => (
                  <div
                    key={f.name}
                    className={`cat-folder-card ${selectedFolder === f.name ? "active" : ""}`}
                    onClick={() =>
                      setSelectedFolder((prev) => (prev === f.name ? null : f.name))
                    }
                  >
                    <div className="folder-card-top">
                      <div className="folder-interactive-stage">
                        <div className="folder-papers-sheaf">
                          <div className="sheaf-paper paper-left" />
                          <div className="sheaf-paper paper-right" />
                        </div>
                        <Folder size={22} className="folder-front-icon" />
                      </div>
                      <span className="folder-items-pill">{f.count} files</span>
                    </div>
                    <div className="folder-card-bottom">
                      <span className="folder-card-name">{f.name}</span>
                      <span className="folder-card-desc">Internal Storage</span>
                    </div>
                  </div>
                ))}
              </div>

              {/* Documents & Files Grid */}
              <div className="cat-section-header" style={{ marginTop: "12px" }}>
                <div className="header-titles">
                  <h3 className="section-title">Synced Files</h3>
                  <span className="section-subtitle">
                    Files ready to download or open on this computer
                  </span>
                </div>
                <div className="segmented-filter-row">
                  {[
                    { key: "all", label: "All" },
                    { key: "documents", label: "Documents" },
                    { key: "images", label: "Images" },
                    { key: "audio", label: "Audio" },
                  ].map((cat) => (
                    <button
                      key={cat.key}
                      className={`seg-btn ${fileCategory === cat.key ? "active" : ""}`}
                      onClick={() => setFileCategory(cat.key as FileCategoryFilter)}
                    >
                      {cat.label}
                    </button>
                  ))}
                </div>
              </div>

              <div className="setting-card">
                <div className="files-shelf-grid">
                  {filteredPhoneFiles.map((doc) => (
                    <div
                      key={doc.id}
                      className="file-shelf-tile"
                      onClick={() => showToast(`Selected ${doc.name}`)}
                    >
                      <div className="shelf-tile-icon">
                        {doc.name.endsWith(".pdf") ? (
                          <FileText size={30} className="file-icon-pdf" />
                        ) : doc.category === "images" ? (
                          <Image size={30} className="file-icon-img" />
                        ) : (
                          <FileText size={30} className="file-icon-generic" />
                        )}
                      </div>
                      <span className="shelf-tile-title" title={doc.name}>
                        {doc.name}
                      </span>
                      <span className="shelf-tile-size-mono">{formatBytes(doc.size)}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* VIEW 4: DEVICES PAGE (Continuity Map & Trusted Peers) */}
          {currentPage === "devices" && (
            <div className="page-content-stack">
              <div className="cat-section-header">
                <div className="header-titles">
                  <h3 className="section-title">Connected Devices</h3>
                  <span className="section-subtitle">
                    Devices currently linked and trusted on your local network
                  </span>
                </div>
                <button
                  className="cat-btn cat-btn-primary"
                  onClick={() => setShowPairDialog(true)}
                >
                  <Plus size={14} />
                  <span>Pair New Device</span>
                </button>
              </div>

              {/* Hardware Continuity Stage Canvas */}
              <div className="setting-card continuity-canvas-card">
                <div className="continuity-canvas">
                  <div className="device-node host">
                    <Laptop size={44} className="device-node-icon" />
                    <span className="device-node-name">{identity.deviceName}</span>
                    <span className="device-node-role">This Computer</span>
                  </div>

                  <div className="device-link-channel">
                    <div className="channel-line" />
                    <div className="channel-badge">
                      <Shield size={13} />
                      <span>Encrypted Local Link</span>
                    </div>
                  </div>

                  <div className="device-node peer">
                    <Smartphone size={44} className="device-node-icon" />
                    <span className="device-node-name">{currentPeer.displayName}</span>
                    <span className="device-node-role online">Connected</span>
                  </div>
                </div>
              </div>

              {/* Trusted Peers List */}
              <div className="cat-section-header" style={{ marginTop: "12px" }}>
                <div className="header-titles">
                  <h3 className="section-title">Trusted Devices</h3>
                  <span className="section-subtitle">
                    Devices you have verified and allowed to connect
                  </span>
                </div>
              </div>

              <div className="setting-card">
                <div className="peers-list-stack">
                  {peers.map((peer) => (
                    <div key={peer.fingerprint} className="peer-record-row">
                      <div className="peer-record-left">
                        <div className="peer-avatar-circle">
                          {peer.displayName.toLowerCase().includes("tablet") ? (
                            <Tablet size={20} />
                          ) : (
                            <Smartphone size={20} />
                          )}
                        </div>
                        <div className="peer-record-meta">
                          <div className="peer-name-line">
                            <span className="peer-title">{peer.displayName}</span>
                            <span
                              className={`peer-status-tag ${
                                peer.isConnected ? "online" : "offline"
                              }`}
                            >
                              {peer.isConnected ? "Connected" : "Offline"}
                            </span>
                          </div>
                          <span className="peer-fingerprint-mono">
                            Local network, key {peer.fingerprint.substring(0, 16)}...
                          </span>
                        </div>
                      </div>

                      <InlineConfirmButton
                        label="Disconnect"
                        confirmLabel="Disconnect?"
                        className="cat-btn cat-btn-sm cat-btn-outline danger"
                        onConfirm={() => handleDisconnectPeer(peer.fingerprint)}
                      />
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* VIEW 5: SETTINGS PAGE (CassetteCat Settings Style) */}
          {currentPage === "settings" && (
            <div className="page-content-stack">
              {/* Appearance & Accent Color Section */}
              <div className="cat-section-header">
                <div className="header-titles">
                  <h3 className="section-title">Appearance</h3>
                  <span className="section-subtitle">
                    Choose an accent color for active items and highlights
                  </span>
                </div>
              </div>

              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-text">
                    <span className="setting-title">Accent Color</span>
                    <span className="setting-subtitle">
                      Used for buttons, badges, and active tabs
                    </span>
                  </div>
                </div>

                <div className="accent-swatches-flow">
                  {ACCENT_PALETTE.map((acc) => (
                    <div
                      key={acc.id}
                      className={`accent-swatch-item ${
                        accentName === acc.id ? "selected" : ""
                      }`}
                      onClick={() => setAccentName(acc.id)}
                    >
                      <div
                        className="swatch-circle"
                        style={{ backgroundColor: acc.base }}
                      />
                      <span className="swatch-label">{acc.label}</span>
                    </div>
                  ))}
                </div>
              </div>

              {/* General Preferences */}
              <div className="cat-section-header" style={{ marginTop: "12px" }}>
                <div className="header-titles">
                  <h3 className="section-title">General</h3>
                  <span className="section-subtitle">
                    Device name, download folder, and transfer behavior
                  </span>
                </div>
              </div>

              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-text">
                    <span className="setting-title">Computer Name</span>
                    <span className="setting-subtitle">
                      Visible to your other devices on the same Wi-Fi network
                    </span>
                  </div>
                  <input
                    type="text"
                    className="cat-text-input name-field"
                    value={identity.deviceName}
                    onChange={(e) =>
                      setIdentity((prev) => ({ ...prev, deviceName: e.target.value }))
                    }
                  />
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-text">
                    <span className="setting-title">Downloads Folder</span>
                    <span className="setting-subtitle">
                      Where received files are saved
                    </span>
                  </div>
                  <input
                    type="text"
                    className="cat-text-input path-field"
                    value={downloadPath}
                    onChange={(e) => setDownloadPath(e.target.value)}
                  />
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-text">
                    <span className="setting-title">Automatically Accept Small Files</span>
                    <span className="setting-subtitle">
                      Save files under 50 MB immediately without asking each time
                    </span>
                  </div>
                  <label className="cat-switch">
                    <input
                      type="checkbox"
                      checked={autoAcceptSmall}
                      onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                    />
                    <span className="cat-slider" />
                  </label>
                </div>
              </div>

              {/* Continuity Features */}
              <div className="cat-section-header" style={{ marginTop: "12px" }}>
                <div className="header-titles">
                  <h3 className="section-title">Features & Syncing</h3>
                  <span className="section-subtitle">
                    Turn specific sharing features on or off
                  </span>
                </div>
              </div>

              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-text">
                    <span className="setting-title">File Sharing</span>
                    <span className="setting-subtitle">
                      Send and receive files with your connected devices
                    </span>
                  </div>
                  <label className="cat-switch">
                    <input
                      type="checkbox"
                      checked={allowFileTransfer}
                      onChange={(e) => setAllowFileTransfer(e.target.checked)}
                    />
                    <span className="cat-slider" />
                  </label>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-text">
                    <span className="setting-title">Clipboard Sync</span>
                    <span className="setting-subtitle">
                      Automatically copy and paste text between your devices
                    </span>
                  </div>
                  <label className="cat-switch">
                    <input
                      type="checkbox"
                      checked={allowClipboardSync}
                      onChange={(e) => setAllowClipboardSync(e.target.checked)}
                    />
                    <span className="cat-slider" />
                  </label>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-text">
                    <span className="setting-title">Phone Notifications</span>
                    <span className="setting-subtitle">
                      See incoming notifications from your phone on this computer
                    </span>
                  </div>
                  <label className="cat-switch">
                    <input
                      type="checkbox"
                      checked={allowNotifications}
                      onChange={(e) => setAllowNotifications(e.target.checked)}
                    />
                    <span className="cat-slider" />
                  </label>
                </div>
              </div>

              {/* Identity & Keys */}
              <div className="cat-section-header" style={{ marginTop: "12px" }}>
                <div className="header-titles">
                  <h3 className="section-title">Security & Keys</h3>
                  <span className="section-subtitle">
                    Unique device credentials used for secure local verification
                  </span>
                </div>
              </div>

              <div className="setting-card">
                <div className="key-display-block">
                  <div className="key-top-row">
                    <span className="key-name">Device Fingerprint</span>
                    <button
                      className="cat-link-btn copy-btn"
                      onClick={handleCopyFingerprint}
                    >
                      {copiedFingerprint ? <Check size={12} /> : <Copy size={12} />}
                      <span>{copiedFingerprint ? "Copied" : "Copy"}</span>
                    </button>
                  </div>
                  <div className="key-string-mono">{identity.fingerprint}</div>
                </div>

                <div className="key-display-block" style={{ marginTop: "12px" }}>
                  <div className="key-top-row">
                    <span className="key-name">Certificate Fingerprint</span>
                  </div>
                  <div className="key-string-mono">{identity.spkiHash}</div>
                </div>
              </div>
            </div>
          )}
        </main>

      {/* 3. Pairing Dialog Modal (CassetteCat surfaceElevated style) */}
      {showPairDialog && (
        <div className="modal-scrim" onClick={() => setShowPairDialog(false)}>
          <div className="modal-dialog-box" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header-row">
              <div className="modal-title-left">
                <QrCode size={18} className="modal-title-icon" />
                <span className="modal-title-text">Pair New Device</span>
              </div>
              <button
                className="modal-close-action"
                onClick={() => setShowPairDialog(false)}
              >
                <X size={16} />
              </button>
            </div>

            <div className="modal-body-content">
              <div className="qr-box-container">
                <div className="qr-code-art">
                  <QrCode size={124} strokeWidth={1.5} />
                </div>
                <p className="qr-scan-instruction">
                  Scan this QR code with the Continue app on your phone or tablet to connect securely.
                </p>
              </div>

              <div className="or-hairline-label">
                <span>Or enter a code</span>
              </div>

              <form onSubmit={handlePairSubmit} className="pair-code-form">
                {!isUriMode ? (
                  <>
                    <SegmentedOtpInput
                      length={6}
                      value={pairingPayload}
                      onChange={setPairingPayload}
                    />
                    <button
                      type="button"
                      className="otp-mode-toggle"
                      onClick={() => setIsUriMode(true)}
                    >
                      Enter network address instead
                    </button>
                  </>
                ) : (
                  <>
                    <input
                      type="text"
                      className="cat-text-input"
                      placeholder="Paste address (e.g., continue://pair?ep=192.168.1.105:4433)..."
                      value={pairingPayload}
                      onChange={(e) => setPairingPayload(e.target.value)}
                      autoFocus
                    />
                    <button
                      type="button"
                      className="otp-mode-toggle"
                      onClick={() => setIsUriMode(false)}
                    >
                      Use 6-digit code instead
                    </button>
                  </>
                )}
                <div className="modal-bottom-actions">
                  <button
                    type="button"
                    className="cat-btn cat-btn-outline"
                    onClick={() => setShowPairDialog(false)}
                  >
                    Cancel
                  </button>
                  <button type="submit" className="cat-btn cat-btn-primary">
                    Connect
                  </button>
                </div>
              </form>
            </div>
          </div>
        </div>
      )}

      {/* 4. Toast Notification */}
      {toastMessage && (
        <div className="toast-pill-notification">
          <Check size={13} className="toast-check" />
          <span>{toastMessage}</span>
        </div>
      )}
    </div>
  );
}

export { App };
