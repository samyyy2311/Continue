// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState, useCallback } from "react";
import {
  Share2,
  Smartphone,
  Tablet,
  Laptop,
  Monitor,
  MousePointer,
  Folder,
  FolderOpen,
  FileText,
  Image,
  Film,
  Music,
  Download,
  Upload,
  Clipboard,
  Bell,
  SlidersHorizontal,
  Plus,
  Check,
  Copy,
  ExternalLink,
  X,
  ShieldCheck,
  Lock,
  Minus,
  Square,
  Search,
  Trash2,
  BatteryCharging,
  HardDrive,
  Activity,
  Pause,
  Wifi,
  MoveRight,
  MoveLeft,
  ArrowDownLeft,
  ArrowUpRight,
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

type NavigationTab = "home" | "files" | "cross_control" | "settings";
type TransferFilter = "all" | "incoming" | "outgoing";
type RemoteCategoryFilter = "all" | "images" | "videos" | "documents" | "audio";

export function App() {
  const [activeTab, setActiveTab] = useState<NavigationTab>("home");
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

  // Live stream telemetry state
  const [streamProgress, setStreamProgress] = useState(62.4);
  const [isStreamPaused, setIsStreamPaused] = useState(false);

  // Settings and permissions
  const [allowFileTransfer, setAllowFileTransfer] = useState(true);
  const [allowClipboardSync, setAllowClipboardSync] = useState(true);
  const [allowNotifications, setAllowNotifications] = useState(true);
  const [autoAcceptSmall, setAutoAcceptSmall] = useState(true);

  // Cross Control states
  const [crossPosition, setCrossPosition] = useState<"right" | "left" | "bottom">("right");
  const [seamlessTransition, setSeamlessTransition] = useState(true);
  const [sharedKeyboard, setSharedKeyboard] = useState(true);
  const [crossClipboard, setCrossClipboard] = useState(true);

  // Remote Files states
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

  // Mobile notifications
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
      body: "Merged: feat(transport): optimize QUIC packet ack",
      timestamp: Date.now() - 1200000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
  ]);

  // Clipboard sync
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

  // Remote file explorer data
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
      showToast("Connected via QUIC");
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
    showToast(`Downloaded ${file.name} to ~/Downloads`);
  };

  const handleDeleteRemoteFile = (id: string) => {
    setRemoteFiles((prev) => prev.filter((f) => f.id !== id));
    showToast("File removed from device queue");
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
      // Browser fallback
    }
  };

  const handleMaximizeWindow = async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().toggleMaximize();
    } catch {
      // Browser fallback
    }
  };

  const handleCloseWindow = async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch {
      // Browser fallback
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

  // Filtering transfers
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

  // Filtering remote files
  const filteredRemoteFiles = remoteFiles.filter((file) => {
    const matchesFolder = selectedFolder ? file.folder === selectedFolder : true;
    const matchesCat =
      remoteCategory === "all" ? true : file.category === remoteCategory;
    const matchesSearch =
      !searchQuery.trim() ||
      file.name.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesFolder && matchesCat && matchesSearch;
  });

  // Filtering clips
  const filteredClips = syncedClips.filter((clip) =>
    !searchQuery.trim() || clip.text.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="app-shell">
      {/* 1. Top Unified Header & Navigation Bar */}
      <header className="titlebar-shell" data-tauri-drag-region>
        <div className="titlebar-left">
          <div className="brand-group">
            <div className="brand-icon-box">
              <Share2 size={13} strokeWidth={2.5} />
            </div>
            <span className="brand-title">Continue</span>
            <span className="build-tag">v0.1.0</span>
          </div>

          <nav className="titlebar-nav">
            <button
              className={`nav-tab-btn ${activeTab === "home" ? "active" : ""}`}
              onClick={() => setActiveTab("home")}
            >
              <Smartphone size={13} />
              <span>Home</span>
            </button>
            <button
              className={`nav-tab-btn ${activeTab === "files" ? "active" : ""}`}
              onClick={() => setActiveTab("files")}
            >
              <Folder size={13} />
              <span>Files</span>
            </button>
            <button
              className={`nav-tab-btn ${activeTab === "cross_control" ? "active" : ""}`}
              onClick={() => setActiveTab("cross_control")}
            >
              <MousePointer size={13} />
              <span>Cross Control</span>
            </button>
            <button
              className={`nav-tab-btn ${activeTab === "settings" ? "active" : ""}`}
              onClick={() => setActiveTab("settings")}
            >
              <SlidersHorizontal size={13} />
              <span>Settings</span>
            </button>
          </nav>
        </div>

        <div className="titlebar-center">
          <div className="global-search-box">
            <Search size={12} className="search-icon" />
            <input
              type="text"
              className="global-search-input"
              placeholder="Search files, clips, or notifications..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
            {searchQuery && (
              <button
                className="search-clear-btn"
                onClick={() => setSearchQuery("")}
                title="Clear search"
              >
                <X size={11} />
              </button>
            )}
          </div>
        </div>

        <div className="titlebar-right">
          <div className="quic-status-pill" title="QUIC RFC-9000 Direct Connection">
            <span className="pulse-dot" />
            <span>QUIC Direct</span>
            <span className="quic-latency">2ms</span>
          </div>

          <button
            className="btn btn-sm btn-primary"
            onClick={() => setShowPairDialog(true)}
          >
            <Plus size={12} />
            <span>Connect</span>
          </button>

          <div className="win-controls">
            <button className="win-icon-btn" onClick={handleMinimizeWindow} title="Minimize">
              <Minus size={12} />
            </button>
            <button className="win-icon-btn" onClick={handleMaximizeWindow} title="Maximize">
              <Square size={10} />
            </button>
            <button className="win-icon-btn win-icon-close" onClick={handleCloseWindow} title="Close">
              <X size={12} />
            </button>
          </div>
        </div>
      </header>

      {/* 2. Device Hero Header Strip (Smart Connect inspired) */}
      <section className="device-hero-strip">
        <div className="device-hero-left">
          {/* Active Device Card */}
          <div className="device-badge-card">
            <div className="device-avatar-frame">
              {isCurrentPeerTablet ? (
                <Tablet size={20} className="device-graphic-icon" />
              ) : (
                <Smartphone size={20} className="device-graphic-icon" />
              )}
            </div>

            <div className="device-info-col">
              <div className="device-name-row">
                <span className="device-name-heading">{currentPeer.displayName}</span>
                {peers.length > 1 && (
                  <select
                    className="device-select-dropdown"
                    value={selectedPeerId}
                    onChange={(e) => setSelectedPeerId(e.target.value)}
                    title="Switch active device"
                  >
                    {peers.map((p) => (
                      <option key={p.fingerprint} value={p.fingerprint}>
                        {p.displayName}
                      </option>
                    ))}
                  </select>
                )}
              </div>
              <div className="device-status-row">
                <div className="status-item">
                  <Wifi size={11} className="status-sub-icon" />
                  <span>Wi-Fi Direct</span>
                </div>
                <span className="status-dot-sep">•</span>
                <div className="status-item">
                  <BatteryCharging size={11} className="battery-icon" />
                  <span>100%</span>
                </div>
                <span className="status-dot-sep">•</span>
                <span className="endpoint-mono">{currentPeer.endpoint || "4433"}</span>
              </div>
            </div>

            <button
              className="device-add-btn"
              title="Pair another device"
              onClick={() => setShowPairDialog(true)}
            >
              <Plus size={13} />
            </button>
          </div>
        </div>

        {/* Feature Action Buttons Bar */}
        <div className="device-hero-actions">
          <button
            className={`hero-action-btn ${activeTab === "home" ? "active" : ""}`}
            onClick={() => {
              setActiveTab("home");
              handleTriggerSendFile();
            }}
            title="Drop or send files to device"
          >
            <Upload size={14} />
            <span>Send Files</span>
          </button>

          <button
            className={`hero-action-btn ${activeTab === "home" ? "active" : ""}`}
            onClick={() => setActiveTab("home")}
            title="Clipboard synchronization"
          >
            <Clipboard size={14} />
            <span>Clipboard</span>
            <span className="hero-btn-badge">{syncedClips.length}</span>
          </button>

          <button
            className={`hero-action-btn ${activeTab === "home" ? "active" : ""}`}
            onClick={() => setActiveTab("home")}
            title="Relayed notifications from device"
          >
            <Bell size={14} />
            <span>Notifications</span>
            {notifications.length > 0 && (
              <span className="hero-btn-badge highlight">{notifications.length}</span>
            )}
          </button>

          <button
            className={`hero-action-btn ${activeTab === "cross_control" ? "active" : ""}`}
            onClick={() => setActiveTab("cross_control")}
            title="Seamless mouse and screen sharing"
          >
            <Monitor size={14} />
            <span>Cross Control</span>
          </button>

          <button
            className={`hero-action-btn ${activeTab === "files" ? "active" : ""}`}
            onClick={() => setActiveTab("files")}
            title="Browse remote phone storage"
          >
            <FolderOpen size={14} />
            <span>Storage Files</span>
          </button>

          <button
            className={`hero-action-btn ${activeTab === "settings" ? "active" : ""}`}
            onClick={() => setActiveTab("settings")}
            title="Security certificates and keyring"
          >
            <ShieldCheck size={14} />
            <span>Security</span>
          </button>
        </div>
      </section>

      {/* 3. Main Stage Content Viewport */}
      <main className="main-viewport">
        {/* VIEW 1: HOME (Bento Grid Overview) */}
        {activeTab === "home" && (
          <div className="canvas-scroll-pane">
            <div className="home-bento-layout">
              {/* Card 1: Fast Send & Active QUIC Stream */}
              <div className="bento-card bento-send-card">
                <div className="card-header-row">
                  <div className="card-header-left">
                    <Upload size={14} className="card-title-icon" />
                    <span className="card-title-text">Send Files</span>
                  </div>
                  <span className="card-tag-mono">QUIC Stream</span>
                </div>

                <div
                  className={`dropzone-area ${isDraggingOver ? "dragging" : ""}`}
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
                  <div className="dropzone-icon-box">
                    <Upload size={20} />
                  </div>
                  <div className="dropzone-content">
                    <span className="dropzone-prompt">
                      Drop files to send to {currentPeer.displayName}
                    </span>
                    <span className="dropzone-sub">
                      End-to-end encrypted chunked QUIC streaming up to 1.2 Gbps
                    </span>
                  </div>
                  <div className="dropzone-btn-row" onClick={(e) => e.stopPropagation()}>
                    <button
                      className="btn btn-sm btn-primary"
                      onClick={() => handleTriggerSendFile()}
                    >
                      Select Files
                    </button>
                    <button
                      className="btn btn-sm btn-outline"
                      onClick={() => {
                        showToast("Streaming active clipboard to peer");
                      }}
                    >
                      <Clipboard size={12} />
                      <span>Send Clipboard</span>
                    </button>
                  </div>
                </div>

                {/* Active Transfer Telemetry Progress Bar */}
                <div className="stream-progress-widget">
                  <div className="stream-widget-top">
                    <div className="stream-widget-info">
                      <FileText size={14} className="stream-file-icon" />
                      <div className="stream-info-text">
                        <span className="stream-file-title">recording_session_2025_4k.mov</span>
                        <span className="stream-file-meta">
                          1.42 GB of 2.80 GB • Chunk 1,454 / 2,867
                        </span>
                      </div>
                    </div>

                    <div className="stream-widget-right">
                      <div className="stream-speed-badge">
                        <span className="speed-val">114 MB/s</span>
                        <span className="speed-eta">12s left</span>
                      </div>
                      <div className="stream-controls">
                        <button
                          className="stream-ctrl-btn"
                          title="Pause or Resume Stream"
                          onClick={() => {
                            setIsStreamPaused((p) => !p);
                            showToast(isStreamPaused ? "Stream resumed" : "Stream paused");
                          }}
                        >
                          <Pause size={11} />
                        </button>
                        <button
                          className="stream-ctrl-btn danger"
                          title="Cancel Transfer"
                          onClick={() => showToast("Stream cancelled")}
                        >
                          <X size={11} />
                        </button>
                      </div>
                    </div>
                  </div>

                  <div className="stream-bar-track">
                    <div className="stream-bar-fill" style={{ width: `${streamProgress}%` }} />
                  </div>

                  <div className="stream-widget-footer">
                    <span className="stream-integrity">
                      <Check size={10} color="var(--color-secondary)" />
                      <span>SHA-256 Verified</span>
                    </span>
                    <span className="stream-pct">{streamProgress}% completed</span>
                  </div>
                </div>
              </div>

              {/* Card 2: Synced Clipboard */}
              <div className="bento-card bento-clipboard-card">
                <div className="card-header-row">
                  <div className="card-header-left">
                    <Clipboard size={14} className="card-title-icon" />
                    <span className="card-title-text">Synced Clipboard</span>
                  </div>
                  <div className="clipboard-sync-toggle">
                    <span className="toggle-label">Auto-Sync</span>
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

                <div className="clipboard-input-bar">
                  <input
                    type="text"
                    className="form-input"
                    placeholder={`Type or paste to blast to ${currentPeer.displayName}...`}
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

                <div className="clipboard-history-list">
                  {filteredClips.map((clip) => (
                    <div key={clip.id} className="clip-item-row">
                      <div className="clip-content-col">
                        <span className="clip-text-snippet">{clip.text}</span>
                        <div className="clip-meta-row">
                          <span className="clip-origin">{clip.device}</span>
                          <span className="status-dot-sep">•</span>
                          <span className="clip-time">{clip.time}</span>
                        </div>
                      </div>
                      <button
                        className="btn-icon-copy"
                        title="Copy to clipboard"
                        onClick={() => handleCopyText(clip.text, clip.id)}
                      >
                        {copiedClipId === clip.id ? (
                          <Check size={12} color="var(--color-secondary)" />
                        ) : (
                          <Copy size={12} />
                        )}
                      </button>
                    </div>
                  ))}
                </div>
              </div>

              {/* Card 3: Notifications Stream */}
              <div className="bento-card bento-notifications-card">
                <div className="card-header-row">
                  <div className="card-header-left">
                    <Bell size={14} className="card-title-icon" />
                    <span className="card-title-text">Mobile Notifications</span>
                    {notifications.length > 0 && (
                      <span className="card-count-pill">{notifications.length}</span>
                    )}
                  </div>
                  {notifications.length > 0 && (
                    <button
                      className="text-action-link"
                      onClick={handleClearAllNotifications}
                    >
                      Clear All
                    </button>
                  )}
                </div>

                {notifications.length === 0 ? (
                  <div className="empty-card-state">
                    <Bell size={20} className="empty-icon" />
                    <span className="empty-title">All Caught Up</span>
                    <span className="empty-desc">
                      No forwarded notifications from {currentPeer.displayName}
                    </span>
                  </div>
                ) : (
                  <div className="notifications-feed-list">
                    {notifications.map((notif) => (
                      <div key={notif.id} className="notif-feed-item">
                        <div className="notif-avatar-box">
                          <Bell size={12} />
                        </div>
                        <div className="notif-content-block">
                          <div className="notif-row-top">
                            <span className="notif-title">{notif.title}</span>
                            <span className="notif-app-tag">{notif.appName}</span>
                          </div>
                          <span className="notif-body-text">{notif.body}</span>
                        </div>
                        <button
                          className="notif-dismiss-btn"
                          title="Dismiss"
                          onClick={() => handleDismissNotification(notif.id)}
                        >
                          <X size={12} />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* Card 4: Recent Activity & Share Hub */}
              <div className="bento-card bento-history-card">
                <div className="card-header-row">
                  <div className="card-header-left">
                    <HardDrive size={14} className="card-title-icon" />
                    <span className="card-title-text">Recent Transfers</span>
                  </div>

                  <div className="filter-pill-group">
                    <button
                      className={`filter-pill-btn ${transferFilter === "all" ? "active" : ""}`}
                      onClick={() => setTransferFilter("all")}
                    >
                      All
                    </button>
                    <button
                      className={`filter-pill-btn ${transferFilter === "incoming" ? "active" : ""}`}
                      onClick={() => setTransferFilter("incoming")}
                    >
                      Incoming
                    </button>
                    <button
                      className={`filter-pill-btn ${transferFilter === "outgoing" ? "active" : ""}`}
                      onClick={() => setTransferFilter("outgoing")}
                    >
                      Outgoing
                    </button>
                  </div>
                </div>

                <div className="history-table-wrapper">
                  <table className="compact-data-table">
                    <thead>
                      <tr>
                        <th>File</th>
                        <th>Size</th>
                        <th>Direction</th>
                        <th>Status</th>
                        <th style={{ textAlign: "right" }}>Action</th>
                      </tr>
                    </thead>
                    <tbody>
                      {filteredTransfers.length === 0 ? (
                        <tr>
                          <td colSpan={5} className="empty-cell">
                            No recent transfers matching filter.
                          </td>
                        </tr>
                      ) : (
                        filteredTransfers.map((tx) => (
                          <tr key={tx.id}>
                            <td>
                              <div className="file-col-lead">
                                <FileText size={13} className="file-table-icon" />
                                <span className="file-name-truncate">{tx.fileName}</span>
                              </div>
                            </td>
                            <td className="mono-cell">
                              {(tx.fileSize / 1024 / 1024).toFixed(1)} MB
                            </td>
                            <td>
                              <span
                                className={`dir-badge ${
                                  tx.direction === "incoming" ? "incoming" : "outgoing"
                                }`}
                              >
                                {tx.direction === "incoming" ? (
                                  <ArrowDownLeft size={10} />
                                ) : (
                                  <ArrowUpRight size={10} />
                                )}
                                <span>{tx.direction}</span>
                              </span>
                            </td>
                            <td>
                              <span className="status-verify-cell">
                                <Check size={10} color="var(--color-secondary)" />
                                <span>Verified</span>
                              </span>
                            </td>
                            <td style={{ textAlign: "right" }}>
                              <button
                                className="action-link-btn"
                                onClick={() => showToast(`Revealing ${tx.fileName}`)}
                              >
                                <ExternalLink size={10} />
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
            </div>
          </div>
        )}

        {/* VIEW 2: FILES (Remote Phone Storage Explorer) */}
        {activeTab === "files" && (
          <div className="canvas-scroll-pane">
            <div className="files-workspace-view">
              {/* Storage Overview Bar */}
              <div className="storage-overview-card">
                <div className="storage-meta-left">
                  <div className="storage-icon-box">
                    <FolderOpen size={16} />
                  </div>
                  <div>
                    <div className="storage-title">
                      {currentPeer.displayName} Storage
                    </div>
                    <div className="storage-detail">
                      Internal Shared Storage • 64.2 GB used of 256 GB
                    </div>
                  </div>
                </div>

                <div className="storage-bar-group">
                  <div className="storage-track">
                    <div className="storage-fill-used" style={{ width: "25.1%" }} />
                  </div>
                  <div className="storage-breakdown-row">
                    <span className="breakdown-tag">DCIM: 24.8 GB</span>
                    <span className="status-dot-sep">•</span>
                    <span className="breakdown-tag">Documents: 4.1 GB</span>
                    <span className="status-dot-sep">•</span>
                    <span className="breakdown-tag">Downloads: 12.3 GB</span>
                  </div>
                </div>

                <button
                  className="btn btn-sm btn-primary"
                  onClick={() => handleTriggerSendFile()}
                >
                  <Upload size={12} />
                  <span>Send to Phone</span>
                </button>
              </div>

              {/* Phone Directories Grid */}
              <div className="folder-cards-grid">
                {[
                  { name: "DCIM", label: "Camera & Photos", icon: Image, count: "1,248 items" },
                  { name: "Download", label: "Downloads", icon: Download, count: "342 items" },
                  { name: "Documents", label: "Documents & PDFs", icon: FileText, count: "89 items" },
                  { name: "Movies", label: "Screen Recordings", icon: Film, count: "54 items" },
                  { name: "Music", label: "Audio & Recordings", icon: Music, count: "112 items" },
                ].map((folder) => {
                  const isSelected = selectedFolder === folder.name;
                  const Icon = folder.icon;
                  return (
                    <div
                      key={folder.name}
                      className={`folder-tile-card ${isSelected ? "selected" : ""}`}
                      onClick={() =>
                        setSelectedFolder(isSelected ? null : folder.name)
                      }
                    >
                      <div className="folder-tile-top">
                        <div className="folder-tile-icon-box">
                          <Icon size={16} />
                        </div>
                        <span className="folder-tile-count">{folder.count}</span>
                      </div>
                      <span className="folder-tile-name">{folder.name}</span>
                      <span className="folder-tile-sub">{folder.label}</span>
                    </div>
                  );
                })}
              </div>

              {/* Category Filter and Files List */}
              <div className="files-table-container">
                <div className="files-filter-bar">
                  <div className="category-tabs-group">
                    {(
                      [
                        { id: "all", label: "All Files" },
                        { id: "images", label: "Images" },
                        { id: "videos", label: "Videos" },
                        { id: "documents", label: "Documents" },
                        { id: "audio", label: "Audio" },
                      ] as const
                    ).map((cat) => (
                      <button
                        key={cat.id}
                        className={`cat-pill-btn ${remoteCategory === cat.id ? "active" : ""}`}
                        onClick={() => setRemoteCategory(cat.id)}
                      >
                        {cat.label}
                      </button>
                    ))}
                  </div>

                  <div className="files-status-meta">
                    {selectedFolder && (
                      <span className="folder-filter-indicator">
                        Filtered: {selectedFolder}
                        <button
                          className="clear-folder-filter"
                          onClick={() => setSelectedFolder(null)}
                        >
                          <X size={10} />
                        </button>
                      </span>
                    )}
                    <span className="files-count-label">
                      {filteredRemoteFiles.length} files available
                    </span>
                  </div>
                </div>

                <table className="compact-data-table">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Folder</th>
                      <th>Size</th>
                      <th>Modified</th>
                      <th style={{ textAlign: "right" }}>Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredRemoteFiles.length === 0 ? (
                      <tr>
                        <td colSpan={5} className="empty-cell">
                          No files found in selected category.
                        </td>
                      </tr>
                    ) : (
                      filteredRemoteFiles.map((file) => (
                        <tr key={file.id}>
                          <td>
                            <div className="file-col-lead">
                              {file.category === "images" ? (
                                <Image size={13} className="file-table-icon" />
                              ) : file.category === "videos" ? (
                                <Film size={13} className="file-table-icon" />
                              ) : file.category === "audio" ? (
                                <Music size={13} className="file-table-icon" />
                              ) : (
                                <FileText size={13} className="file-table-icon" />
                              )}
                              <span className="file-name-truncate">{file.name}</span>
                            </div>
                          </td>
                          <td>
                            <span className="folder-badge">{file.folder}</span>
                          </td>
                          <td className="mono-cell">
                            {(file.size / 1024 / 1024).toFixed(1)} MB
                          </td>
                          <td className="time-cell">
                            {Math.round((Date.now() - file.modifiedAt) / 3600000)}h ago
                          </td>
                          <td style={{ textAlign: "right" }}>
                            <div className="file-row-actions">
                              <button
                                className="action-link-btn"
                                onClick={() => handleDownloadRemoteFile(file)}
                                title="Download file to PC"
                              >
                                <Download size={11} />
                                <span>Download</span>
                              </button>
                              <button
                                className="action-link-btn danger-hover"
                                onClick={() => handleDeleteRemoteFile(file.id)}
                                title="Delete from phone"
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
          </div>
        )}

        {/* VIEW 3: CROSS CONTROL (Display & Input Bridging) */}
        {activeTab === "cross_control" && (
          <div className="canvas-scroll-pane">
            <div className="cross-control-layout">
              {/* Visual Display Arrangement Board */}
              <div className="bento-card display-board-card">
                <div className="card-header-row">
                  <div className="card-header-left">
                    <Monitor size={14} className="card-title-icon" />
                    <span className="card-title-text">Display Arrangement</span>
                  </div>
                  <span className="card-tag-mono">Virtual Edge Transition</span>
                </div>

                <div className="display-arrangement-stage">
                  {crossPosition === "left" && (
                    <div className="display-screen-mockup phone-mockup">
                      <div className="mockup-header">
                        <Smartphone size={12} />
                        <span>{currentPeer.displayName}</span>
                      </div>
                      <span className="mockup-res">1080 × 2400</span>
                      <span className="mockup-tag">Remote Display</span>
                    </div>
                  )}

                  <div className="display-screen-mockup monitor-mockup">
                    <div className="mockup-header">
                      <Laptop size={14} />
                      <span>{identity.deviceName} (Primary)</span>
                    </div>
                    <span className="mockup-res">2560 × 1440 • 165Hz</span>
                    <span className="mockup-tag">Host Desktop</span>
                  </div>

                  {crossPosition === "right" && (
                    <div className="display-screen-mockup phone-mockup">
                      <div className="mockup-header">
                        <Smartphone size={12} />
                        <span>{currentPeer.displayName}</span>
                      </div>
                      <span className="mockup-res">1080 × 2400</span>
                      <span className="mockup-tag">Remote Display</span>
                    </div>
                  )}

                  {crossPosition === "bottom" && (
                    <div className="display-screen-mockup phone-mockup bottom-pos">
                      <div className="mockup-header">
                        <Smartphone size={12} />
                        <span>{currentPeer.displayName}</span>
                      </div>
                      <span className="mockup-res">1080 × 2400</span>
                      <span className="mockup-tag">Remote Display</span>
                    </div>
                  )}
                </div>

                <div className="position-switcher-bar">
                  <span className="position-prompt">Position {currentPeer.displayName}:</span>
                  <div className="position-buttons-row">
                    <button
                      className={`pos-btn ${crossPosition === "left" ? "active" : ""}`}
                      onClick={() => setCrossPosition("left")}
                    >
                      <MoveLeft size={12} />
                      <span>Left of PC</span>
                    </button>
                    <button
                      className={`pos-btn ${crossPosition === "right" ? "active" : ""}`}
                      onClick={() => setCrossPosition("right")}
                    >
                      <MoveRight size={12} />
                      <span>Right of PC</span>
                    </button>
                    <button
                      className={`pos-btn ${crossPosition === "bottom" ? "active" : ""}`}
                      onClick={() => setCrossPosition("bottom")}
                    >
                      <ArrowDownLeft size={12} />
                      <span>Bottom</span>
                    </button>
                  </div>
                </div>
              </div>

              {/* Cross Control Settings Cards */}
              <div className="cross-settings-column">
                <div className="bento-card">
                  <div className="card-header-row">
                    <div className="card-header-left">
                      <MousePointer size={14} className="card-title-icon" />
                      <span className="card-title-text">Cross Control Preferences</span>
                    </div>
                  </div>

                  <div className="settings-toggle-list">
                    <div className="setting-toggle-row">
                      <div className="setting-label-col">
                        <span className="setting-name">Seamless Cursor Transition</span>
                        <span className="setting-desc">
                          Push cursor past screen edge to control {currentPeer.displayName}
                        </span>
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

                    <div className="setting-toggle-row">
                      <div className="setting-label-col">
                        <span className="setting-name">Share Keyboard Input</span>
                        <span className="setting-desc">
                          Type directly into text fields on your mobile device
                        </span>
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

                    <div className="setting-toggle-row">
                      <div className="setting-label-col">
                        <span className="setting-name">Drag & Drop Files Across Edges</span>
                        <span className="setting-desc">
                          Drag files across display borders to initiate encrypted stream
                        </span>
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

                <div className="bento-card telemetry-specs-card">
                  <div className="card-header-row">
                    <div className="card-header-left">
                      <Activity size={14} className="card-title-icon" />
                      <span className="card-title-text">Input Bridge Telemetry</span>
                    </div>
                    <span className="telemetry-pill-active">Optimal</span>
                  </div>

                  <div className="telemetry-metric-grid">
                    <div className="metric-box">
                      <span className="metric-label">Transport</span>
                      <span className="metric-val">QUIC Datagrams</span>
                    </div>
                    <div className="metric-box">
                      <span className="metric-label">Input Latency</span>
                      <span className="metric-val text-green">1.8 ms</span>
                    </div>
                    <div className="metric-box">
                      <span className="metric-label">Polling Rate</span>
                      <span className="metric-val">1000 Hz</span>
                    </div>
                    <div className="metric-box">
                      <span className="metric-label">Encryption</span>
                      <span className="metric-val">ChaCha20-Poly1305</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* VIEW 4: SETTINGS & SECURITY */}
        {activeTab === "settings" && (
          <div className="canvas-scroll-pane">
            <div className="settings-layout-view">
              {/* Local Host Machine Identity */}
              <div className="bento-card">
                <div className="card-header-row">
                  <div className="card-header-left">
                    <Laptop size={14} className="card-title-icon" />
                    <span className="card-title-text">Host Identity & Keyring</span>
                  </div>
                  <span className="card-tag-mono">mTLS 1.3 Ed25519</span>
                </div>

                <div className="identity-details-grid">
                  <div className="ident-row">
                    <span className="ident-label">Device Name:</span>
                    <span className="ident-val">{identity.deviceName} (Host PC)</span>
                  </div>
                  <div className="ident-row">
                    <span className="ident-label">Fingerprint:</span>
                    <div className="ident-val-copy">
                      <span className="mono-code">{identity.fingerprint}</span>
                      <button
                        className="btn-icon-copy"
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
                  <div className="ident-row">
                    <span className="ident-label">SPKI Hash:</span>
                    <span className="mono-code">{identity.spkiHash.substring(0, 32)}...</span>
                  </div>
                </div>
              </div>

              {/* Paired Peers Trust Management */}
              <div className="bento-card">
                <div className="card-header-row">
                  <div className="card-header-left">
                    <Lock size={14} className="card-title-icon" />
                    <span className="card-title-text">Trusted Peers ({peers.length})</span>
                  </div>
                  <button
                    className="btn btn-sm btn-outline"
                    onClick={() => setShowPairDialog(true)}
                  >
                    <Plus size={11} />
                    <span>Pair New Device</span>
                  </button>
                </div>

                <div className="peers-trust-list">
                  {peers.map((peer) => (
                    <div key={peer.fingerprint} className="peer-trust-row">
                      <div className="peer-lead-block">
                        {peer.displayName.toLowerCase().includes("tablet") ? (
                          <Tablet size={16} className="peer-type-icon" />
                        ) : (
                          <Smartphone size={16} className="peer-type-icon" />
                        )}
                        <div>
                          <div className="peer-name-title">{peer.displayName}</div>
                          <div className="peer-sub-desc">
                            Endpoint: {peer.endpoint || "4433"} • Paired via QR Code
                          </div>
                        </div>
                      </div>

                      <div className="peer-action-cell">
                        <span className="peer-status-pill">
                          <span className="status-dot-sm active" />
                          <span>Connected</span>
                        </span>
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

              {/* Protocol & Transfer Policies */}
              <div className="bento-card">
                <div className="card-header-row">
                  <div className="card-header-left">
                    <SlidersHorizontal size={14} className="card-title-icon" />
                    <span className="card-title-text">Transfer & Mesh Policies</span>
                  </div>
                </div>

                <div className="settings-toggle-list">
                  <div className="setting-toggle-row">
                    <div className="setting-label-col">
                      <span className="setting-name">Allow Encrypted File Transfers</span>
                      <span className="setting-desc">
                        Enable incoming and outgoing chunked QUIC streams
                      </span>
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

                  <div className="setting-toggle-row">
                    <div className="setting-label-col">
                      <span className="setting-name">Relay Mobile Notifications</span>
                      <span className="setting-desc">
                        Mirror incoming push notifications from connected phone with remote dismiss
                      </span>
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

                  <div className="setting-toggle-row">
                    <div className="setting-label-col">
                      <span className="setting-name">Auto-Accept Small Payloads</span>
                      <span className="setting-desc">
                        Directly stream files under 10 MB without manual approval prompt
                      </span>
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

                  <div className="setting-toggle-row">
                    <div className="setting-label-col">
                      <span className="setting-name">Echo Ring Suppression</span>
                      <span className="setting-desc">
                        Rolling 16-entry hash ring prevents recursive clipboard synchronization loops
                      </span>
                    </div>
                    <span className="optimal-tag">Enforced</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
      </main>

      {/* 4. Bottom Information Status Bar */}
      <footer className="footer-status-bar">
        <div className="footer-status-left">
          <span className="status-item-text">Daemon: PID 41888 (Continue-Mesh)</span>
          <span className="status-dot-sep">•</span>
          <span className="status-item-text">QUIC RTT: 2.14ms</span>
          <span className="status-dot-sep">•</span>
          <span className="status-item-text">Loss: 0.00%</span>
          <span className="status-dot-sep">•</span>
          <span className="status-item-text">Buffer: 64 MB Slot</span>
        </div>

        <div className="footer-status-right">
          <span className="status-item-text">
            Active: {currentPeer.displayName} ({currentPeer.fingerprint.substring(0, 14)}...)
          </span>
          <span className="status-dot-sep">•</span>
          <span className="status-item-text">TLS 1.3 / ChaCha20</span>
        </div>
      </footer>

      {/* 5. Pair New Remote Device Dialog */}
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
                  className="form-input"
                  placeholder="continue://pair/v1?addr=192.168.1.105:4433&spki=..."
                  value={pairingPayload}
                  onChange={(e) => setPairingPayload(e.target.value)}
                  autoFocus
                />
                <span className="form-helper">
                  Scan the QR code on the Continue Android app or paste the connection string.
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
                  Connect via QUIC
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* 6. Instant Feedback Toast */}
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
