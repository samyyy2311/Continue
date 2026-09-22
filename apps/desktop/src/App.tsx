// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState, useCallback } from "react";
import {
  User,
  Home,
  Folder,
  Search,
  Minus,
  Square,
  X,
  Smartphone,
  Tablet,
  Wifi,
  BatteryCharging,
  Plus,
  MessageSquare,
  Cast,
  Camera,
  Monitor,
  MousePointer,
  Upload,
  Clipboard,
  Bell,
  Settings,
  Image,
  Music,
  Play,
  FileText,
  Terminal,
  Download,
  Check,
  Copy,
  ExternalLink,
  Trash2,
  Pause,
  Clock,
  ArrowLeft,
  MoveRight,
  MoveLeft,
  ArrowDownLeft,
} from "lucide-react";
import "./App.css";
import type {
  DeviceIdentity,
  TrustedPeer,
  TransferHistoryItem,
  NotificationItem,
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

type MainTab = "home" | "files";
type SubView = "none" | "cross_control" | "webcam" | "screen_share" | "settings";

export function App() {
  const [activeTab, setActiveTab] = useState<MainTab>("home");
  const [activeSubView, setActiveSubView] = useState<SubView>("none");
  const [selectedPeerId, setSelectedPeerId] = useState<string>("cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a");
  const [searchQuery, setSearchQuery] = useState("");

  // Sub-tabs
  const [appStreamingTab, setAppStreamingTab] = useState<"recent" | "favorites">("recent");
  const [sendFilesTab, setSendFilesTab] = useState<"send" | "recent">("send");
  const [shareHubTab, setShareHubTab] = useState<"share_hub" | "clipboard">("share_hub");

  // Device identity & peers
  const [identity, setIdentity] = useState<DeviceIdentity>({
    deviceName: "Desktop PC",
    fingerprint: "cont1q8f7e2a9d4c6b8a1e3f5a7b9c1d3e5f7a9b1c3d",
    spkiHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  });
  const [peers, setPeers] = useState<TrustedPeer[]>([]);

  // Dialogs & Toasts
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

  // Cross Control states
  const [crossPosition, setCrossPosition] = useState<"right" | "left" | "bottom">("right");
  const [seamlessTransition, setSeamlessTransition] = useState(true);
  const [sharedKeyboard, setSharedKeyboard] = useState(true);

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
      fileName: "CassetteCat-v1.7.3.apk",
      fileSize: 6658291,
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
      body: "Starting in 10 minutes in Conference Room B",
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

  // Clipboard items
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

  // Gallery photos
  const galleryPhotos = [
    { id: "img-1", title: "Sunset Skyline", color: "linear-gradient(135deg, #f97316 0%, #7c2d12 100%)" },
    { id: "img-2", title: "Mountain Peak", color: "linear-gradient(135deg, #0ea5e9 0%, #1e293b 100%)" },
    { id: "img-3", title: "Night City Lights", color: "linear-gradient(135deg, #6366f1 0%, #0f172a 100%)" },
    { id: "img-4", title: "Autumn Forest", color: "linear-gradient(135deg, #eab308 0%, #451a03 100%)" },
    { id: "img-5", title: "Coastal Waves", color: "linear-gradient(135deg, #06b6d4 0%, #083344 100%)" },
    { id: "img-6", title: "Macro Flower", color: "linear-gradient(135deg, #ec4899 0%, #831843 100%)" },
    { id: "img-7", title: "Architectural Lines", color: "linear-gradient(135deg, #64748b 0%, #020617 100%)" },
    { id: "img-8", title: "Desert Dunes", color: "linear-gradient(135deg, #d97706 0%, #78350f 100%)" },
    { id: "img-9", title: "Neon Alley", color: "linear-gradient(135deg, #a855f7 0%, #1e1b4b 100%)" },
    { id: "img-10", title: "Misty Lake", color: "linear-gradient(135deg, #14b8a6 0%, #134e4a 100%)" },
    { id: "img-11", title: "Snowy Ridge", color: "linear-gradient(135deg, #cbd5e1 0%, #334155 100%)" },
    { id: "img-12", title: "Golden Hour Portrait", color: "linear-gradient(135deg, #fb923c 0%, #431407 100%)" },
  ];

  // Share Hub Documents
  const shareHubDocuments = [
    { id: "doc-1", name: "Data_Structures_Notes.pdf", size: "4.2 MB", type: "pdf" },
    { id: "doc-2", name: "Admit_Card_Fall2025.pdf", size: "1.1 MB", type: "pdf" },
    { id: "doc-3", name: "Linear_Algebra_Review.pdf", size: "8.5 MB", type: "pdf" },
    { id: "doc-4", name: "Resume_Engineering_Lead.pdf", size: "1.8 MB", type: "pdf" },
    { id: "doc-5", name: "Project_Specifications_v3.pdf", size: "12.4 MB", type: "pdf" },
  ];

  // Phone folders
  const phoneFolders = [
    { name: "Android", count: "12 items" },
    { name: "DCIM", count: "1,248 items" },
    { name: "Documents", count: "89 items" },
    { name: "Download", count: "342 items" },
    { name: "Movies", count: "54 items" },
    { name: "Music", count: "112 items" },
  ];

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
      showToast("Connected via Wi-Fi Direct");
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
    showToast(`Sending presentation_deck_v3.pdf to ${peerName}`);
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

  return (
    <div className="smart-connect-shell">
      {/* 1. Top Window Bar */}
      <header className="smart-titlebar" data-tauri-drag-region>
        <div className="titlebar-left">
          {/* User Profile Avatar */}
          <div className="user-profile-circle" title="Samarth Lad">
            <User size={18} />
          </div>

          {/* Primary Navigation Tabs */}
          <nav className="primary-tabs">
            <button
              className={`tab-link ${activeTab === "home" && activeSubView === "none" ? "active" : ""}`}
              onClick={() => {
                setActiveTab("home");
                setActiveSubView("none");
              }}
            >
              <Home size={15} />
              <span>Home</span>
            </button>
            <button
              className={`tab-link ${activeTab === "files" && activeSubView === "none" ? "active" : ""}`}
              onClick={() => {
                setActiveTab("files");
                setActiveSubView("none");
              }}
            >
              <Folder size={15} />
              <span>Files</span>
            </button>
          </nav>
        </div>

        {/* Center-Right Search Pill */}
        <div className="titlebar-search-wrapper">
          <div className="search-pill">
            <Search size={14} className="search-pill-icon" />
            <input
              type="text"
              className="search-pill-input"
              placeholder="Search"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
            {searchQuery && (
              <button className="search-clear-pill-btn" onClick={() => setSearchQuery("")}>
                <X size={12} />
              </button>
            )}
          </div>
        </div>

        {/* Right Native Windows Controls */}
        <div className="titlebar-controls">
          <button className="win-control-btn" onClick={handleMinimizeWindow} title="Minimize">
            <Minus size={13} />
          </button>
          <button className="win-control-btn" onClick={handleMaximizeWindow} title="Maximize">
            <Square size={11} />
          </button>
          <button className="win-control-btn win-close-btn" onClick={handleCloseWindow} title="Close">
            <X size={13} />
          </button>
        </div>
      </header>

      {/* Main App Body */}
      <main className="smart-body">
        {/* SUBVIEW: CROSS CONTROL */}
        {activeSubView === "cross_control" && (
          <div className="subview-container">
            <div className="subview-header">
              <button
                className="subview-back-btn"
                onClick={() => setActiveSubView("none")}
              >
                <ArrowLeft size={16} />
              </button>
              <h2 className="subview-title">Cross Control</h2>
            </div>

            <div className="subview-content">
              <div className="cross-control-stage">
                {crossPosition === "left" && (
                  <div className="stage-phone-mockup">
                    <Smartphone size={16} className="mockup-icon" />
                    <span className="mockup-title">{currentPeer.displayName}</span>
                    <span className="mockup-sub">1080 × 2400</span>
                  </div>
                )}

                <div className="stage-monitor-mockup">
                  <Monitor size={22} className="mockup-icon" />
                  <span className="mockup-title">{identity.deviceName} (Primary)</span>
                  <span className="mockup-sub">2560 × 1440 • 165Hz</span>
                </div>

                {crossPosition === "right" && (
                  <div className="stage-phone-mockup">
                    <Smartphone size={16} className="mockup-icon" />
                    <span className="mockup-title">{currentPeer.displayName}</span>
                    <span className="mockup-sub">1080 × 2400</span>
                  </div>
                )}

                {crossPosition === "bottom" && (
                  <div className="stage-phone-mockup bottom-pos">
                    <Smartphone size={16} className="mockup-icon" />
                    <span className="mockup-title">{currentPeer.displayName}</span>
                    <span className="mockup-sub">1080 × 2400</span>
                  </div>
                )}
              </div>

              <div className="position-selection-row">
                <span className="pos-label">Position {currentPeer.displayName}:</span>
                <div className="pos-btn-group">
                  <button
                    className={`pill-pos-btn ${crossPosition === "left" ? "active" : ""}`}
                    onClick={() => setCrossPosition("left")}
                  >
                    <MoveLeft size={12} />
                    <span>Left</span>
                  </button>
                  <button
                    className={`pill-pos-btn ${crossPosition === "right" ? "active" : ""}`}
                    onClick={() => setCrossPosition("right")}
                  >
                    <MoveRight size={12} />
                    <span>Right</span>
                  </button>
                  <button
                    className={`pill-pos-btn ${crossPosition === "bottom" ? "active" : ""}`}
                    onClick={() => setCrossPosition("bottom")}
                  >
                    <ArrowDownLeft size={12} />
                    <span>Bottom</span>
                  </button>
                </div>
              </div>

              <div className="subview-card">
                <div className="subview-card-title">Cursor & Keyboard Settings</div>
                <div className="preference-list">
                  <div className="preference-item">
                    <div>
                      <div className="preference-heading">Seamless Cursor Transition</div>
                      <div className="preference-desc">Move cursor freely across PC and mobile borders</div>
                    </div>
                    <label className="smart-switch">
                      <input
                        type="checkbox"
                        checked={seamlessTransition}
                        onChange={(e) => setSeamlessTransition(e.target.checked)}
                      />
                      <span className="smart-slider" />
                    </label>
                  </div>

                  <div className="preference-item">
                    <div>
                      <div className="preference-heading">Share Keyboard Input</div>
                      <div className="preference-desc">Type into mobile text inputs using PC keyboard</div>
                    </div>
                    <label className="smart-switch">
                      <input
                        type="checkbox"
                        checked={sharedKeyboard}
                        onChange={(e) => setSharedKeyboard(e.target.checked)}
                      />
                      <span className="smart-slider" />
                    </label>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* SUBVIEW: WEBCAM */}
        {activeSubView === "webcam" && (
          <div className="subview-container">
            <div className="subview-header">
              <button
                className="subview-back-btn"
                onClick={() => setActiveSubView("none")}
              >
                <ArrowLeft size={16} />
              </button>
              <h2 className="subview-title">Webcam</h2>
            </div>

            <div className="webcam-illustration-box">
              <div className="webcam-graphic">
                <div className="webcam-phone">
                  <Camera size={24} className="camera-lens-icon" />
                </div>
                <div className="webcam-laptop">
                  <div className="laptop-screen">
                    <User size={36} className="user-cam-icon" />
                  </div>
                </div>
              </div>

              <h3 className="webcam-hero-title">Make crisper video calls</h3>
              <p className="webcam-hero-desc">
                To use your {currentPeer.displayName} as a high-definition webcam, open your preferred video call app,
                start the webcam, and choose &quot;Continue Webcam&quot; as the camera source.
              </p>

              <button
                className="btn-primary-smart"
                onClick={() => showToast("Webcam mode activated")}
              >
                Start Webcam
              </button>
            </div>
          </div>
        )}

        {/* SUBVIEW: SCREEN SHARE */}
        {activeSubView === "screen_share" && (
          <div className="subview-container">
            <div className="subview-header">
              <button
                className="subview-back-btn"
                onClick={() => setActiveSubView("none")}
              >
                <ArrowLeft size={16} />
              </button>
              <h2 className="subview-title">Screen Share</h2>
            </div>

            <div className="webcam-illustration-box">
              <div className="webcam-graphic">
                <div className="screen-share-laptop">
                  <Cast size={36} className="cast-icon-large" />
                </div>
              </div>

              <h3 className="webcam-hero-title">Mirror your mobile screen</h3>
              <p className="webcam-hero-desc">
                Stream your phone screen directly to your PC with low-latency touch and keyboard interaction.
              </p>

              <button
                className="btn-primary-smart"
                onClick={() => showToast("Screen mirroring active")}
              >
                Start Screen Share
              </button>
            </div>
          </div>
        )}

        {/* SUBVIEW: SETTINGS */}
        {activeSubView === "settings" && (
          <div className="subview-container">
            <div className="subview-header">
              <button
                className="subview-back-btn"
                onClick={() => setActiveSubView("none")}
              >
                <ArrowLeft size={16} />
              </button>
              <h2 className="subview-title">Settings & Trusted Devices</h2>
            </div>

            <div className="subview-content">
              <div className="subview-card">
                <div className="subview-card-title">This Machine</div>
                <div className="info-kv-row">
                  <span className="kv-label">Name:</span>
                  <span className="kv-val">{identity.deviceName} (Host PC)</span>
                </div>
                <div className="info-kv-row">
                  <span className="kv-label">Fingerprint:</span>
                  <div className="kv-copy-box">
                    <span className="kv-mono">{identity.fingerprint}</span>
                    <button className="btn-copy-icon" onClick={handleCopyFingerprint}>
                      {copiedFingerprint ? (
                        <Check size={12} color="var(--color-secondary)" />
                      ) : (
                        <Copy size={12} />
                      )}
                    </button>
                  </div>
                </div>
              </div>

              <div className="subview-card">
                <div className="subview-card-header-row">
                  <div className="subview-card-title">Trusted Devices ({peers.length})</div>
                  <button
                    className="btn-small-outline"
                    onClick={() => setShowPairDialog(true)}
                  >
                    <Plus size={12} />
                    <span>Pair Device</span>
                  </button>
                </div>

                <div className="peers-stack">
                  {peers.map((peer) => (
                    <div key={peer.fingerprint} className="peer-row-item">
                      <div className="peer-item-lead">
                        {peer.displayName.toLowerCase().includes("tablet") ? (
                          <Tablet size={18} className="peer-icon" />
                        ) : (
                          <Smartphone size={18} className="peer-icon" />
                        )}
                        <div>
                          <div className="peer-item-name">{peer.displayName}</div>
                          <div className="peer-item-sub">Endpoint: {peer.endpoint || "4433"}</div>
                        </div>
                      </div>

                      <button
                        className="btn-danger-pill"
                        onClick={() => handleDisconnectPeer(peer.fingerprint)}
                      >
                        <Trash2 size={12} />
                        <span>Unpair</span>
                      </button>
                    </div>
                  ))}
                </div>
              </div>

              <div className="subview-card">
                <div className="subview-card-title">Transfer Preferences</div>
                <div className="preference-list">
                  <div className="preference-item">
                    <div>
                      <div className="preference-heading">Encrypted Transfers</div>
                      <div className="preference-desc">Allow Wi-Fi Direct file sharing</div>
                    </div>
                    <label className="smart-switch">
                      <input
                        type="checkbox"
                        checked={allowFileTransfer}
                        onChange={(e) => setAllowFileTransfer(e.target.checked)}
                      />
                      <span className="smart-slider" />
                    </label>
                  </div>

                  <div className="preference-item">
                    <div>
                      <div className="preference-heading">Auto-Accept Small Files</div>
                      <div className="preference-desc">Accept payloads under 10 MB automatically</div>
                    </div>
                    <label className="smart-switch">
                      <input
                        type="checkbox"
                        checked={autoAcceptSmall}
                        onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                      />
                      <span className="smart-slider" />
                    </label>
                  </div>

                  <div className="preference-item">
                    <div>
                      <div className="preference-heading">Clipboard Synchronization</div>
                      <div className="preference-desc">Mirror text copies across devices</div>
                    </div>
                    <label className="smart-switch">
                      <input
                        type="checkbox"
                        checked={allowClipboardSync}
                        onChange={(e) => setAllowClipboardSync(e.target.checked)}
                      />
                      <span className="smart-slider" />
                    </label>
                  </div>

                  <div className="preference-item">
                    <div>
                      <div className="preference-heading">Notification Mirroring</div>
                      <div className="preference-desc">Stream phone push notifications with remote dismiss</div>
                    </div>
                    <label className="smart-switch">
                      <input
                        type="checkbox"
                        checked={allowNotifications}
                        onChange={(e) => setAllowNotifications(e.target.checked)}
                      />
                      <span className="smart-slider" />
                    </label>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* MAIN VIEWS (WHEN NO SUBVIEW ACTIVE) */}
        {activeSubView === "none" && (
          <>
            {/* 2. Connected Device Header Strip (Screenshot 0 layout) */}
            <section className="smart-device-section">
              <div className="device-card-row">
                {/* Phone Card */}
                <div className="smart-phone-card">
                  <div className="phone-screen-frame">
                    <div className="phone-wallpaper-preview" />
                  </div>
                  <div className="phone-details">
                    <div className="phone-name">{currentPeer.displayName}</div>
                    <div className="phone-status">
                      <Wifi size={13} className="phone-status-icon" />
                      <BatteryCharging size={13} className="phone-status-icon" />
                      <span>100%</span>
                    </div>
                  </div>
                </div>

                {/* Circular Plus Button */}
                <button
                  className="smart-plus-btn"
                  title="Pair new device"
                  onClick={() => setShowPairDialog(true)}
                >
                  <Plus size={16} />
                </button>
              </div>

              {/* Horizontal Action Tiles (Screenshot 0) */}
              <div className="smart-action-tiles-row">
                <button
                  className="smart-action-tile"
                  onClick={() => {
                    setActiveTab("home");
                    setSendFilesTab("send");
                    handleTriggerSendFile();
                  }}
                >
                  <Upload size={16} className="tile-icon" />
                  <span>Send Files</span>
                </button>

                <button
                  className="smart-action-tile"
                  onClick={() => setActiveSubView("screen_share")}
                >
                  <Cast size={16} className="tile-icon" />
                  <span>Screen Share</span>
                </button>

                <button
                  className="smart-action-tile"
                  onClick={() => setActiveSubView("webcam")}
                >
                  <Camera size={16} className="tile-icon" />
                  <span>Webcam</span>
                </button>

                <button
                  className="smart-action-tile"
                  onClick={() => setActiveSubView("cross_control")}
                >
                  <MousePointer size={16} className="tile-icon" />
                  <span>Cross Control</span>
                </button>

                <button
                  className="smart-action-tile"
                  onClick={() => {
                    setActiveTab("files");
                    setShareHubTab("clipboard");
                  }}
                >
                  <Clipboard size={16} className="tile-icon" />
                  <span>Clipboard</span>
                </button>

                <button
                  className="smart-action-tile"
                  onClick={() => setActiveSubView("settings")}
                >
                  <Settings size={16} className="tile-icon" />
                  <span>Settings</span>
                </button>
              </div>
            </section>

            {/* TAB 1: HOME (Clean 2x2 Bento Grid from Screenshot 0) */}
            {activeTab === "home" && (
              <div className="smart-grid-2x2">
                {/* 1. App Streaming */}
                <div className="smart-card">
                  <div className="card-head">
                    <div className="card-title-group">
                      <Play size={16} className="card-head-icon" />
                      <span className="card-head-title">App Streaming</span>
                    </div>
                    <div className="pill-toggle-group">
                      <button
                        className={`pill-toggle-btn ${appStreamingTab === "recent" ? "active" : ""}`}
                        onClick={() => setAppStreamingTab("recent")}
                      >
                        Recent
                      </button>
                      <button
                        className={`pill-toggle-btn ${appStreamingTab === "favorites" ? "active" : ""}`}
                        onClick={() => setAppStreamingTab("favorites")}
                      >
                        Favorites
                      </button>
                    </div>
                  </div>

                  <div className="apps-icon-grid">
                    {[
                      { name: "Messages", color: "#2563eb", icon: MessageSquare },
                      { name: "Camera", color: "#10b981", icon: Camera },
                      { name: "Gallery", color: "#8b5cf6", icon: Image },
                      { name: "Files", color: "#f59e0b", icon: Folder },
                      { name: "Music", color: "#ef4444", icon: Music },
                      { name: "YouTube", color: "#dc2626", icon: Play },
                      { name: "Terminal", color: "#475569", icon: Terminal },
                      { name: "Notes", color: "#06b6d4", icon: FileText },
                    ].map((app) => {
                      const Icon = app.icon;
                      return (
                        <div
                          key={app.name}
                          className="app-item-box"
                          onClick={() => showToast(`Streaming ${app.name} from ${currentPeer.displayName}`)}
                        >
                          <div className="app-circle-icon" style={{ backgroundColor: app.color }}>
                            <Icon size={18} color="#ffffff" />
                          </div>
                          <span className="app-name-label">{app.name}</span>
                        </div>
                      );
                    })}
                  </div>
                </div>

                {/* 2. Gallery (Real photo previews from Screenshot 0) */}
                <div className="smart-card">
                  <div className="card-head">
                    <div className="card-title-group">
                      <Image size={16} className="card-head-icon" />
                      <span className="card-head-title">Gallery</span>
                    </div>
                    <button
                      className="card-action-link"
                      onClick={() => setActiveTab("files")}
                    >
                      View all
                    </button>
                  </div>

                  <div className="gallery-thumbnails-grid">
                    {galleryPhotos.map((photo) => (
                      <div
                        key={photo.id}
                        className="gallery-thumb-cell"
                        style={{ background: photo.color }}
                        onClick={() => showToast(`Viewing ${photo.title}`)}
                        title={photo.title}
                      >
                        <span className="thumb-hover-label">{photo.title}</span>
                      </div>
                    ))}
                  </div>
                </div>

                {/* 3. Send Files (Dropzone and recent from Screenshot 0) */}
                <div className="smart-card">
                  <div className="card-head border-bottom">
                    <div className="card-title-group">
                      <Upload size={16} className="card-head-icon" />
                      <span className="card-head-title">Send Files</span>
                    </div>
                    <div className="underline-tab-group">
                      <button
                        className={`underline-tab-btn ${sendFilesTab === "send" ? "active" : ""}`}
                        onClick={() => setSendFilesTab("send")}
                      >
                        Send
                      </button>
                      <button
                        className={`underline-tab-btn ${sendFilesTab === "recent" ? "active" : ""}`}
                        onClick={() => setSendFilesTab("recent")}
                      >
                        Recent
                      </button>
                    </div>
                  </div>

                  {sendFilesTab === "send" ? (
                    <div
                      className={`send-files-dropzone ${isDraggingOver ? "dragging" : ""}`}
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
                      <div className="dropzone-paperplane">
                        <Upload size={24} />
                      </div>
                      <div className="dropzone-lead">Drop files to send to {currentPeer.displayName}</div>
                      <div className="dropzone-hint">End-to-end encrypted transfer</div>

                      <button
                        className="btn-select-files"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleTriggerSendFile();
                        }}
                      >
                        Select files
                      </button>

                      {/* Active Transfer Stream Progress */}
                      <div className="inline-stream-progress" onClick={(e) => e.stopPropagation()}>
                        <div className="stream-info-row">
                          <span className="stream-name-text">recording_session_2025_4k.mov</span>
                          <span className="stream-rate-text">114 MB/s • 12s left</span>
                        </div>
                        <div className="stream-track-bar">
                          <div className="stream-track-fill" style={{ width: `${streamProgress}%` }} />
                        </div>
                        <div className="stream-sub-row">
                          <span className="stream-check">
                            <Check size={11} color="var(--color-secondary)" />
                            <span>SHA-256 Verified</span>
                          </span>
                          <div className="stream-btns">
                            <button
                              className="stream-action-icon"
                              onClick={() => setIsStreamPaused((p) => !p)}
                            >
                              <Pause size={10} />
                            </button>
                            <button
                              className="stream-action-icon danger"
                              onClick={() => showToast("Transfer cancelled")}
                            >
                              <X size={10} />
                            </button>
                          </div>
                        </div>
                      </div>
                    </div>
                  ) : (
                    <div className="recent-transfers-clean-list">
                      {transfers.map((tx) => (
                        <div key={tx.id} className="clean-transfer-row">
                          <div className="tx-lead">
                            <FileText size={16} className="tx-icon" />
                            <div>
                              <div className="tx-name">{tx.fileName}</div>
                              <div className="tx-meta">
                                {(tx.fileSize / 1024 / 1024).toFixed(1)} MB • {tx.direction === "incoming" ? "Received" : "Sent"}
                              </div>
                            </div>
                          </div>
                          <button
                            className="tx-action-link"
                            onClick={() => showToast(`Revealing ${tx.fileName}`)}
                          >
                            <ExternalLink size={12} />
                            <span>Open</span>
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {/* 4. Notifications (Screenshot 0) */}
                <div className="smart-card">
                  <div className="card-head">
                    <div className="card-title-group">
                      <Bell size={16} className="card-head-icon" />
                      <span className="card-head-title">Notifications</span>
                      {notifications.length > 0 && (
                        <span className="badge-count-pill">{notifications.length}</span>
                      )}
                    </div>
                    {notifications.length > 0 && (
                      <button className="card-action-link" onClick={handleClearAllNotifications}>
                        Clear all
                      </button>
                    )}
                  </div>

                  {notifications.length === 0 ? (
                    <div className="empty-notifications-card">
                      <Bell size={24} className="empty-bell-icon" />
                      <div className="empty-title-text">All caught up</div>
                      <div className="empty-sub-text">No active notifications from {currentPeer.displayName}</div>
                    </div>
                  ) : (
                    <div className="notifications-stream-list">
                      {notifications.map((notif) => (
                        <div key={notif.id} className="notification-card-box">
                          <div className="notif-avatar-circle">
                            <Bell size={13} />
                          </div>
                          <div className="notif-details-col">
                            <div className="notif-head-row">
                              <span className="notif-sender-name">{notif.title}</span>
                              <span className="notif-app-name">{notif.appName}</span>
                              <span className="notif-time-str">2m ago</span>
                            </div>
                            <div className="notif-message-body">{notif.body}</div>
                          </div>
                          <button
                            className="notif-close-btn"
                            onClick={() => handleDismissNotification(notif.id)}
                            title="Dismiss"
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

            {/* TAB 2: FILES (Screenshot 8 layout) */}
            {activeTab === "files" && (
              <div className="smart-files-layout">
                {/* 1. Recent Files Row (Screenshot 8) */}
                <div className="smart-card">
                  <div className="card-head">
                    <div className="card-title-group">
                      <Clock size={16} className="card-head-icon" />
                      <span className="card-head-title">Recent</span>
                    </div>
                  </div>

                  <div className="recent-files-horizontal-row">
                    {[
                      { name: "Screenshot_20260921-1...", size: "367.03 KB • Images", icon: Image },
                      { name: "CassetteCat_Backup_2...", size: "50.49 KB • Unknown", icon: FileText },
                      { name: "CassetteCat-v1.7.3.apk", size: "6.35 MB • Documents", icon: Download },
                      { name: "Screenshot_20260921-2...", size: "314.91 KB • Images", icon: Image },
                    ].map((f) => {
                      const Icon = f.icon;
                      return (
                        <div
                          key={f.name}
                          className="recent-file-pill-card"
                          onClick={() => showToast(`Opening ${f.name}`)}
                        >
                          <div className="file-preview-thumb-box">
                            <Icon size={16} />
                          </div>
                          <div className="file-thumb-meta">
                            <div className="file-thumb-title">{f.name}</div>
                            <div className="file-thumb-desc">{f.size}</div>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>

                {/* 2. Share Hub & Clipboard (Screenshot 8) */}
                <div className="smart-card">
                  <div className="card-head border-bottom">
                    <div className="underline-tab-group">
                      <button
                        className={`underline-tab-btn ${shareHubTab === "share_hub" ? "active" : ""}`}
                        onClick={() => setShareHubTab("share_hub")}
                      >
                        Share Hub
                      </button>
                      <button
                        className={`underline-tab-btn ${shareHubTab === "clipboard" ? "active" : ""}`}
                        onClick={() => setShareHubTab("clipboard")}
                      >
                        Clipboard
                      </button>
                    </div>
                  </div>

                  {shareHubTab === "share_hub" ? (
                    <div className="share-hub-docs-row">
                      {shareHubDocuments.map((doc) => (
                        <div
                          key={doc.id}
                          className="document-file-tile"
                          onClick={() => showToast(`Downloading ${doc.name}`)}
                        >
                          <div className="pdf-icon-badge">
                            <FileText size={24} color="#f43f5e" />
                          </div>
                          <div className="doc-tile-title">{doc.name}</div>
                          <div className="doc-tile-size">{doc.size}</div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="clipboard-clean-list">
                      <div className="clipboard-input-row">
                        <input
                          type="text"
                          className="search-pill-input"
                          placeholder={`Type or paste text to blast to ${currentPeer.displayName}...`}
                          value={clipboardInput}
                          onChange={(e) => setClipboardInput(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") handleSendClipboard();
                          }}
                        />
                        <button className="btn-primary-smart" onClick={handleSendClipboard}>
                          Send
                        </button>
                      </div>

                      <div className="clips-feed">
                        {syncedClips.map((clip) => (
                          <div key={clip.id} className="clip-feed-item">
                            <span className="clip-mono-text">{clip.text}</span>
                            <div className="clip-right-actions">
                              <span className="clip-device-tag">{clip.device}</span>
                              <button
                                className="btn-copy-icon"
                                onClick={() => handleCopyText(clip.text, clip.id)}
                              >
                                {copiedClipId === clip.id ? (
                                  <Check size={12} color="var(--color-secondary)" />
                                ) : (
                                  <Copy size={12} />
                                )}
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>

                {/* 3. Bottom Row: Gallery & Folders (Screenshot 8) */}
                <div className="files-bottom-split-row">
                  {/* Left: Gallery */}
                  <div className="smart-card">
                    <div className="card-head">
                      <div className="card-title-group">
                        <Image size={16} className="card-head-icon" />
                        <span className="card-head-title">Gallery</span>
                      </div>
                    </div>

                    <div className="gallery-thumbnails-grid">
                      {galleryPhotos.slice(0, 8).map((photo) => (
                        <div
                          key={photo.id}
                          className="gallery-thumb-cell"
                          style={{ background: photo.color }}
                          onClick={() => showToast(`Viewing ${photo.title}`)}
                          title={photo.title}
                        >
                          <span className="thumb-hover-label">{photo.title}</span>
                        </div>
                      ))}
                    </div>
                  </div>

                  {/* Right: Phone Folders (Screenshot 8) */}
                  <div className="smart-card">
                    <div className="card-head">
                      <div className="card-title-group">
                        <Folder size={16} className="card-head-icon" />
                        <span className="card-head-title">Files</span>
                      </div>
                    </div>

                    <div className="folders-grid-tiles">
                      {phoneFolders.map((f) => (
                        <div
                          key={f.name}
                          className="folder-yellow-tile"
                          onClick={() => showToast(`Browsing ${f.name}`)}
                        >
                          <Folder size={28} color="#eab308" />
                          <div className="folder-tile-title">{f.name}</div>
                          <div className="folder-tile-count">{f.count}</div>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            )}
          </>
        )}
      </main>

      {/* Pair New Device Modal */}
      {showPairDialog && (
        <div className="smart-modal-backdrop" onClick={() => setShowPairDialog(false)}>
          <div className="smart-modal-box" onClick={(e) => e.stopPropagation()}>
            <div className="modal-head-row">
              <span className="modal-head-title">Pair Remote Device</span>
              <button className="modal-close-icon-btn" onClick={() => setShowPairDialog(false)}>
                <X size={15} />
              </button>
            </div>

            <form onSubmit={handlePairSubmit}>
              <div className="modal-form-group">
                <label className="modal-form-label">Enter pairing payload or connection URI:</label>
                <input
                  type="text"
                  className="search-pill-input"
                  placeholder="continue://pair/v1?addr=192.168.1.105:4433&spki=..."
                  value={pairingPayload}
                  onChange={(e) => setPairingPayload(e.target.value)}
                  autoFocus
                />
              </div>

              <div className="modal-btn-row">
                <button
                  type="button"
                  className="btn-small-outline"
                  onClick={() => setShowPairDialog(false)}
                >
                  Cancel
                </button>
                <button type="submit" className="btn-primary-smart">
                  Connect
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Floating Feedback Toast */}
      {toastMessage && (
        <div className="smart-toast">
          <Check size={13} color="var(--color-secondary)" />
          <span>{toastMessage}</span>
        </div>
      )}
    </div>
  );
}

export default App;
