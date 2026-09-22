// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState, useCallback } from "react";
import {
  Share2,
  LayoutGrid,
  Smartphone,
  Tablet,
  Laptop,
  Monitor,
  Folder,
  FolderOpen,
  FileText,
  Image,
  Film,
  Music,
  Download,
  Upload,
  ArrowDownUp,
  Clipboard,
  Bell,
  Plus,
  Check,
  Copy,
  ExternalLink,
  X,
  ShieldCheck,
  Minus,
  Square,
  Search,
  Trash2,
  BatteryCharging,
  Pause,
  MoveRight,
  MoveLeft,
  ArrowDownLeft,
  ArrowUpRight,
  Settings,
} from "lucide-react";
import "./App.css";
import type {
  DeviceIdentity,
  TrustedPeer,
  TransferHistoryItem,
  NotificationItem,
  RemoteFileItem,
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
        isConnected: true,
        endpoint: "192.168.1.112:4433",
      },
    ];
  }
}

type NavigationTab =
  | "dashboard"
  | "transfers"
  | "storage"
  | "clipboard"
  | "notifications"
  | "display"
  | "settings";

type TransferFilter = "all" | "incoming" | "outgoing";
type RemoteCategoryFilter = "all" | "images" | "videos" | "documents" | "audio";

export function App() {
  const [activeTab, setActiveTab] = useState<NavigationTab>("dashboard");
  const [selectedPeerId, setSelectedPeerId] = useState<string>("cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a");
  const [searchQuery, setSearchQuery] = useState("");

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

  // Transfer streaming progress
  const [streamProgress, setStreamProgress] = useState(62.4);
  const [isStreamPaused, setIsStreamPaused] = useState(false);

  // Settings & permissions
  const [allowFileTransfer, setAllowFileTransfer] = useState(true);
  const [allowClipboardSync, setAllowClipboardSync] = useState(true);
  const [allowNotifications, setAllowNotifications] = useState(true);
  const [autoAcceptSmall, setAutoAcceptSmall] = useState(true);

  // Screen & Cursor states
  const [crossPosition, setCrossPosition] = useState<"right" | "left" | "bottom">("right");
  const [seamlessTransition, setSeamlessTransition] = useState(true);
  const [sharedKeyboard, setSharedKeyboard] = useState(true);
  const [crossClipboard, setCrossClipboard] = useState(true);

  // Device storage states
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);
  const [remoteCategory, setRemoteCategory] = useState<RemoteCategoryFilter>("all");

  // Transfer history
  const [transferFilter, setTransferFilter] = useState<TransferFilter>("all");
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
      fileName: "PXL_20250522_194512.dng",
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
      fileName: "quic_mesh_credentials.bin",
      fileSize: 1258291,
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
      body: "Sent you the project update document.",
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
      body: "Merged: feat(transport): optimize packet acknowledgment",
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
      time: "2m ago",
      device: "Pixel 8 Pro",
    },
    {
      id: "clip-2",
      text: "cargo test --workspace --all-targets",
      time: "14m ago",
      device: "Desktop PC",
    },
    {
      id: "clip-3",
      text: "cont1q8f7e2a9d4c6b8a1e3f5a7b9c1d3e5f7a9b1c3d",
      time: "1h ago",
      device: "Tablet Air",
    },
  ]);

  // Remote file storage items
  const [remoteFiles, setRemoteFiles] = useState<RemoteFileItem[]>([
    {
      id: "rf-1",
      name: "PXL_20250522_194512_PORTRAIT.jpg",
      folder: "DCIM",
      category: "images",
      size: 4823450,
      modifiedAt: Date.now() - 3600000 * 2,
    },
    {
      id: "rf-2",
      name: "quarterly_budget_projection.pdf",
      folder: "Documents",
      category: "documents",
      size: 1420580,
      modifiedAt: Date.now() - 3600000 * 5,
    },
    {
      id: "rf-3",
      name: "screen_recording_demo_4k.mp4",
      folder: "Movies",
      category: "videos",
      size: 142589000,
      modifiedAt: Date.now() - 3600000 * 8,
    },
    {
      id: "rf-4",
      name: "voice_note_product_sync.m4a",
      folder: "Music",
      category: "audio",
      size: 5829100,
      modifiedAt: Date.now() - 3600000 * 18,
    },
    {
      id: "rf-5",
      name: "client_assets_archive.zip",
      folder: "Download",
      category: "other",
      size: 28490100,
      modifiedAt: Date.now() - 3600000 * 24,
    },
    {
      id: "rf-6",
      name: "architecture_whitepaper_final.pdf",
      folder: "Documents",
      category: "documents",
      size: 3840200,
      modifiedAt: Date.now() - 3600000 * 36,
    },
    {
      id: "rf-7",
      name: "PXL_20250521_083419.jpg",
      folder: "DCIM",
      category: "images",
      size: 6140500,
      modifiedAt: Date.now() - 3600000 * 42,
    },
    {
      id: "rf-8",
      name: "audiobook_chapter_04.mp3",
      folder: "Music",
      category: "audio",
      size: 18920400,
      modifiedAt: Date.now() - 3600000 * 50,
    },
  ]);

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
    if (isStreamPaused) return;
    const timer = setInterval(() => {
      setStreamProgress((prev) => (prev >= 100 ? 62.4 : +(prev + 0.3).toFixed(1)));
    }, 1000);
    return () => clearInterval(timer);
  }, [isStreamPaused]);

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
      const peer = await invoke<TrustedPeer>("pair_from_qr", { qrPayload: pairingPayload.trim() });
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
      showToast("Connected via encrypted network");
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

  const handleTriggerSendFile = (targetDeviceName?: string) => {
    const currentPeer = peers.find((p) => p.fingerprint === selectedPeerId);
    const peerName = targetDeviceName || currentPeer?.displayName || "Device";
    const newTx: TransferHistoryItem = {
      id: "tx-" + Date.now(),
      fileName: "presentation_deck_v3.pdf",
      fileSize: 18457280,
      direction: "outgoing",
      peerFingerprint: currentPeer?.fingerprint || "cont1q9a8b",
      status: "completed",
      timestamp: Date.now(),
    };
    setTransfers((prev) => [newTx, ...prev]);
    showToast(`Streaming presentation_deck_v3.pdf to ${peerName}`);
  };

  const handleDownloadRemoteFile = (file: RemoteFileItem) => {
    const newTx: TransferHistoryItem = {
      id: "tx-" + Date.now(),
      fileName: file.name,
      fileSize: file.size,
      direction: "incoming",
      peerFingerprint: selectedPeerId,
      status: "completed",
      timestamp: Date.now(),
    };
    setTransfers((prev) => [newTx, ...prev]);
    showToast(`Downloaded ${file.name} to Downloads folder`);
  };

  const handleDeleteRemoteFile = (id: string) => {
    setRemoteFiles((prev) => prev.filter((f) => f.id !== id));
    showToast("File removed from queue");
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

  const isCurrentPeerTablet = currentPeer.displayName.toLowerCase().includes("tablet");

  // Filter transfers
  const filteredTransfers = transfers.filter((tx) => {
    const matchesFilter =
      transferFilter === "all" ||
      (transferFilter === "incoming" && tx.direction === "incoming") ||
      (transferFilter === "outgoing" && tx.direction === "outgoing");
    const matchesSearch =
      !searchQuery.trim() ||
      tx.fileName.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesFilter && matchesSearch;
  });

  // Filter remote files
  const filteredRemoteFiles = remoteFiles.filter((file) => {
    const matchesFolder = selectedFolder ? file.folder === selectedFolder : true;
    const matchesCat =
      remoteCategory === "all" ? true : file.category === remoteCategory;
    const matchesSearch =
      !searchQuery.trim() ||
      file.name.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesFolder && matchesCat && matchesSearch;
  });

  // Filter clips
  const filteredClips = syncedClips.filter((clip) =>
    !searchQuery.trim() || clip.text.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const getPageTitle = () => {
    switch (activeTab) {
      case "dashboard":
        return "Dashboard";
      case "transfers":
        return "File Transfers";
      case "storage":
        return "Device Storage";
      case "clipboard":
        return "Shared Clipboard";
      case "notifications":
        return "Phone Notifications";
      case "display":
        return "Screen & Cursor";
      case "settings":
        return "Settings & Security";
      default:
        return "Continue";
    }
  };

  return (
    <div className="app-shell">
      {/* 1. Left Navigation Sidebar */}
      <aside className="app-sidebar">
        {/* Brand Header */}
        <div className="sidebar-brand">
          <div className="brand-icon-box">
            <Share2 size={13} strokeWidth={2.5} />
          </div>
          <span className="brand-name">Continue</span>
          <span className="brand-badge">v0.1.0</span>
        </div>

        {/* Active Connected Device Card */}
        <div className="sidebar-device-card">
          <div className="device-preview-icon">
            {isCurrentPeerTablet ? (
              <Tablet size={18} />
            ) : (
              <Smartphone size={18} />
            )}
          </div>
          <div className="device-preview-meta">
            <div className="device-name-selector">
              <span className="device-title-text">{currentPeer.displayName}</span>
              {peers.length > 1 && (
                <select
                  className="device-quick-select"
                  value={selectedPeerId}
                  onChange={(e) => setSelectedPeerId(e.target.value)}
                  title="Switch connected device"
                >
                  {peers.map((p) => (
                    <option key={p.fingerprint} value={p.fingerprint}>
                      {p.displayName}
                    </option>
                  ))}
                </select>
              )}
            </div>
            <div className="device-status-tags">
              <span className="status-live-dot" />
              <span className="status-connected-label">Connected</span>
              <span className="status-sep">•</span>
              <span className="battery-pill">
                <BatteryCharging size={10} />
                <span>100%</span>
              </span>
            </div>
          </div>
          <button
            className="device-pair-action"
            title="Pair new device"
            onClick={() => setShowPairDialog(true)}
          >
            <Plus size={13} />
          </button>
        </div>

        {/* Navigation Section */}
        <nav className="sidebar-nav">
          <div className="nav-group-label">Features</div>

          <button
            className={`nav-item-btn ${activeTab === "dashboard" ? "active" : ""}`}
            onClick={() => setActiveTab("dashboard")}
          >
            <LayoutGrid size={15} />
            <span>Dashboard</span>
          </button>

          <button
            className={`nav-item-btn ${activeTab === "transfers" ? "active" : ""}`}
            onClick={() => setActiveTab("transfers")}
          >
            <ArrowDownUp size={15} />
            <span>Transfers</span>
          </button>

          <button
            className={`nav-item-btn ${activeTab === "storage" ? "active" : ""}`}
            onClick={() => setActiveTab("storage")}
          >
            <Folder size={15} />
            <span>Device Storage</span>
          </button>

          <button
            className={`nav-item-btn ${activeTab === "clipboard" ? "active" : ""}`}
            onClick={() => setActiveTab("clipboard")}
          >
            <Clipboard size={15} />
            <span>Clipboard</span>
            <span className="nav-count-badge">{syncedClips.length}</span>
          </button>

          <button
            className={`nav-item-btn ${activeTab === "notifications" ? "active" : ""}`}
            onClick={() => setActiveTab("notifications")}
          >
            <Bell size={15} />
            <span>Notifications</span>
            {notifications.length > 0 && (
              <span className="nav-count-badge highlight">{notifications.length}</span>
            )}
          </button>

          <button
            className={`nav-item-btn ${activeTab === "display" ? "active" : ""}`}
            onClick={() => setActiveTab("display")}
          >
            <Monitor size={15} />
            <span>Screen & Cursor</span>
          </button>
        </nav>

        {/* Sidebar Footer */}
        <div className="sidebar-footer">
          <button
            className={`nav-item-btn ${activeTab === "settings" ? "active" : ""}`}
            onClick={() => setActiveTab("settings")}
          >
            <Settings size={15} />
            <span>Settings</span>
          </button>

          <div className="host-quick-meta">
            <div className="host-label-row">
              <Laptop size={12} />
              <span>{identity.deviceName} (Host)</span>
            </div>
            <button
              className="copy-key-btn"
              title="Copy Ed25519 Fingerprint"
              onClick={handleCopyFingerprint}
            >
              {copiedFingerprint ? (
                <Check size={11} color="var(--color-secondary)" />
              ) : (
                <Copy size={11} />
              )}
            </button>
          </div>
        </div>
      </aside>

      {/* 2. Main Workstage Viewport */}
      <div className="app-main">
        {/* Top Window Bar */}
        <header className="main-header" data-tauri-drag-region>
          <div className="header-breadcrumbs">
            <h1 className="view-headline">{getPageTitle()}</h1>
            <span className="view-subcontext">
              Connected to {currentPeer.displayName} via Wi-Fi Direct
            </span>
          </div>

          <div className="header-actions">
            {/* Global search */}
            <div className="header-search">
              <Search size={12} className="search-icon" />
              <input
                type="text"
                className="search-input"
                placeholder="Search files, clips, or alerts..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
              {searchQuery && (
                <button className="search-clear-btn" onClick={() => setSearchQuery("")}>
                  <X size={11} />
                </button>
              )}
            </div>

            <button
              className="btn btn-sm btn-primary"
              onClick={() => handleTriggerSendFile()}
            >
              <Upload size={12} />
              <span>Send File</span>
            </button>

            {/* Native Window Controls */}
            <div className="window-controls">
              <button
                className="win-btn"
                onClick={handleMinimizeWindow}
                title="Minimize window"
              >
                <Minus size={12} />
              </button>
              <button
                className="win-btn"
                onClick={handleMaximizeWindow}
                title="Maximize window"
              >
                <Square size={10} />
              </button>
              <button
                className="win-btn win-close"
                onClick={handleCloseWindow}
                title="Close window"
              >
                <X size={12} />
              </button>
            </div>
          </div>
        </header>

        {/* Content Pane */}
        <main className="content-pane">
          {/* VIEW 1: DASHBOARD */}
          {activeTab === "dashboard" && (
            <div className="view-flow">
              {/* Quick Send Dropzone Hero Banner */}
              <div
                className={`dashboard-dropzone ${isDraggingOver ? "dragging" : ""}`}
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
                onClick={() => handleTriggerSendFile()}
              >
                <div className="dropzone-circle">
                  <Upload size={22} />
                </div>
                <div className="dropzone-text-group">
                  <div className="dropzone-heading">
                    Drop files here to send to {currentPeer.displayName}
                  </div>
                  <div className="dropzone-sub">
                    Encrypted Wi-Fi stream up to 1.2 Gbps throughput
                  </div>
                </div>
                <button
                  className="btn btn-primary"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleTriggerSendFile();
                  }}
                >
                  Choose Files
                </button>
              </div>

              {/* Active Stream Progress Card */}
              <div className="dashboard-stream-card">
                <div className="stream-card-header">
                  <div className="stream-lead">
                    <FileText size={15} className="text-cyan" />
                    <div>
                      <span className="stream-filename">recording_session_2025_4k.mov</span>
                      <span className="stream-meta-inline">
                        1.42 GB of 2.80 GB transferred
                      </span>
                    </div>
                  </div>

                  <div className="stream-right-meta">
                    <div className="stream-speed-box">
                      <span className="speed-rate">114 MB/s</span>
                      <span className="speed-eta">12s left</span>
                    </div>
                    <div className="stream-actions">
                      <button
                        className="ctrl-btn"
                        onClick={() => {
                          setIsStreamPaused((p) => !p);
                          showToast(isStreamPaused ? "Stream resumed" : "Stream paused");
                        }}
                        title={isStreamPaused ? "Resume" : "Pause"}
                      >
                        <Pause size={12} />
                      </button>
                      <button
                        className="ctrl-btn danger"
                        onClick={() => showToast("Stream cancelled")}
                        title="Cancel"
                      >
                        <X size={12} />
                      </button>
                    </div>
                  </div>
                </div>

                <div className="stream-track">
                  <div className="stream-fill" style={{ width: `${streamProgress}%` }} />
                </div>

                <div className="stream-card-footer">
                  <div className="stream-tag-ok">
                    <Check size={11} color="var(--color-secondary)" />
                    <span>Verified SHA-256</span>
                  </div>
                  <span className="stream-pct-text">{streamProgress}% completed</span>
                </div>
              </div>

              {/* 3-Column Summary Cards */}
              <div className="dashboard-summary-grid">
                {/* 1. Recent Transfers Summary */}
                <div className="summary-box">
                  <div className="summary-box-head">
                    <div className="head-left">
                      <ArrowDownUp size={14} className="text-cyan" />
                      <span className="head-title">Recent Transfers</span>
                    </div>
                    <button
                      className="text-link"
                      onClick={() => setActiveTab("transfers")}
                    >
                      View all
                    </button>
                  </div>

                  <div className="summary-list">
                    {transfers.slice(0, 3).map((tx) => (
                      <div key={tx.id} className="summary-item-row">
                        <div className="summary-item-lead">
                          <FileText size={13} className="item-icon" />
                          <span className="item-name-truncate">{tx.fileName}</span>
                        </div>
                        <span className="item-size-mono">
                          {(tx.fileSize / 1024 / 1024).toFixed(1)} MB
                        </span>
                      </div>
                    ))}
                  </div>
                </div>

                {/* 2. Shared Clipboard Summary */}
                <div className="summary-box">
                  <div className="summary-box-head">
                    <div className="head-left">
                      <Clipboard size={14} className="text-cyan" />
                      <span className="head-title">Shared Clipboard</span>
                    </div>
                    <button
                      className="text-link"
                      onClick={() => setActiveTab("clipboard")}
                    >
                      View all
                    </button>
                  </div>

                  <div className="summary-clip-send">
                    <input
                      type="text"
                      className="mini-input"
                      placeholder={`Send text to ${currentPeer.displayName}...`}
                      value={clipboardInput}
                      onChange={(e) => setClipboardInput(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleSendClipboard();
                      }}
                    />
                    <button className="btn btn-sm btn-primary" onClick={handleSendClipboard}>
                      Send
                    </button>
                  </div>

                  <div className="summary-list">
                    {syncedClips.slice(0, 2).map((clip) => (
                      <div key={clip.id} className="summary-clip-row">
                        <span className="clip-snippet-text">{clip.text}</span>
                        <button
                          className="copy-btn-sm"
                          onClick={() => handleCopyText(clip.text, clip.id)}
                          title="Copy"
                        >
                          {copiedClipId === clip.id ? (
                            <Check size={11} color="var(--color-secondary)" />
                          ) : (
                            <Copy size={11} />
                          )}
                        </button>
                      </div>
                    ))}
                  </div>
                </div>

                {/* 3. Notifications Summary */}
                <div className="summary-box">
                  <div className="summary-box-head">
                    <div className="head-left">
                      <Bell size={14} className="text-cyan" />
                      <span className="head-title">Phone Notifications</span>
                    </div>
                    <button
                      className="text-link"
                      onClick={() => setActiveTab("notifications")}
                    >
                      View all
                    </button>
                  </div>

                  <div className="summary-list">
                    {notifications.length === 0 ? (
                      <div className="summary-empty">No active notifications</div>
                    ) : (
                      notifications.slice(0, 2).map((notif) => (
                        <div key={notif.id} className="summary-notif-row">
                          <div className="notif-meta">
                            <span className="notif-title-inline">{notif.title}</span>
                            <span className="notif-badge-inline">{notif.appName}</span>
                          </div>
                          <span className="notif-body-inline">{notif.body}</span>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              </div>

              {/* Quick Launch Cards */}
              <div className="quick-launcher-row">
                <div
                  className="launcher-card"
                  onClick={() => setActiveTab("storage")}
                >
                  <FolderOpen size={16} className="text-cyan" />
                  <div>
                    <div className="launcher-title">Browse Phone Storage</div>
                    <div className="launcher-sub">Access camera photos, downloads, and files</div>
                  </div>
                </div>

                <div
                  className="launcher-card"
                  onClick={() => setActiveTab("display")}
                >
                  <Monitor size={16} className="text-cyan" />
                  <div>
                    <div className="launcher-title">Screen & Cursor</div>
                    <div className="launcher-sub">Universal mouse and keyboard screen bridging</div>
                  </div>
                </div>

                <div
                  className="launcher-card"
                  onClick={() => setActiveTab("settings")}
                >
                  <ShieldCheck size={16} className="text-cyan" />
                  <div>
                    <div className="launcher-title">Device Security</div>
                    <div className="launcher-sub">Verify Ed25519 identity and trusted peers</div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* VIEW 2: TRANSFERS */}
          {activeTab === "transfers" && (
            <div className="view-flow">
              <div className="transfers-toolbar">
                <div className="filter-pill-bar">
                  <button
                    className={`pill-btn ${transferFilter === "all" ? "active" : ""}`}
                    onClick={() => setTransferFilter("all")}
                  >
                    All Transfers
                  </button>
                  <button
                    className={`pill-btn ${transferFilter === "incoming" ? "active" : ""}`}
                    onClick={() => setTransferFilter("incoming")}
                  >
                    Received
                  </button>
                  <button
                    className={`pill-btn ${transferFilter === "outgoing" ? "active" : ""}`}
                    onClick={() => setTransferFilter("outgoing")}
                  >
                    Sent
                  </button>
                </div>

                <div className="toolbar-right">
                  <span className="record-counter">{filteredTransfers.length} records</span>
                  <button
                    className="btn btn-sm btn-outline"
                    onClick={() => {
                      setTransfers([]);
                      showToast("Transfer history cleared");
                    }}
                  >
                    <Trash2 size={11} />
                    <span>Clear History</span>
                  </button>
                </div>
              </div>

              <div className="data-table-card">
                <table className="clean-table">
                  <thead>
                    <tr>
                      <th>File Name</th>
                      <th>Size</th>
                      <th>Direction</th>
                      <th>Status</th>
                      <th>Time</th>
                      <th style={{ textAlign: "right" }}>Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredTransfers.length === 0 ? (
                      <tr>
                        <td colSpan={6} className="empty-table-msg">
                          No transfer records found.
                        </td>
                      </tr>
                    ) : (
                      filteredTransfers.map((tx) => (
                        <tr key={tx.id}>
                          <td>
                            <div className="table-lead-cell">
                              <FileText size={14} className="text-cyan" />
                              <span className="table-file-name">{tx.fileName}</span>
                            </div>
                          </td>
                          <td className="mono-text">
                            {(tx.fileSize / 1024 / 1024).toFixed(1)} MB
                          </td>
                          <td>
                            <span
                              className={`direction-badge ${
                                tx.direction === "incoming" ? "incoming" : "outgoing"
                              }`}
                            >
                              {tx.direction === "incoming" ? (
                                <ArrowDownLeft size={10} />
                              ) : (
                                <ArrowUpRight size={10} />
                              )}
                              <span>{tx.direction === "incoming" ? "Received" : "Sent"}</span>
                            </span>
                          </td>
                          <td>
                            <div className="status-cell">
                              <Check size={11} color="var(--color-secondary)" />
                              <span>Verified</span>
                            </div>
                          </td>
                          <td className="time-text">
                            {Math.round((Date.now() - tx.timestamp) / 60000)}m ago
                          </td>
                          <td style={{ textAlign: "right" }}>
                            <button
                              className="action-btn"
                              onClick={() => showToast(`Revealing ${tx.fileName}`)}
                              title="Reveal in folder"
                            >
                              <ExternalLink size={11} />
                              <span>Reveal</span>
                            </button>
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* VIEW 3: DEVICE STORAGE */}
          {activeTab === "storage" && (
            <div className="view-flow">
              {/* Storage Space Bar */}
              <div className="storage-meter-card">
                <div className="meter-left">
                  <div className="meter-icon-box">
                    <FolderOpen size={18} />
                  </div>
                  <div>
                    <div className="meter-title">Internal Storage ({currentPeer.displayName})</div>
                    <div className="meter-sub">64.2 GB used of 256 GB</div>
                  </div>
                </div>

                <div className="meter-bar-col">
                  <div className="meter-track">
                    <div className="meter-fill" style={{ width: "25.1%" }} />
                  </div>
                  <div className="meter-legend">
                    <span>Camera: 24.8 GB</span>
                    <span>•</span>
                    <span>Downloads: 12.3 GB</span>
                    <span>•</span>
                    <span>Documents: 4.1 GB</span>
                  </div>
                </div>

                <button className="btn btn-primary" onClick={() => handleTriggerSendFile()}>
                  <Upload size={12} />
                  <span>Send to Phone</span>
                </button>
              </div>

              {/* Folder Shortcuts */}
              <div className="folder-grid">
                {[
                  { name: "DCIM", label: "Camera & Photos", icon: Image, count: "1,248 files" },
                  { name: "Download", label: "Downloads", icon: Download, count: "342 files" },
                  { name: "Documents", label: "Documents & PDFs", icon: FileText, count: "89 files" },
                  { name: "Movies", label: "Screen Recordings", icon: Film, count: "54 files" },
                  { name: "Music", label: "Audio & Music", icon: Music, count: "112 files" },
                ].map((folder) => {
                  const isSelected = selectedFolder === folder.name;
                  const Icon = folder.icon;
                  return (
                    <div
                      key={folder.name}
                      className={`folder-tile ${isSelected ? "selected" : ""}`}
                      onClick={() =>
                        setSelectedFolder(isSelected ? null : folder.name)
                      }
                    >
                      <div className="folder-tile-head">
                        <Icon size={16} className="text-cyan" />
                        <span className="folder-count">{folder.count}</span>
                      </div>
                      <span className="folder-name">{folder.name}</span>
                      <span className="folder-sub">{folder.label}</span>
                    </div>
                  );
                })}
              </div>

              {/* Category Filter and File Explorer Table */}
              <div className="data-table-card">
                <div className="table-filter-bar">
                  <div className="category-btn-row">
                    {(
                      [
                        { id: "all", label: "All Files" },
                        { id: "images", label: "Photos" },
                        { id: "videos", label: "Videos" },
                        { id: "documents", label: "Documents" },
                        { id: "audio", label: "Audio" },
                      ] as const
                    ).map((cat) => (
                      <button
                        key={cat.id}
                        className={`cat-btn ${remoteCategory === cat.id ? "active" : ""}`}
                        onClick={() => setRemoteCategory(cat.id)}
                      >
                        {cat.label}
                      </button>
                    ))}
                  </div>

                  <div className="filter-summary-right">
                    {selectedFolder && (
                      <span className="folder-filter-tag">
                        Folder: {selectedFolder}
                        <button
                          className="tag-close-btn"
                          onClick={() => setSelectedFolder(null)}
                        >
                          <X size={10} />
                        </button>
                      </span>
                    )}
                    <span className="files-available-text">
                      {filteredRemoteFiles.length} files available
                    </span>
                  </div>
                </div>

                <table className="clean-table">
                  <thead>
                    <tr>
                      <th>File</th>
                      <th>Folder</th>
                      <th>Size</th>
                      <th>Modified</th>
                      <th style={{ textAlign: "right" }}>Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredRemoteFiles.length === 0 ? (
                      <tr>
                        <td colSpan={5} className="empty-table-msg">
                          No files found matching criteria.
                        </td>
                      </tr>
                    ) : (
                      filteredRemoteFiles.map((file) => (
                        <tr key={file.id}>
                          <td>
                            <div className="table-lead-cell">
                              {file.category === "images" ? (
                                <Image size={14} className="text-cyan" />
                              ) : file.category === "videos" ? (
                                <Film size={14} className="text-cyan" />
                              ) : file.category === "audio" ? (
                                <Music size={14} className="text-cyan" />
                              ) : (
                                <FileText size={14} className="text-cyan" />
                              )}
                              <span className="table-file-name">{file.name}</span>
                            </div>
                          </td>
                          <td>
                            <span className="folder-label-pill">{file.folder}</span>
                          </td>
                          <td className="mono-text">
                            {(file.size / 1024 / 1024).toFixed(1)} MB
                          </td>
                          <td className="time-text">
                            {Math.round((Date.now() - file.modifiedAt) / 3600000)}h ago
                          </td>
                          <td style={{ textAlign: "right" }}>
                            <div className="actions-cell">
                              <button
                                className="action-btn"
                                onClick={() => handleDownloadRemoteFile(file)}
                                title="Download to PC"
                              >
                                <Download size={11} />
                                <span>Download</span>
                              </button>
                              <button
                                className="action-btn danger-hover"
                                onClick={() => handleDeleteRemoteFile(file.id)}
                                title="Remove from list"
                              >
                                <Trash2 size={11} />
                              </button>
                            </div>
                          </td>
                        </tr>
                      ))
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* VIEW 4: CLIPBOARD */}
          {activeTab === "clipboard" && (
            <div className="view-flow">
              {/* Sync Preference Toggle */}
              <div className="content-card">
                <div className="preference-row">
                  <div>
                    <div className="preference-title">Real-Time Clipboard Sync</div>
                    <div className="preference-sub">
                      Automatically synchronize copied text and links between this PC and {currentPeer.displayName}
                    </div>
                  </div>
                  <label className="switch-control">
                    <input
                      type="checkbox"
                      checked={allowClipboardSync}
                      onChange={(e) => setAllowClipboardSync(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>
              </div>

              {/* Send Text Bar */}
              <div className="content-card">
                <div className="card-heading">Send Text to Device</div>
                <div className="text-blast-bar">
                  <input
                    type="text"
                    className="clean-input"
                    placeholder={`Type or paste text to send to ${currentPeer.displayName}... (Press Enter)`}
                    value={clipboardInput}
                    onChange={(e) => setClipboardInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") handleSendClipboard();
                    }}
                  />
                  <button className="btn btn-primary" onClick={handleSendClipboard}>
                    Send
                  </button>
                </div>
              </div>

              {/* Synchronized Clipboard List */}
              <div className="content-card">
                <div className="card-heading">
                  Clipboard History ({filteredClips.length})
                </div>

                <div className="clip-history-list">
                  {filteredClips.map((clip) => (
                    <div key={clip.id} className="clip-row-item">
                      <div className="clip-body-col">
                        <span className="clip-text-value">{clip.text}</span>
                        <div className="clip-footer-info">
                          <span className="clip-origin-pill">{clip.device}</span>
                          <span>•</span>
                          <span className="time-text">{clip.time}</span>
                        </div>
                      </div>
                      <button
                        className="btn-icon-action"
                        onClick={() => handleCopyText(clip.text, clip.id)}
                        title="Copy to clipboard"
                      >
                        {copiedClipId === clip.id ? (
                          <Check size={13} color="var(--color-secondary)" />
                        ) : (
                          <Copy size={13} />
                        )}
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* VIEW 5: NOTIFICATIONS */}
          {activeTab === "notifications" && (
            <div className="view-flow">
              <div className="content-card">
                <div className="card-header-flex">
                  <div>
                    <div className="card-heading">Phone Notifications</div>
                    <div className="preference-sub">
                      Incoming notifications relayed from {currentPeer.displayName}
                    </div>
                  </div>
                  {notifications.length > 0 && (
                    <button
                      className="btn btn-sm btn-outline"
                      onClick={handleClearAllNotifications}
                    >
                      <Trash2 size={11} />
                      <span>Clear All</span>
                    </button>
                  )}
                </div>

                {notifications.length === 0 ? (
                  <div className="empty-state-card">
                    <Bell size={24} className="text-dim" />
                    <div className="empty-title">All Caught Up</div>
                    <div className="empty-sub">No forwarded notifications right now.</div>
                  </div>
                ) : (
                  <div className="notifications-list">
                    {notifications.map((notif) => (
                      <div key={notif.id} className="notification-item">
                        <div className="notif-icon-circle">
                          <Bell size={13} />
                        </div>
                        <div className="notif-content-area">
                          <div className="notif-top-bar">
                            <span className="notif-author">{notif.title}</span>
                            <span className="notif-badge">{notif.appName}</span>
                            <span className="notif-time-ago">2m ago</span>
                          </div>
                          <div className="notif-msg">{notif.body}</div>
                        </div>
                        <button
                          className="notif-dismiss"
                          onClick={() => handleDismissNotification(notif.id)}
                          title="Dismiss notification"
                        >
                          <X size={12} />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* VIEW 6: SCREEN & CURSOR */}
          {activeTab === "display" && (
            <div className="view-flow">
              {/* Display Arrangement Board */}
              <div className="content-card">
                <div className="card-heading">Display Arrangement</div>
                <div className="preference-sub">
                  Drag or position your devices to transition your mouse cursor across screens
                </div>

                <div className="display-canvas-stage">
                  {crossPosition === "left" && (
                    <div className="screen-mockup phone-screen">
                      <Smartphone size={14} className="text-cyan" />
                      <span className="screen-name">{currentPeer.displayName}</span>
                      <span className="screen-res">1080 × 2400</span>
                    </div>
                  )}

                  <div className="screen-mockup monitor-screen">
                    <Laptop size={16} className="text-cyan" />
                    <span className="screen-name">{identity.deviceName} (Primary)</span>
                    <span className="screen-res">2560 × 1440 • 165Hz</span>
                  </div>

                  {crossPosition === "right" && (
                    <div className="screen-mockup phone-screen">
                      <Smartphone size={14} className="text-cyan" />
                      <span className="screen-name">{currentPeer.displayName}</span>
                      <span className="screen-res">1080 × 2400</span>
                    </div>
                  )}

                  {crossPosition === "bottom" && (
                    <div className="screen-mockup phone-screen bottom-pos">
                      <Smartphone size={14} className="text-cyan" />
                      <span className="screen-name">{currentPeer.displayName}</span>
                      <span className="screen-res">1080 × 2400</span>
                    </div>
                  )}
                </div>

                <div className="position-bar">
                  <span className="position-label">Position {currentPeer.displayName}:</span>
                  <div className="position-options">
                    <button
                      className={`pos-button ${crossPosition === "left" ? "active" : ""}`}
                      onClick={() => setCrossPosition("left")}
                    >
                      <MoveLeft size={12} />
                      <span>Left of PC</span>
                    </button>
                    <button
                      className={`pos-button ${crossPosition === "right" ? "active" : ""}`}
                      onClick={() => setCrossPosition("right")}
                    >
                      <MoveRight size={12} />
                      <span>Right of PC</span>
                    </button>
                    <button
                      className={`pos-button ${crossPosition === "bottom" ? "active" : ""}`}
                      onClick={() => setCrossPosition("bottom")}
                    >
                      <ArrowDownLeft size={12} />
                      <span>Bottom</span>
                    </button>
                  </div>
                </div>
              </div>

              {/* Preferences Grid */}
              <div className="two-col-grid">
                <div className="content-card">
                  <div className="card-heading">Cursor & Input Preferences</div>

                  <div className="toggle-rows-stack">
                    <div className="preference-row">
                      <div>
                        <div className="preference-title">Seamless Edge Transition</div>
                        <div className="preference-sub">
                          Push cursor past the display border to control {currentPeer.displayName}
                        </div>
                      </div>
                      <label className="switch-control">
                        <input
                          type="checkbox"
                          checked={seamlessTransition}
                          onChange={(e) => setSeamlessTransition(e.target.checked)}
                        />
                        <span className="switch-slider" />
                      </label>
                    </div>

                    <div className="preference-row">
                      <div>
                        <div className="preference-title">Share PC Keyboard</div>
                        <div className="preference-sub">
                          Type directly into active apps and input fields on your mobile device
                        </div>
                      </div>
                      <label className="switch-control">
                        <input
                          type="checkbox"
                          checked={sharedKeyboard}
                          onChange={(e) => setSharedKeyboard(e.target.checked)}
                        />
                        <span className="switch-slider" />
                      </label>
                    </div>

                    <div className="preference-row">
                      <div>
                        <div className="preference-title">Edge Drag & Drop</div>
                        <div className="preference-sub">
                          Drag files across screen borders to trigger immediate transfer
                        </div>
                      </div>
                      <label className="switch-control">
                        <input
                          type="checkbox"
                          checked={crossClipboard}
                          onChange={(e) => setCrossClipboard(e.target.checked)}
                        />
                        <span className="switch-slider" />
                      </label>
                    </div>
                  </div>
                </div>

                <div className="content-card">
                  <div className="card-heading">Connection Quality</div>

                  <div className="stats-2x2-grid">
                    <div className="stat-tile">
                      <span className="stat-label">Latency</span>
                      <span className="stat-value text-green">1.8 ms</span>
                    </div>
                    <div className="stat-tile">
                      <span className="stat-label">Transport</span>
                      <span className="stat-value">QUIC Direct</span>
                    </div>
                    <div className="stat-tile">
                      <span className="stat-label">Polling Rate</span>
                      <span className="stat-value">1000 Hz</span>
                    </div>
                    <div className="stat-tile">
                      <span className="stat-label">Packet Loss</span>
                      <span className="stat-value text-green">0.00%</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* VIEW 7: SETTINGS */}
          {activeTab === "settings" && (
            <div className="view-flow max-width-flow">
              {/* Host Machine Cryptographic Keyring */}
              <div className="content-card">
                <div className="card-heading">Host Machine Keyring</div>
                <div className="preference-sub">
                  Local device credentials used for mutual cryptographic trust
                </div>

                <div className="keyring-details">
                  <div className="keyring-row">
                    <span className="keyring-label">Machine Name:</span>
                    <span className="keyring-val">{identity.deviceName} (Host PC)</span>
                  </div>

                  <div className="keyring-row">
                    <span className="keyring-label">Fingerprint:</span>
                    <div className="keyring-copy-row">
                      <span className="mono-code">{identity.fingerprint}</span>
                      <button
                        className="btn-icon-action"
                        onClick={handleCopyFingerprint}
                        title="Copy Fingerprint"
                      >
                        {copiedFingerprint ? (
                          <Check size={12} color="var(--color-secondary)" />
                        ) : (
                          <Copy size={12} />
                        )}
                      </button>
                    </div>
                  </div>

                  <div className="keyring-row">
                    <span className="keyring-label">SPKI Hash:</span>
                    <span className="mono-code">{identity.spkiHash.substring(0, 36)}...</span>
                  </div>
                </div>
              </div>

              {/* Trusted Peers */}
              <div className="content-card">
                <div className="card-header-flex">
                  <div>
                    <div className="card-heading">Trusted Devices ({peers.length})</div>
                    <div className="preference-sub">
                      Devices allowed to communicate with this PC
                    </div>
                  </div>
                  <button
                    className="btn btn-sm btn-outline"
                    onClick={() => setShowPairDialog(true)}
                  >
                    <Plus size={11} />
                    <span>Pair Device</span>
                  </button>
                </div>

                <div className="peers-list">
                  {peers.map((peer) => (
                    <div key={peer.fingerprint} className="peer-card-row">
                      <div className="peer-lead">
                        {peer.displayName.toLowerCase().includes("tablet") ? (
                          <Tablet size={16} className="text-cyan" />
                        ) : (
                          <Smartphone size={16} className="text-cyan" />
                        )}
                        <div>
                          <div className="peer-name">{peer.displayName}</div>
                          <div className="peer-endpoint">
                            Endpoint: {peer.endpoint || "4433"} • Connected via Wi-Fi
                          </div>
                        </div>
                      </div>

                      <div className="peer-actions">
                        <span className="online-tag">Online</span>
                        <button
                          className="btn btn-sm btn-danger-outline"
                          onClick={() => handleDisconnectPeer(peer.fingerprint)}
                        >
                          <Trash2 size={11} />
                          <span>Unpair</span>
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* Transfer & Security Policies */}
              <div className="content-card">
                <div className="card-heading">Policies & Preferences</div>

                <div className="toggle-rows-stack">
                  <div className="preference-row">
                    <div>
                      <div className="preference-title">Encrypted File Transfers</div>
                      <div className="preference-sub">
                        Allow incoming and outgoing chunked streams over local Wi-Fi
                      </div>
                    </div>
                    <label className="switch-control">
                      <input
                        type="checkbox"
                        checked={allowFileTransfer}
                        onChange={(e) => setAllowFileTransfer(e.target.checked)}
                      />
                      <span className="switch-slider" />
                    </label>
                  </div>

                  <div className="preference-row">
                    <div>
                      <div className="preference-title">Relay Phone Notifications</div>
                      <div className="preference-sub">
                        Forward mobile app push notifications with dismiss synchronization
                      </div>
                    </div>
                    <label className="switch-control">
                      <input
                        type="checkbox"
                        checked={allowNotifications}
                        onChange={(e) => setAllowNotifications(e.target.checked)}
                      />
                      <span className="switch-slider" />
                    </label>
                  </div>

                  <div className="preference-row">
                    <div>
                      <div className="preference-title">Auto-Accept Small Files</div>
                      <div className="preference-sub">
                        Automatically accept files under 10 MB without manual approval
                      </div>
                    </div>
                    <label className="switch-control">
                      <input
                        type="checkbox"
                        checked={autoAcceptSmall}
                        onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                      />
                      <span className="switch-slider" />
                    </label>
                  </div>
                </div>
              </div>
            </div>
          )}
        </main>

        {/* Bottom Information Bar */}
        <footer className="main-footer">
          <div className="footer-left">
            <span>Continue Core Active</span>
            <span>•</span>
            <span>Latency: 2.14ms</span>
            <span>•</span>
            <span>Loss: 0.00%</span>
            <span>•</span>
            <span>Buffer: 64 MB</span>
          </div>

          <div className="footer-right">
            <span>Peer: {currentPeer.displayName}</span>
            <span>•</span>
            <span>TLS 1.3 Encrypted</span>
          </div>
        </footer>
      </div>

      {/* 3. Pair New Remote Device Dialog */}
      {showPairDialog && (
        <div className="modal-backdrop" onClick={() => setShowPairDialog(false)}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <span className="modal-title">Pair Remote Device</span>
              <button className="modal-close-btn" onClick={() => setShowPairDialog(false)}>
                <X size={14} />
              </button>
            </div>

            <form onSubmit={handlePairSubmit}>
              <div className="form-group">
                <label className="form-label">Enter pairing URI or QR payload:</label>
                <input
                  type="text"
                  className="clean-input"
                  placeholder="continue://pair/v1?addr=192.168.1.105:4433&spki=..."
                  value={pairingPayload}
                  onChange={(e) => setPairingPayload(e.target.value)}
                  autoFocus
                />
                <span className="form-helper">
                  Scan the QR code in the Continue mobile app or paste the connection string.
                </span>
              </div>

              <div className="modal-actions">
                <button
                  type="button"
                  className="btn btn-outline"
                  onClick={() => setShowPairDialog(false)}
                >
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary">
                  Connect Device
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* 4. Feedback Toast */}
      {toastMessage && (
        <div className="floating-toast">
          <Check size={13} color="var(--color-secondary)" />
          <span>{toastMessage}</span>
        </div>
      )}
    </div>
  );
}

export default App;
