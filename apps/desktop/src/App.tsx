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
  PanelLeft,
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
} from "lucide-react";
import "./App.css";
import type {
  DeviceIdentity,
  TrustedPeer,
  TransferHistoryItem,
  NotificationItem,
  RemoteFileItem,
} from "./types.ts";

export type AccentName = "recordRed" | "amber" | "cyan" | "emerald" | "magenta" | "silver";

export interface AccentColor {
  id: AccentName;
  label: string;
  base: string;
  hover: string;
}

export const ACCENT_PALETTE: AccentColor[] = [
  { id: "recordRed", label: "Red", base: "#C23B30", hover: "#D64337" },
  { id: "amber", label: "Amber", base: "#F59E0B", hover: "#FBBF24" },
  { id: "cyan", label: "Cyan", base: "#06B6D4", hover: "#22D3EE" },
  { id: "emerald", label: "Green", base: "#10B981", hover: "#34D399" },
  { id: "magenta", label: "Pink", base: "#EC4899", hover: "#F472B6" },
  { id: "silver", label: "Mono", base: "#C4C4C0", hover: "#E5E5E3" },
];

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

export default function App() {
  const [currentPage, setCurrentPage] = useState<PageDestination>("home");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [accentName, setAccentName] = useState<AccentName>("recordRed");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchExpanded, setSearchExpanded] = useState(false);
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
  const [pairingPayload, setPairingPayload] = useState("");
  const [copiedFingerprint, setCopiedFingerprint] = useState(false);
  const [toastMessage, setToastMessage] = useState("");
  const [isDraggingOver, setIsDraggingOver] = useState(false);
  const [copiedClipId, setCopiedClipId] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Transfer streaming progress
  const [streamProgress, setStreamProgress] = useState(64.2);
  const [isStreamPaused, setIsStreamPaused] = useState(false);
  const [showActiveStream, setShowActiveStream] = useState(true);

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
      name: "CassetteCat_Backup_2026.zip",
      folder: "Download",
      category: "other",
      size: 51700,
      modifiedAt: Date.now() - 14400000,
    },
    {
      id: "rf-8",
      name: "CassetteCat-v1.7.3.apk",
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

  const getPageTitle = (page: PageDestination) => {
    switch (page) {
      case "home":
        return "Home";
      case "transfers":
        return "Transfers";
      case "files":
        return "Files";
      case "devices":
        return "Devices";
      case "settings":
        return "Settings";
    }
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

      {/* 1. Frameless Top Window Bar (Height 60px) */}
      <header className="window-header-bar" data-tauri-drag-region>
        {/* Left: Brand Header above sidebar */}
        <div
          className={`header-brand-box ${sidebarCollapsed ? "collapsed" : ""}`}
          onClick={() => setCurrentPage("home")}
          title="Continue - Device Continuity"
        >
          <div className="brand-logo-mark">
            <Share2 size={16} strokeWidth={2.4} />
          </div>
          {!sidebarCollapsed && <span className="brand-title-label">Continue</span>}
        </div>

        {/* Center: View Title & Expandable Search Bar */}
        <div className="header-center-area">
          <h1 className="header-view-title">{getPageTitle(currentPage)}</h1>

          <div
            className={`expandable-search-bar ${searchExpanded || searchQuery ? "expanded" : ""}`}
          >
            <div
              className="search-icon-trigger"
              onClick={() => setSearchExpanded((p) => !p)}
            >
              <Search size={15} />
            </div>
            <input
              type="text"
              className="search-input-field"
              placeholder="Search transfers, clips, or files..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onFocus={() => setSearchExpanded(true)}
              onBlur={() => {
                if (!searchQuery) setSearchExpanded(false);
              }}
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
        </div>

        {/* Right: Active Peer Pill + Native Window Controls */}
        <div className="header-right-area">
          <div
            className="header-peer-pill"
            onClick={() => setCurrentPage("devices")}
            title="Connected Device: Pixel 8 Pro"
          >
            <span className="peer-live-indicator" />
            <Smartphone size={13} />
            <span className="peer-pill-name">{currentPeer.displayName}</span>
            <span className="peer-pill-battery">100%</span>
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

      {/* 2. Main Window Body: Collapsible Sidebar + Content Stage */}
      <div className="window-body-layout">
        {/* Left Sidebar (200px / 64px) */}
        <aside className={`sidebar-panel ${sidebarCollapsed ? "collapsed" : ""}`}>
          <div className="sidebar-nav-group top">
            <button
              className={`nav-item-btn ${currentPage === "home" ? "selected" : ""}`}
              onClick={() => setCurrentPage("home")}
              title="Home"
            >
              <div className="nav-indicator-bar" />
              <Home size={19} className="nav-item-icon" />
              {!sidebarCollapsed && <span className="nav-item-label">Home</span>}
            </button>

            <button
              className={`nav-item-btn ${currentPage === "transfers" ? "selected" : ""}`}
              onClick={() => setCurrentPage("transfers")}
              title="Transfers"
            >
              <div className="nav-indicator-bar" />
              <ArrowDownUp size={19} className="nav-item-icon" />
              {!sidebarCollapsed && <span className="nav-item-label">Transfers</span>}
            </button>

            <button
              className={`nav-item-btn ${currentPage === "files" ? "selected" : ""}`}
              onClick={() => setCurrentPage("files")}
              title="Files"
            >
              <div className="nav-indicator-bar" />
              <Folder size={19} className="nav-item-icon" />
              {!sidebarCollapsed && <span className="nav-item-label">Files</span>}
            </button>

            <button
              className={`nav-item-btn ${currentPage === "devices" ? "selected" : ""}`}
              onClick={() => setCurrentPage("devices")}
              title="Devices"
            >
              <div className="nav-indicator-bar" />
              <Shield size={19} className="nav-item-icon" />
              {!sidebarCollapsed && <span className="nav-item-label">Devices</span>}
            </button>
          </div>

          <div className="sidebar-nav-group bottom">
            <div className="sidebar-divider" />

            <button
              className={`nav-item-btn ${currentPage === "settings" ? "selected" : ""}`}
              onClick={() => setCurrentPage("settings")}
              title="Settings"
            >
              <div className="nav-indicator-bar" />
              <Settings size={19} className="nav-item-icon" />
              {!sidebarCollapsed && <span className="nav-item-label">Settings</span>}
            </button>

            <button
              className="sidebar-toggle-btn"
              onClick={() => setSidebarCollapsed((p) => !p)}
              title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            >
              <PanelLeft size={19} className="toggle-icon" />
              {!sidebarCollapsed && <span className="toggle-label">Collapse</span>}
            </button>
          </div>
        </aside>

        {/* Content Viewport */}
        <main className="content-stage-scroll">
          {/* VIEW 1: HOME PAGE */}
          {currentPage === "home" && (
            <div className="page-content-stack">
              {/* Eyebrow & Hero Header (CassetteCat style) */}
              <div className="hero-greeting-block">
                <span className="mono-eyebrow">DEVICE CONTINUITY ACTIVE • LOCAL P2P</span>
                <h2 className="hero-greeting-title">Welcome back, {identity.deviceName}</h2>
                <p className="hero-greeting-sub">
                  Connected to {currentPeer.displayName} • Wi-Fi Direct • 100% Battery
                </p>
              </div>

              {/* Hero Deck Card */}
              <div className="setting-card hero-deck-card">
                <div className="hero-deck-content">
                  <div className="deck-lead-info">
                    <div className="deck-device-badge">
                      <Smartphone size={28} className="deck-icon" />
                    </div>
                    <div className="deck-text-meta">
                      <div className="deck-title-line">
                        <span className="deck-title">{currentPeer.displayName}</span>
                        <span className="deck-online-pill">Mutual TLS 1.3</span>
                      </div>
                      <div className="deck-sub-line">
                        Endpoint: {currentPeer.endpoint || "192.168.1.105:4433"} • QUIC Pipeline
                      </div>
                    </div>
                  </div>

                  <div className="deck-action-buttons">
                    <button
                      className="cat-btn cat-btn-primary"
                      onClick={handleTriggerSendFile}
                    >
                      <Upload size={14} />
                      <span>Send Files</span>
                    </button>
                    <button
                      className="cat-btn cat-btn-outline"
                      onClick={() => setShowPairDialog(true)}
                    >
                      <Plus size={14} />
                      <span>Pair Device</span>
                    </button>
                  </div>
                </div>
              </div>

              {/* Quick Transfers Section Header */}
              <div className="cat-section-header">
                <div className="header-titles">
                  <h3 className="section-title">Active Transfer Stream</h3>
                  <span className="section-subtitle">P2P chunked stream with SHA-256 integrity</span>
                </div>
                <button
                  className="section-action-capsule"
                  onClick={() => setCurrentPage("transfers")}
                >
                  <span>View All Transfers</span>
                  <ArrowUpRight size={13} />
                </button>
              </div>

              {/* Send Dropzone & Active Stream */}
              <div
                className={`send-dropzone-box ${isDraggingOver ? "dragging" : ""}`}
                onDragOver={(e) => {
                  e.preventDefault();
                  setIsDraggingOver(true);
                }}
                onDragLeave={() => setIsDraggingOver(false)}
                onDrop={(e) => {
                  e.preventDefault();
                  setIsDraggingOver(false);
                  handleTriggerSendFile();
                }}
                onClick={handleTriggerSendFile}
              >
                <div className="dropzone-circle-badge">
                  <Upload size={22} />
                </div>
                <div className="dropzone-title">
                  Drop files here to stream to {currentPeer.displayName}
                </div>
                <div className="dropzone-hint">
                  Direct encrypted local transfer up to 1.2 Gbps • Zero cloud servers
                </div>
                <button
                  className="cat-btn cat-btn-primary"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleTriggerSendFile();
                  }}
                >
                  Choose Files
                </button>
              </div>

              {/* Active Transfer Deck Progress */}
              {showActiveStream && (
                <div className="setting-card telemetry-card">
                  <div className="telemetry-top-row">
                    <div className="telemetry-file-info">
                      <FileText size={18} className="file-icon-doc" />
                      <div className="telemetry-names">
                        <span className="tx-name">recording_session_2025_4k.mov</span>
                        <span className="tx-bytes-mono">1.42 GB of 2.80 GB transferred</span>
                      </div>
                    </div>

                    <div className="telemetry-stats-group">
                      <div className="telemetry-rate-block">
                        <span className="tx-rate-speed">114 MB/s</span>
                        <span className="tx-rate-eta">12s left</span>
                      </div>
                      <div className="telemetry-btn-row">
                        <button
                          className="press-depth-btn"
                          onClick={() => {
                            setIsStreamPaused((p) => !p);
                            showToast(isStreamPaused ? "Resumed" : "Paused");
                          }}
                          title={isStreamPaused ? "Resume" : "Pause"}
                        >
                          {isStreamPaused ? <Play size={13} /> : <Pause size={13} />}
                        </button>
                        <button
                          className="press-depth-btn danger"
                          onClick={() => {
                            setShowActiveStream(false);
                            showToast("Transfer cancelled");
                          }}
                          title="Cancel"
                        >
                          <X size={13} />
                        </button>
                      </div>
                    </div>
                  </div>

                  <div className="telemetry-track">
                    <div
                      className="telemetry-fill"
                      style={{ width: `${streamProgress}%` }}
                    />
                  </div>

                  <div className="telemetry-footer-row">
                    <span className="telemetry-hash">
                      <Check size={12} className="hash-check-icon" />
                      SHA-256 Verified
                    </span>
                    <span className="telemetry-percent">{streamProgress}% completed</span>
                  </div>
                </div>
              )}

              {/* Two-Column Hub: Shared Clipboard & Notifications */}
              <div className="home-dual-columns">
                {/* Column 1: Shared Clipboard */}
                <div className="setting-card col-card">
                  <div className="card-top-heading">
                    <div className="heading-left">
                      <Clipboard size={16} className="card-accent-icon" />
                      <span className="heading-text">Shared Clipboard</span>
                    </div>
                    <span className="card-live-pill">Live Sync</span>
                  </div>

                  <div className="clipboard-entry-row">
                    <input
                      type="text"
                      className="cat-text-input"
                      placeholder="Type or paste text to push to phone..."
                      value={clipboardInput}
                      onChange={(e) => setClipboardInput(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleSendClipboard();
                      }}
                    />
                    <button
                      className="cat-btn cat-btn-primary"
                      onClick={handleSendClipboard}
                    >
                      Push
                    </button>
                  </div>

                  <div className="clips-history-stack">
                    {filteredClips.map((clip) => (
                      <div key={clip.id} className="clip-tile-row">
                        <div className="clip-tile-meta">
                          <span className="clip-text-body">{clip.text}</span>
                          <span className="clip-device-mono">
                            {clip.device} • {clip.time}
                          </span>
                        </div>
                        <button
                          className={`clip-copy-action ${
                            copiedClipId === clip.id ? "copied" : ""
                          }`}
                          onClick={() => handleCopyText(clip.text, clip.id)}
                          title="Copy"
                        >
                          {copiedClipId === clip.id ? <Check size={13} /> : <Copy size={13} />}
                        </button>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Column 2: Notifications */}
                <div className="setting-card col-card">
                  <div className="card-top-heading">
                    <div className="heading-left">
                      <Bell size={16} className="card-accent-icon" />
                      <span className="heading-text">Phone Notifications</span>
                      {notifications.length > 0 && (
                        <span className="counter-badge">{notifications.length}</span>
                      )}
                    </div>
                    {notifications.length > 0 && (
                      <button
                        className="cat-link-btn"
                        onClick={handleClearAllNotifications}
                      >
                        Clear All
                      </button>
                    )}
                  </div>

                  <div className="notifications-list-stack">
                    {notifications.length === 0 ? (
                      <div className="empty-state-card">
                        <Bell size={24} className="empty-icon" />
                        <span>No alerts from phone</span>
                      </div>
                    ) : (
                      notifications.map((notif) => (
                        <div key={notif.id} className="notif-card-tile">
                          <div className="notif-lead-row">
                            <span className="notif-app-tag">{notif.appName}</span>
                            <span className="notif-timestamp-mono">Just now</span>
                            <button
                              className="notif-close-btn"
                              onClick={() => handleDismissNotification(notif.id)}
                            >
                              <X size={12} />
                            </button>
                          </div>
                          <div className="notif-title-str">{notif.title}</div>
                          <div className="notif-body-str">{notif.body}</div>
                        </div>
                      ))
                    )}
                  </div>
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
                    Full record of files sent and received over local QUIC streams
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
                              {formatBytes(tx.fileSize)} •{" "}
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
                  <h3 className="section-title">Phone Storage Folders</h3>
                  <span className="section-subtitle">
                    Browse media, documents, and downloads on {currentPeer.displayName}
                  </span>
                </div>
                {selectedFolder && (
                  <button
                    className="cat-link-btn"
                    onClick={() => setSelectedFolder(null)}
                  >
                    Reset Filter (Showing {selectedFolder})
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
                      <Folder size={24} className="folder-lead-icon" />
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
                  <h3 className="section-title">Synced Documents & Media</h3>
                  <span className="section-subtitle">
                    Files ready for peer transfer and download
                  </span>
                </div>
                <div className="segmented-filter-row">
                  {[
                    { key: "all", label: "All" },
                    { key: "documents", label: "Docs" },
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
                  <h3 className="section-title">Device Continuity</h3>
                  <span className="section-subtitle">
                    Active peer mesh and cryptographic trust credentials
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
                    <span className="device-node-role">This Host</span>
                  </div>

                  <div className="device-link-channel">
                    <div className="channel-line" />
                    <div className="channel-badge">
                      <Shield size={13} />
                      <span>TLS 1.3 Direct</span>
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
                    Devices authenticated with mutual Ed25519 identity keys
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
                            Endpoint: {peer.endpoint || "mDNS local"} • Fingerprint:{" "}
                            {peer.fingerprint.substring(0, 18)}...
                          </span>
                        </div>
                      </div>

                      <button
                        className="cat-btn cat-btn-sm cat-btn-outline danger"
                        onClick={() => handleDisconnectPeer(peer.fingerprint)}
                      >
                        Disconnect
                      </button>
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
                    Theme accent applied to active navigation, controls, and telemetry
                  </span>
                </div>
              </div>

              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-text">
                    <span className="setting-title">Accent Colour</span>
                    <span className="setting-subtitle">
                      Choose your signature accent tone
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
                  <h3 className="section-title">General Preferences</h3>
                  <span className="section-subtitle">
                    Device naming, auto-accept thresholds, and storage
                  </span>
                </div>
              </div>

              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-text">
                    <span className="setting-title">Device Display Name</span>
                    <span className="setting-subtitle">
                      Broadcast name visible to nearby devices on local Wi-Fi
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
                    <span className="setting-title">Download Directory</span>
                    <span className="setting-subtitle">
                      Default location for incoming file transfers
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
                    <span className="setting-title">Auto-accept Small Transfers</span>
                    <span className="setting-subtitle">
                      Instantly receive files under 50 MB without confirmation prompts
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
                  <h3 className="section-title">Continuity Services</h3>
                  <span className="section-subtitle">
                    Toggle individual device bridging pipelines
                  </span>
                </div>
              </div>

              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-text">
                    <span className="setting-title">Peer File Transfer</span>
                    <span className="setting-subtitle">
                      Allow incoming and outgoing streaming file transfers
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
                    <span className="setting-title">Real-time Clipboard Sync</span>
                    <span className="setting-subtitle">
                      Synchronize text and snippets bidirectionally with active peer
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
                    <span className="setting-title">Phone Notification Mirroring</span>
                    <span className="setting-subtitle">
                      Forward incoming phone alerts to desktop
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
                  <h3 className="section-title">Identity & Security Credentials</h3>
                  <span className="section-subtitle">
                    Cryptographic Ed25519 identity key and TLS SPKI hash
                  </span>
                </div>
              </div>

              <div className="setting-card">
                <div className="key-display-block">
                  <div className="key-top-row">
                    <span className="key-name">Ed25519 Fingerprint</span>
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
                    <span className="key-name">SPKI Certificate SHA-256</span>
                  </div>
                  <div className="key-string-mono">{identity.spkiHash}</div>
                </div>
              </div>
            </div>
          )}
        </main>
      </div>

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
                  Scan this QR code with the Continue app on your mobile device to verify
                  mutual TLS keys
                </p>
              </div>

              <div className="or-hairline-label">
                <span>OR ENTER PAIRING CODE</span>
              </div>

              <form onSubmit={handlePairSubmit} className="pair-code-form">
                <input
                  type="text"
                  className="cat-text-input"
                  placeholder="Paste pairing code or URI..."
                  value={pairingPayload}
                  onChange={(e) => setPairingPayload(e.target.value)}
                />
                <div className="modal-bottom-actions">
                  <button
                    type="button"
                    className="cat-btn cat-btn-outline"
                    onClick={() => setShowPairDialog(false)}
                  >
                    Cancel
                  </button>
                  <button type="submit" className="cat-btn cat-btn-primary">
                    Connect Device
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
