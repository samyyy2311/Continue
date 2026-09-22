// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState, useCallback, useRef } from "react";
import {
  Share2,
  Smartphone,
  Tablet,
  Laptop,
  ArrowDownUp,
  Clipboard,
  Bell,
  Upload,
  Check,
  Copy,
  Folder,
  Pause,
  Play,
  X,
  Minus,
  Square,
  Search,
  Wifi,
  BatteryCharging,
  FileText,
  Image,
  Music,
  Film,
  Plus,
  ArrowDownLeft,
  ArrowUpRight,
  ShieldCheck,
  QrCode,
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
        isConnected: false,
        endpoint: "192.168.1.112:4433",
      },
    ];
  }
}

type MainTab = "home" | "files" | "devices" | "settings";
type TransferSubTab = "send" | "recent";
type FileCategoryFilter = "all" | "images" | "videos" | "documents" | "audio";
type ShareHubSubTab = "documents" | "clipboard";

export default function App() {
  const [activeTab, setActiveTab] = useState<MainTab>("home");
  const [selectedPeerId, setSelectedPeerId] = useState<string>(
    "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a"
  );
  const [searchQuery, setSearchQuery] = useState("");
  const [transferSubTab, setTransferSubTab] = useState<TransferSubTab>("send");
  const [shareHubSubTab, setShareHubSubTab] = useState<ShareHubSubTab>("documents");
  const [fileCategory, setFileCategory] = useState<FileCategoryFilter>("all");
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);

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

  // Transfer streaming progress simulation
  const [streamProgress, setStreamProgress] = useState(64.2);
  const [isStreamPaused, setIsStreamPaused] = useState(false);
  const [showActiveStream, setShowActiveStream] = useState(true);

  // Settings state
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

  // Clipboard items
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

  // Remote phone documents & files
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
    if (!searchQuery.trim()) return true;
    return tx.fileName.toLowerCase().includes(searchQuery.toLowerCase());
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

      {/* 1. Header Bar: Profile avatar + Nav Tabs + Search Bar + Window Controls */}
      <header className="app-header" data-tauri-drag-region>
        <div className="header-left">
          <div className="user-avatar-badge" title="Local Host Identity: Desktop PC">
            <Share2 size={16} />
          </div>

          <nav className="header-nav-tabs">
            <button
              className={`nav-tab-btn ${activeTab === "home" ? "active" : ""}`}
              onClick={() => setActiveTab("home")}
            >
              <span>Home</span>
            </button>
            <button
              className={`nav-tab-btn ${activeTab === "files" ? "active" : ""}`}
              onClick={() => setActiveTab("files")}
            >
              <span>Files</span>
            </button>
            <button
              className={`nav-tab-btn ${activeTab === "devices" ? "active" : ""}`}
              onClick={() => setActiveTab("devices")}
            >
              <span>Devices</span>
            </button>
            <button
              className={`nav-tab-btn ${activeTab === "settings" ? "active" : ""}`}
              onClick={() => setActiveTab("settings")}
            >
              <span>Settings</span>
            </button>
          </nav>
        </div>

        <div className="header-center">
          <div className="search-pill-container">
            <Search size={14} className="search-pill-icon" />
            <input
              type="text"
              className="search-pill-input"
              placeholder="Search files, clips, or alerts..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
            {searchQuery && (
              <button
                className="search-clear-btn"
                onClick={() => setSearchQuery("")}
                title="Clear search"
              >
                <X size={12} />
              </button>
            )}
          </div>
        </div>

        <div className="header-right">
          <div className="window-controls">
            <button className="win-btn" onClick={handleMinimizeWindow} title="Minimize">
              <Minus size={12} />
            </button>
            <button className="win-btn" onClick={handleMaximizeWindow} title="Maximize">
              <Square size={10} />
            </button>
            <button className="win-btn win-close" onClick={handleCloseWindow} title="Close">
              <X size={12} />
            </button>
          </div>
        </div>
      </header>

      {/* 2. Connected Device Card & Quick Actions Bar */}
      <section className="device-action-strip">
        <div className="device-tile-card" onClick={() => setActiveTab("devices")}>
          <div className="device-screen-thumb">
            <Smartphone size={24} className="device-thumb-icon" />
          </div>
          <div className="device-tile-info">
            <div className="device-tile-title">{currentPeer.displayName}</div>
            <div className="device-tile-sub">
              <Wifi size={12} />
              <BatteryCharging size={12} />
              <span>100%</span>
            </div>
          </div>
          <button
            className="device-add-circle"
            onClick={(e) => {
              e.stopPropagation();
              setShowPairDialog(true);
            }}
            title="Pair another device"
          >
            <Plus size={14} />
          </button>
        </div>

        <div className="quick-actions-row">
          <button
            className="action-pill-btn"
            onClick={handleTriggerSendFile}
            title="Send files to phone"
          >
            <Upload size={14} />
            <span>Send Files</span>
          </button>
          <button
            className="action-pill-btn"
            onClick={() => setActiveTab("home")}
            title="Go to shared clipboard"
          >
            <Clipboard size={14} />
            <span>Clipboard</span>
          </button>
          <button
            className="action-pill-btn"
            onClick={() => setActiveTab("home")}
            title="Go to phone notifications"
          >
            <Bell size={14} />
            <span>Notifications</span>
          </button>
          <button
            className="action-pill-btn"
            onClick={() => setActiveTab("files")}
            title="Browse phone files and storage"
          >
            <Folder size={14} />
            <span>Phone Files</span>
          </button>
          <button
            className="action-pill-btn"
            onClick={() => setActiveTab("devices")}
            title="Manage trusted devices and peer security"
          >
            <ShieldCheck size={14} />
            <span>Peer Security</span>
          </button>
        </div>
      </section>

      {/* 3. Main Stage Content Viewport */}
      <main className="app-stage">
        {/* VIEW 1: HOME */}
        {activeTab === "home" && (
          <div className="home-grid-layout">
            {/* Left Column: Send Files Hub */}
            <div className="content-card send-files-card">
              <div className="card-header-row">
                <div className="card-title-group">
                  <ArrowUpRight size={18} className="card-header-icon" />
                  <span className="card-title">Send Files</span>
                </div>
                <div className="subtab-pills">
                  <button
                    className={`subtab-pill ${transferSubTab === "send" ? "active" : ""}`}
                    onClick={() => setTransferSubTab("send")}
                  >
                    Send
                  </button>
                  <button
                    className={`subtab-pill ${transferSubTab === "recent" ? "active" : ""}`}
                    onClick={() => setTransferSubTab("recent")}
                  >
                    Recent
                  </button>
                </div>
              </div>

              {transferSubTab === "send" ? (
                <div className="send-tab-content">
                  {/* Dropzone */}
                  <div
                    className={`dropzone-box ${isDraggingOver ? "dragging" : ""}`}
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
                    <div className="dropzone-icon-ring">
                      <Upload size={24} />
                    </div>
                    <div className="dropzone-heading">
                      Drop files here to send to {currentPeer.displayName}
                    </div>
                    <div className="dropzone-caption">
                      Direct P2P local Wi-Fi transfer up to 1.2 Gbps
                    </div>
                    <button
                      className="btn btn-pill-primary"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleTriggerSendFile();
                      }}
                    >
                      Choose Files
                    </button>
                  </div>

                  {/* Active Transfer Card */}
                  {showActiveStream && (
                    <div className="active-transfer-strip">
                      <div className="active-transfer-lead">
                        <div className="transfer-file-meta">
                          <FileText size={18} className="file-icon-doc" />
                          <div className="transfer-names">
                            <span className="tx-name">recording_session_2025_4k.mov</span>
                            <span className="tx-bytes">1.42 GB of 2.80 GB</span>
                          </div>
                        </div>

                        <div className="transfer-controls">
                          <div className="tx-stats">
                            <span className="tx-speed">114 MB/s</span>
                            <span className="tx-eta">12s left</span>
                          </div>
                          <div className="tx-actions">
                            <button
                              className="ctrl-icon-btn"
                              onClick={() => {
                                setIsStreamPaused((p) => !p);
                                showToast(isStreamPaused ? "Resumed" : "Paused");
                              }}
                              title={isStreamPaused ? "Resume" : "Pause"}
                            >
                              {isStreamPaused ? <Play size={12} /> : <Pause size={12} />}
                            </button>
                            <button
                              className="ctrl-icon-btn danger"
                              onClick={() => {
                                setShowActiveStream(false);
                                showToast("Transfer cancelled");
                              }}
                              title="Cancel"
                            >
                              <X size={12} />
                            </button>
                          </div>
                        </div>
                      </div>

                      <div className="progress-track">
                        <div
                          className="progress-fill"
                          style={{ width: `${streamProgress}%` }}
                        />
                      </div>

                      <div className="transfer-foot">
                        <span className="foot-status">
                          <Check size={12} className="status-check-icon" />
                          SHA-256 Verified
                        </span>
                        <span className="foot-percent">{streamProgress}% completed</span>
                      </div>
                    </div>
                  )}
                </div>
              ) : (
                /* Recent Transfers List */
                <div className="recent-transfers-pane">
                  {filteredTransfers.length === 0 ? (
                    <div className="empty-history-text">No transfer history</div>
                  ) : (
                    <div className="history-items-list">
                      {filteredTransfers.map((tx) => (
                        <div key={tx.id} className="history-item-row">
                          <div className="item-main">
                            {getFileIcon(tx.fileName)}
                            <div className="item-texts">
                              <span className="item-name">{tx.fileName}</span>
                              <span className="item-meta">
                                {formatBytes(tx.fileSize)} •{" "}
                                {tx.direction === "outgoing" ? "Sent" : "Received"}
                              </span>
                            </div>
                          </div>
                          <div className="item-tag-row">
                            <span className={`direction-badge ${tx.direction}`}>
                              {tx.direction === "outgoing" ? (
                                <ArrowUpRight size={11} />
                              ) : (
                                <ArrowDownLeft size={11} />
                              )}
                              {tx.direction === "outgoing" ? "Sent" : "Received"}
                            </span>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Right Column: Shared Clipboard & Phone Notifications */}
            <div className="home-right-stack">
              {/* Card: Shared Clipboard */}
              <div className="content-card clipboard-card">
                <div className="card-header-row">
                  <div className="card-title-group">
                    <Clipboard size={18} className="card-header-icon" />
                    <span className="card-title">Shared Clipboard</span>
                  </div>
                  <span className="pill-status-dot">Live Sync</span>
                </div>

                <div className="clipboard-input-bar">
                  <input
                    type="text"
                    className="clip-input"
                    placeholder="Type or paste text to send to phone..."
                    value={clipboardInput}
                    onChange={(e) => setClipboardInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") handleSendClipboard();
                    }}
                  />
                  <button
                    className="btn btn-pill-primary"
                    onClick={handleSendClipboard}
                  >
                    Send
                  </button>
                </div>

                <div className="synced-clips-list">
                  {filteredClips.map((clip) => (
                    <div key={clip.id} className="synced-clip-row">
                      <div className="clip-content-box">
                        <span className="clip-text-value">{clip.text}</span>
                        <span className="clip-time-tag">
                          {clip.device} • {clip.time}
                        </span>
                      </div>
                      <button
                        className={`clip-copy-btn ${copiedClipId === clip.id ? "copied" : ""}`}
                        onClick={() => handleCopyText(clip.text, clip.id)}
                        title="Copy to clipboard"
                      >
                        {copiedClipId === clip.id ? <Check size={14} /> : <Copy size={14} />}
                      </button>
                    </div>
                  ))}
                </div>
              </div>

              {/* Card: Phone Notifications */}
              <div className="content-card notifications-card">
                <div className="card-header-row">
                  <div className="card-title-group">
                    <Bell size={18} className="card-header-icon" />
                    <span className="card-title">Phone Notifications</span>
                    {notifications.length > 0 && (
                      <span className="counter-pill">{notifications.length}</span>
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

                <div className="notifications-stream">
                  {notifications.length === 0 ? (
                    <div className="empty-notif-box">
                      <Bell size={24} className="empty-notif-icon" />
                      <span>No new notifications</span>
                    </div>
                  ) : (
                    notifications.map((notif) => (
                      <div key={notif.id} className="notification-tile">
                        <div className="notif-header">
                          <span className="notif-app-badge">{notif.appName}</span>
                          <span className="notif-time-ago">Just now</span>
                        </div>
                        <div className="notif-title-line">{notif.title}</div>
                        <div className="notif-body-line">{notif.body}</div>
                        <button
                          className="notif-dismiss-btn"
                          onClick={() => handleDismissNotification(notif.id)}
                          title="Dismiss notification"
                        >
                          <X size={12} />
                        </button>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>
          </div>
        )}

        {/* VIEW 2: FILES (Matches Smart Connect Files Layout) */}
        {activeTab === "files" && (
          <div className="files-view-layout">
            {/* 1. Recent Files Strip */}
            <div className="content-card files-recent-card">
              <div className="card-section-heading">Recent Files</div>
              <div className="recent-files-horizontal-row">
                <div
                  className="recent-file-chip"
                  onClick={() => showToast("Opening Screenshot_20260921-140210.png")}
                >
                  <Image size={18} className="chip-file-icon img" />
                  <div className="chip-info">
                    <span className="chip-name">Screenshot_20260921-140210.png</span>
                    <span className="chip-sub">367 KB • Images</span>
                  </div>
                </div>

                <div
                  className="recent-file-chip"
                  onClick={() => showToast("Opening CassetteCat_Backup_2026.zip")}
                >
                  <FileText size={18} className="chip-file-icon doc" />
                  <div className="chip-info">
                    <span className="chip-name">CassetteCat_Backup_2026.zip</span>
                    <span className="chip-sub">50.5 KB • Archive</span>
                  </div>
                </div>

                <div
                  className="recent-file-chip"
                  onClick={() => showToast("Opening CassetteCat-v1.7.3.apk")}
                >
                  <FileText size={18} className="chip-file-icon doc" />
                  <div className="chip-info">
                    <span className="chip-name">CassetteCat-v1.7.3.apk</span>
                    <span className="chip-sub">6.35 MB • Documents</span>
                  </div>
                </div>

                <div
                  className="recent-file-chip"
                  onClick={() => showToast("Opening LINEAR ALGEBRA.pdf")}
                >
                  <FileText size={18} className="chip-file-icon pdf" />
                  <div className="chip-info">
                    <span className="chip-name">LINEAR ALGEBRA.pdf</span>
                    <span className="chip-sub">8.93 MB • Documents</span>
                  </div>
                </div>
              </div>
            </div>

            {/* 2. Share Hub & Documents Grid */}
            <div className="content-card share-hub-card">
              <div className="card-header-row">
                <div className="hub-nav-tabs">
                  <button
                    className={`hub-tab-btn ${shareHubSubTab === "documents" ? "active" : ""}`}
                    onClick={() => setShareHubSubTab("documents")}
                  >
                    <ArrowDownUp size={14} />
                    <span>Share Hub</span>
                  </button>
                  <button
                    className={`hub-tab-btn ${shareHubSubTab === "clipboard" ? "active" : ""}`}
                    onClick={() => setShareHubSubTab("clipboard")}
                  >
                    <Clipboard size={14} />
                    <span>Clipboard</span>
                  </button>
                </div>

                <button
                  className="btn btn-pill-primary"
                  onClick={handleTriggerSendFile}
                >
                  <Upload size={13} />
                  <span>Send to Phone</span>
                </button>
              </div>

              {shareHubSubTab === "documents" ? (
                <div className="documents-shelf-grid">
                  {filteredPhoneFiles.map((doc) => (
                    <div
                      key={doc.id}
                      className="doc-shelf-item"
                      onClick={() => showToast(`Selected ${doc.name}`)}
                    >
                      <div className="doc-icon-wrapper">
                        {doc.name.endsWith(".pdf") ? (
                          <FileText size={32} className="shelf-icon-pdf" />
                        ) : doc.category === "images" ? (
                          <Image size={32} className="shelf-icon-img" />
                        ) : (
                          <FileText size={32} className="shelf-icon-generic" />
                        )}
                      </div>
                      <span className="doc-shelf-title" title={doc.name}>
                        {doc.name}
                      </span>
                      <span className="doc-shelf-size">{formatBytes(doc.size)}</span>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="synced-clips-list files-clipboard-embed">
                  {filteredClips.map((clip) => (
                    <div key={clip.id} className="synced-clip-row">
                      <div className="clip-content-box">
                        <span className="clip-text-value">{clip.text}</span>
                        <span className="clip-time-tag">
                          {clip.device} • {clip.time}
                        </span>
                      </div>
                      <button
                        className={`clip-copy-btn ${copiedClipId === clip.id ? "copied" : ""}`}
                        onClick={() => handleCopyText(clip.text, clip.id)}
                      >
                        {copiedClipId === clip.id ? <Check size={14} /> : <Copy size={14} />}
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* 3. Phone Folders & Category Filter Bar */}
            <div className="content-card phone-folders-card">
              <div className="card-header-row">
                <div className="card-section-heading">Phone Storage Folders</div>
                {selectedFolder && (
                  <button
                    className="text-action-link"
                    onClick={() => setSelectedFolder(null)}
                  >
                    Reset Filter (Showing {selectedFolder})
                  </button>
                )}
              </div>

              <div className="folders-tiles-row">
                {[
                  { name: "Documents", count: 5 },
                  { name: "DCIM", count: 1 },
                  { name: "Download", count: 2 },
                  { name: "Movies", count: 0 },
                  { name: "Music", count: 0 },
                ].map((f) => (
                  <div
                    key={f.name}
                    className={`folder-tile ${selectedFolder === f.name ? "active" : ""}`}
                    onClick={() =>
                      setSelectedFolder((prev) => (prev === f.name ? null : f.name))
                    }
                  >
                    <Folder size={32} className="folder-tile-icon" />
                    <span className="folder-tile-name">{f.name}</span>
                    <span className="folder-tile-count">{f.count} items</span>
                  </div>
                ))}
              </div>

              <div className="category-pill-strip">
                {[
                  { key: "all", label: "All Files", icon: Folder },
                  { key: "images", label: "Images", icon: Image },
                  { key: "videos", label: "Videos", icon: Film },
                  { key: "documents", label: "Documents", icon: FileText },
                  { key: "audio", label: "Audio", icon: Music },
                ].map((cat) => {
                  const Icon = cat.icon;
                  return (
                    <button
                      key={cat.key}
                      className={`cat-pill-btn ${fileCategory === cat.key ? "active" : ""}`}
                      onClick={() => setFileCategory(cat.key as FileCategoryFilter)}
                    >
                      <Icon size={14} />
                      <span>{cat.label}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          </div>
        )}

        {/* VIEW 3: DEVICES (Matches Smart Connect Cross Control / Device Ecosystem) */}
        {activeTab === "devices" && (
          <div className="devices-view-layout">
            <div className="content-card device-continuity-stage">
              <div className="continuity-header">
                <h2 className="stage-title">Device Continuity</h2>
                <p className="stage-subtitle">
                  Manage your paired devices and verify peer-to-peer security credentials
                </p>
              </div>

              {/* Visual Continuity Map */}
              <div className="device-stage-canvas">
                <div className="stage-device-node host">
                  <Laptop size={44} className="node-icon" />
                  <span className="node-name">Desktop PC (Host)</span>
                  <span className="node-badge">This Computer</span>
                </div>

                <div className="stage-link-track">
                  <div className="link-line active" />
                  <div className="link-status-badge">
                    <ShieldCheck size={14} />
                    <span>TLS 1.3 P2P Direct</span>
                  </div>
                </div>

                <div className="stage-device-node peer">
                  <Smartphone size={44} className="node-icon" />
                  <span className="node-name">{currentPeer.displayName}</span>
                  <span className="node-badge online">Connected</span>
                </div>
              </div>

              <div className="stage-action-center">
                <button
                  className="btn btn-pill-primary"
                  onClick={() => setShowPairDialog(true)}
                >
                  <Plus size={14} />
                  <span>Pair Another Device</span>
                </button>
              </div>
            </div>

            {/* Trusted Peers Table */}
            <div className="content-card trusted-peers-card">
              <div className="card-header-row">
                <div className="card-title-group">
                  <ShieldCheck size={18} className="card-header-icon" />
                  <span className="card-title">Trusted Devices</span>
                </div>
                <button
                  className="btn btn-pill-outline"
                  onClick={() => setShowPairDialog(true)}
                >
                  <QrCode size={13} />
                  <span>Pair with QR</span>
                </button>
              </div>

              <div className="trusted-peers-list">
                {peers.map((peer) => (
                  <div key={peer.fingerprint} className="peer-card-row">
                    <div className="peer-card-left">
                      <div className="peer-device-icon">
                        {peer.displayName.toLowerCase().includes("tablet") ? (
                          <Tablet size={20} />
                        ) : (
                          <Smartphone size={20} />
                        )}
                      </div>
                      <div className="peer-card-meta">
                        <div className="peer-card-name-row">
                          <span className="peer-name">{peer.displayName}</span>
                          <span
                            className={`peer-status-pill ${
                              peer.isConnected ? "online" : "offline"
                            }`}
                          >
                            {peer.isConnected ? "Connected" : "Offline"}
                          </span>
                        </div>
                        <div className="peer-fingerprint-line">
                          <span>Endpoint: {peer.endpoint || "mDNS local"}</span>
                          <span className="dot">•</span>
                          <span>Fingerprint: {peer.fingerprint.substring(0, 16)}...</span>
                        </div>
                      </div>
                    </div>

                    <div className="peer-card-actions">
                      <button
                        className="btn btn-sm btn-outline danger"
                        onClick={() => handleDisconnectPeer(peer.fingerprint)}
                      >
                        Disconnect
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* VIEW 4: SETTINGS */}
        {activeTab === "settings" && (
          <div className="settings-view-layout">
            <div className="content-card settings-card">
              <div className="card-section-heading">General Preferences</div>

              <div className="settings-row">
                <div className="setting-info">
                  <div className="setting-label">Device Display Name</div>
                  <div className="setting-desc">Name visible to nearby peers during discovery</div>
                </div>
                <input
                  type="text"
                  className="setting-text-input"
                  value={identity.deviceName}
                  onChange={(e) =>
                    setIdentity((prev) => ({ ...prev, deviceName: e.target.value }))
                  }
                />
              </div>

              <div className="settings-row">
                <div className="setting-info">
                  <div className="setting-label">File Transfer Downloads</div>
                  <div className="setting-desc">Default directory for incoming received files</div>
                </div>
                <input
                  type="text"
                  className="setting-text-input path"
                  value={downloadPath}
                  onChange={(e) => setDownloadPath(e.target.value)}
                />
              </div>

              <div className="settings-row">
                <div className="setting-info">
                  <div className="setting-label">Auto-accept Small Transfers</div>
                  <div className="setting-desc">Automatically receive files under 50 MB without prompting</div>
                </div>
                <label className="toggle-switch">
                  <input
                    type="checkbox"
                    checked={autoAcceptSmall}
                    onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                  />
                  <span className="slider round" />
                </label>
              </div>
            </div>

            <div className="content-card settings-card">
              <div className="card-section-heading">Continuity Features</div>

              <div className="settings-row">
                <div className="setting-info">
                  <div className="setting-label">Peer File Transfer</div>
                  <div className="setting-desc">Enable high-speed local QUIC file streaming</div>
                </div>
                <label className="toggle-switch">
                  <input
                    type="checkbox"
                    checked={allowFileTransfer}
                    onChange={(e) => setAllowFileTransfer(e.target.checked)}
                  />
                  <span className="slider round" />
                </label>
              </div>

              <div className="settings-row">
                <div className="setting-info">
                  <div className="setting-label">Real-time Shared Clipboard</div>
                  <div className="setting-desc">Synchronize copied text instantly with active peer</div>
                </div>
                <label className="toggle-switch">
                  <input
                    type="checkbox"
                    checked={allowClipboardSync}
                    onChange={(e) => setAllowClipboardSync(e.target.checked)}
                  />
                  <span className="slider round" />
                </label>
              </div>

              <div className="settings-row">
                <div className="setting-info">
                  <div className="setting-label">Forward Phone Notifications</div>
                  <div className="setting-desc">Display incoming phone alerts on desktop</div>
                </div>
                <label className="toggle-switch">
                  <input
                    type="checkbox"
                    checked={allowNotifications}
                    onChange={(e) => setAllowNotifications(e.target.checked)}
                  />
                  <span className="slider round" />
                </label>
              </div>
            </div>

            <div className="content-card settings-card">
              <div className="card-section-heading">Identity and Cryptographic Keys</div>

              <div className="identity-key-box">
                <div className="key-heading-row">
                  <span className="key-label">Ed25519 Device Fingerprint</span>
                  <button
                    className="key-copy-btn"
                    onClick={handleCopyFingerprint}
                  >
                    {copiedFingerprint ? <Check size={13} /> : <Copy size={13} />}
                    <span>{copiedFingerprint ? "Copied" : "Copy"}</span>
                  </button>
                </div>
                <div className="key-value-mono">{identity.fingerprint}</div>
              </div>

              <div className="identity-key-box">
                <div className="key-heading-row">
                  <span className="key-label">SPKI Certificate SHA-256</span>
                </div>
                <div className="key-value-mono">{identity.spkiHash}</div>
              </div>
            </div>
          </div>
        )}
      </main>

      {/* 4. Pairing Dialog Modal */}
      {showPairDialog && (
        <div className="modal-backdrop" onClick={() => setShowPairDialog(false)}>
          <div className="modal-container" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <div className="modal-title-row">
                <QrCode size={18} className="modal-title-icon" />
                <span className="modal-title">Pair New Device</span>
              </div>
              <button
                className="modal-close-btn"
                onClick={() => setShowPairDialog(false)}
              >
                <X size={16} />
              </button>
            </div>

            <div className="modal-body">
              <div className="qr-preview-card">
                <div className="qr-fake-visual">
                  <QrCode size={120} strokeWidth={1.5} />
                </div>
                <p className="qr-instruction">
                  Scan this QR code with your mobile Continue app to verify mutual TLS keys
                </p>
              </div>

              <div className="or-separator">
                <span>OR ENTER PAIRING CODE</span>
              </div>

              <form onSubmit={handlePairSubmit} className="pair-form">
                <input
                  type="text"
                  className="pair-input"
                  placeholder="Paste QR payload or device URI..."
                  value={pairingPayload}
                  onChange={(e) => setPairingPayload(e.target.value)}
                />
                <div className="modal-actions">
                  <button
                    type="button"
                    className="btn btn-pill-outline"
                    onClick={() => setShowPairDialog(false)}
                  >
                    Cancel
                  </button>
                  <button type="submit" className="btn btn-pill-primary">
                    Connect Device
                  </button>
                </div>
              </form>
            </div>
          </div>
        </div>
      )}

      {/* 5. Toast Notification Banner */}
      {toastMessage && (
        <div className="toast-notification">
          <Check size={14} className="toast-icon" />
          <span>{toastMessage}</span>
        </div>
      )}
    </div>
  );
}

export { App };
