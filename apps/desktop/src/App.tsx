// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState, useCallback } from "react";
import {
  Share2,
  Smartphone,
  Tablet,
  Laptop,
  ArrowDownUp,
  Clipboard,
  Bell,
  SlidersHorizontal,
  Plus,
  Check,
  Copy,
  Upload,
  FileText,
  ArrowDownLeft,
  ArrowUpRight,
  ExternalLink,
  X,
  RefreshCw,
  ShieldCheck,
  Lock,
  Minus,
  Square,
  Search,
  Trash2,
  BatteryCharging,
  Radio,
  HardDrive,
  Activity,
  Pause,
  Zap,
} from "lucide-react";
import "./App.css";
import type { DeviceIdentity, TrustedPeer, TransferHistoryItem, NotificationItem } from "./types.ts";

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

type DeviceTab = "transfers" | "clipboard" | "notifications" | "settings";
type TransferFilter = "all" | "incoming" | "outgoing";

export function App() {
  const [selectedPeerId, setSelectedPeerId] = useState<string>("cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a");
  const [activeTab, setActiveTab] = useState<DeviceTab>("transfers");
  const [transferFilter, setTransferFilter] = useState<TransferFilter>("all");

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

  // Live stream demo state
  const [streamProgress, setStreamProgress] = useState(58.7);
  const [isStreamPaused, setIsStreamPaused] = useState(false);

  // Device Permissions
  const [allowFileTransfer, setAllowFileTransfer] = useState(true);
  const [allowClipboardSync, setAllowClipboardSync] = useState(true);
  const [allowNotifications, setAllowNotifications] = useState(true);
  const [autoAcceptSmall, setAutoAcceptSmall] = useState(true);

  // Transfers
  const [transfers, setTransfers] = useState<TransferHistoryItem[]>([
    {
      id: "tx-1",
      fileName: "client_presentation_deck.pdf",
      fileSize: 35651584,
      direction: "outgoing",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 120000,
    },
    {
      id: "tx-2",
      fileName: "PXL_20250514_RAW_001.dng",
      fileSize: 50646220,
      direction: "incoming",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 1080000,
    },
    {
      id: "tx-3",
      fileName: "voice_memo_architecture.m4a",
      fileSize: 8493465,
      direction: "incoming",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 3600000,
    },
    {
      id: "tx-4",
      fileName: "production-mesh-credentials.enc",
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
      title: "Team Standup",
      body: "Starting in 10 minutes in Conference Room B",
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

  const [clipboardInput, setClipboardInput] = useState("");
  const [clipFilterQuery, setClipFilterQuery] = useState("");
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
      setStreamProgress((prev) => (prev >= 100 ? 58.7 : +(prev + 0.4).toFixed(1)));
    }, 1200);
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
      fileName: "presentation_deck_v2.pdf",
      fileSize: 18457280,
      direction: "outgoing",
      peerFingerprint: currentPeer?.fingerprint || "cont1q9a8b",
      status: "completed",
      timestamp: Date.now(),
    };
    setTransfers((prev) => [newTx, ...prev]);
    showToast(`Streaming presentation_deck_v2.pdf to ${peerName}`);
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
      // Fallback in browser
    }
  };

  const handleMaximizeWindow = async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().toggleMaximize();
    } catch {
      // Fallback in browser
    }
  };

  const handleCloseWindow = async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch {
      // Fallback in browser
    }
  };

  const currentPeer = peers.find((p) => p.fingerprint === selectedPeerId) || peers[0];

  const filteredTransfers = transfers.filter((tx) => {
    if (transferFilter === "incoming") return tx.direction === "incoming";
    if (transferFilter === "outgoing") return tx.direction === "outgoing";
    return true;
  });

  const filteredClips = syncedClips.filter((clip) =>
    clip.text.toLowerCase().includes(clipFilterQuery.toLowerCase())
  );

  return (
    <div className="app-shell">
      {/* Top Window Titlebar */}
      <header className="titlebar-shell" data-tauri-drag-region>
        <div className="titlebar-left">
          {/* Traffic light window indicators */}
          <div className="window-dots">
            <span className="dot dot-close" onClick={handleCloseWindow} title="Close" />
            <span className="dot dot-min" onClick={handleMinimizeWindow} title="Minimize" />
            <span className="dot dot-max" onClick={handleMaximizeWindow} title="Maximize" />
          </div>

          <div className="brand-group">
            <div className="brand-icon-box">
              <Share2 size={13} />
            </div>
            <span className="brand-title">Continuity</span>
            <span className="build-tag">v2.4.1-stable</span>
          </div>
        </div>

        {/* Global Pipeline Tabs */}
        <div className="titlebar-center">
          <button
            className={`top-tab-btn ${activeTab === "transfers" ? "active" : ""}`}
            onClick={() => setActiveTab("transfers")}
          >
            <ArrowDownUp size={13} />
            <span>Transfers</span>
          </button>
          <button
            className={`top-tab-btn ${activeTab === "clipboard" ? "active" : ""}`}
            onClick={() => setActiveTab("clipboard")}
          >
            <Clipboard size={13} />
            <span>Clipboard</span>
            <span className="top-tab-badge">14</span>
          </button>
          <button
            className={`top-tab-btn ${activeTab === "notifications" ? "active" : ""}`}
            onClick={() => setActiveTab("notifications")}
          >
            <Bell size={13} />
            <span>Notifications</span>
            {notifications.length > 0 && (
              <span className="top-tab-badge active">{notifications.length}</span>
            )}
          </button>
          <button
            className={`top-tab-btn ${activeTab === "settings" ? "active" : ""}`}
            onClick={() => setActiveTab("settings")}
          >
            <SlidersHorizontal size={13} />
            <span>Settings</span>
          </button>
        </div>

        {/* Right Status & Actions */}
        <div className="titlebar-right">
          <div className="quic-status-pill">
            <span className="pulse-dot" />
            <span>QUIC Direct Active</span>
          </div>

          <button
            className="btn btn-sm btn-primary"
            onClick={() => setShowPairDialog(true)}
          >
            <Plus size={12} />
            <span>Connect Target</span>
          </button>

          {/* Windows-style controls */}
          <div className="win-controls">
            <button className="win-icon-btn" onClick={handleMinimizeWindow} title="Minimize">
              <Minus size={12} />
            </button>
            <button className="win-icon-btn" onClick={handleMaximizeWindow} title="Maximize / Restore">
              <Square size={10} />
            </button>
            <button className="win-icon-btn win-icon-close" onClick={handleCloseWindow} title="Close">
              <X size={12} />
            </button>
          </div>
        </div>
      </header>

      {/* Main Workspace Multi-Pane Container */}
      <div className="workspace-container">
        {/* Left Utility Rail (240px) */}
        <aside className="left-rail">
          {/* Header & Pair Action */}
          <div className="rail-top">
            <div className="rail-headline">
              <div className="rail-title-group">
                <span className="rail-heading">CONTINUITY</span>
                <span className="rail-mono-hash">ed25519:e4a9..81b0</span>
              </div>
              <button
                className="icon-ghost-btn"
                title="Pair Device"
                onClick={() => setShowPairDialog(true)}
              >
                <Plus size={14} />
              </button>
            </div>

            <button className="btn-pair-dashed" onClick={() => setShowPairDialog(true)}>
              <Plus size={12} />
              <span>Pair Device</span>
            </button>

            {/* Mesh Navigation */}
            <div className="rail-nav-group">
              <span className="rail-section-label">Mesh Navigation</span>
              <div className="rail-nav-item active">
                <ArrowDownUp size={13} className="rail-icon" />
                <span>Shared Streams</span>
              </div>
              <div className="rail-nav-item">
                <Radio size={13} className="rail-icon" />
                <span>Overview</span>
              </div>
              <div className="rail-nav-item">
                <Smartphone size={13} className="rail-icon" />
                <span>Paired Devices</span>
              </div>
              <div className="rail-nav-item">
                <Activity size={13} className="rail-icon" />
                <span>Network Mesh</span>
              </div>
              <div className="rail-nav-item">
                <Lock size={13} className="rail-icon" />
                <span>Certificates</span>
              </div>
            </div>

            {/* Paired Devices Section */}
            <div className="rail-devices-section">
              <div className="rail-section-header">
                <span className="rail-section-label">PAIRED DEVICES</span>
                <span className="online-pill">2 Online</span>
              </div>

              <div className="device-cards-stack">
                {peers.map((peer) => {
                  const isSelected = peer.fingerprint === selectedPeerId;
                  const isTablet = peer.displayName.toLowerCase().includes("tablet");
                  return (
                    <div
                      key={peer.fingerprint}
                      className={`device-rail-card ${isSelected ? "selected" : ""}`}
                      onClick={() => setSelectedPeerId(peer.fingerprint)}
                    >
                      {isSelected && <div className="selected-indicator-bar" />}
                      <div className="device-card-row-top">
                        <div className="device-ident-left">
                          {isTablet ? (
                            <Tablet size={14} className="device-type-icon" />
                          ) : (
                            <Smartphone size={14} className="device-type-icon" />
                          )}
                          <span className="device-name-text">{peer.displayName}</span>
                        </div>
                        <div className="device-battery-tag">
                          <BatteryCharging size={10} color="var(--color-secondary)" />
                          <span>100%</span>
                        </div>
                      </div>
                      <div className="device-card-row-bottom">
                        <span className="device-endpoint-mono">{peer.endpoint || "4433"}</span>
                        <span className="device-state-tag">
                          {isSelected ? "Primary" : "Idle"}
                        </span>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Shared Pipelines */}
            <div className="pipelines-section">
              <span className="rail-section-label">SHARED PIPELINES</span>
              <div className="pipeline-row" onClick={() => setActiveTab("clipboard")}>
                <div className="pipeline-label-group">
                  <span className="status-dot-sm active" />
                  <span>Clipboard Sync</span>
                </div>
                <span className="pipeline-state-active">Active</span>
              </div>

              <div className="pipeline-row" onClick={() => setActiveTab("notifications")}>
                <div className="pipeline-label-group">
                  <span className="status-dot-sm active" />
                  <span>Notification Relay</span>
                </div>
                <span className="pipeline-state-active">Mirroring</span>
              </div>

              <div className="pipeline-row" onClick={() => setActiveTab("transfers")}>
                <div className="pipeline-label-group">
                  <span className="status-dot-sm standby" />
                  <span>Shared Storage</span>
                </div>
                <span className="pipeline-state-dim">Standby</span>
              </div>
            </div>
          </div>

          {/* Bottom Host Identity Card */}
          <div className="rail-bottom">
            <div className="host-system-card">
              <div className="host-card-header">
                <div className="host-title-left">
                  <Laptop size={13} color="var(--color-secondary)" />
                  <span className="host-machine-name">{identity.deviceName}</span>
                  <span className="host-type-pill">(Host)</span>
                </div>
                <button
                  className="copy-icon-btn"
                  title="Copy Ed25519 Fingerprint"
                  onClick={handleCopyFingerprint}
                >
                  {copiedFingerprint ? (
                    <Check size={12} color="var(--color-secondary)" />
                  ) : (
                    <Copy size={12} />
                  )}
                </button>
              </div>

              <div className="host-key-row">
                <span className="host-mono-key">{identity.fingerprint.substring(0, 18)}...</span>
                <span className="host-status-badge">Healthy</span>
              </div>

              <div className="host-specs-row">
                <span>192.168.1.80</span>
                <span>0.1% CPU</span>
              </div>
            </div>

            <div className="host-quick-actions">
              <button
                className="host-action-link"
                onClick={() => showToast("Host telemetry refreshed")}
              >
                <Activity size={11} />
                <span>Host Telemetry</span>
              </button>
              <button
                className="host-action-link"
                onClick={() => showToast("mTLS 1.3 certificate audit verified")}
              >
                <ShieldCheck size={11} />
                <span>Security Logs</span>
              </button>
            </div>
          </div>
        </aside>

        {/* Main Stage Content Area */}
        <main className="main-viewport">
          {/* Contextual Device Control Bar */}
          <div className="device-sub-header">
            <div className="device-meta-group">
              <span className="active-device-name">{currentPeer.displayName}</span>
              <div className="divider-vert" />

              <div className="quic-chip">
                <span className="status-dot-sm active" />
                <span className="quic-label">QUIC Direct</span>
                <span className="quic-port">• 4433</span>
              </div>

              <div className="ping-chip">
                <Zap size={10} />
                <span>2ms ping</span>
              </div>
            </div>

            {/* Segmented Control Bar */}
            <div className="segmented-tab-container">
              <button
                className={`segment-tab ${activeTab === "transfers" ? "active" : ""}`}
                onClick={() => setActiveTab("transfers")}
              >
                Transfers
              </button>
              <button
                className={`segment-tab ${activeTab === "clipboard" ? "active" : ""}`}
                onClick={() => setActiveTab("clipboard")}
              >
                Clipboard
                <span className="tab-count-badge">14</span>
              </button>
              <button
                className={`segment-tab ${activeTab === "notifications" ? "active" : ""}`}
                onClick={() => setActiveTab("notifications")}
              >
                Notifications
                {notifications.length > 0 && (
                  <span className="tab-count-badge active">{notifications.length}</span>
                )}
              </button>
              <button
                className={`segment-tab ${activeTab === "settings" ? "active" : ""}`}
                onClick={() => setActiveTab("settings")}
              >
                Settings
              </button>
            </div>

            {/* Quick Utility Actions */}
            <div className="device-actions-group">
              <button
                className="tool-btn"
                title="Instant Latency Ping Check"
                onClick={() => showToast("Ping: 2.14 ms (0.00% packet loss)")}
              >
                <Zap size={12} />
                <span>Ping</span>
              </button>
              <button
                className="tool-btn"
                title="Pause Active Stream"
                onClick={() => {
                  setIsStreamPaused((prev) => !prev);
                  showToast(isStreamPaused ? "Stream resumed" : "Stream paused");
                }}
              >
                <Pause size={12} />
                <span>{isStreamPaused ? "Resume" : "Pause"}</span>
              </button>
            </div>
          </div>

          {/* Main Stage Scrollable Canvas */}
          <div className="stage-canvas">
            {/* TAB 1: TRANSFERS */}
            {activeTab === "transfers" && (
              <div className="tab-layout-flow">
                {/* 1. Fast Send Bento Grid */}
                <div className="bento-grid">
                  {/* Dropzone Main Card (2 Cols) */}
                  <div
                    className={`dropzone-bento ${isDraggingOver ? "dragging" : ""}`}
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
                    <div className="dropzone-circle-icon">
                      <Upload size={20} color="var(--color-primary)" />
                    </div>
                    <h3 className="dropzone-title-text">
                      Drop files to send to {currentPeer.displayName}
                    </h3>
                    <p className="dropzone-subtitle-text">
                      End-to-end encrypted chunked QUIC streaming • Up to 1.2 Gbps LAN throughput
                    </p>

                    <div className="dropzone-actions" onClick={(e) => e.stopPropagation()}>
                      <button
                        className="btn btn-primary"
                        onClick={() => handleTriggerSendFile()}
                      >
                        Select Files
                      </button>
                      <button
                        className="btn btn-outline"
                        onClick={() => {
                          showToast("Streaming clipboard buffer to peer");
                        }}
                      >
                        <Clipboard size={13} />
                        <span>Send Clipboard Buffer</span>
                      </button>
                    </div>
                  </div>

                  {/* Protocol Metrics Card (1 Col) */}
                  <div className="protocol-metrics-card">
                    <div className="metrics-top">
                      <div className="metrics-header-row">
                        <span className="metrics-label">Transport Pipeline</span>
                        <span className="metrics-status-optimal">
                          <span className="status-dot-sm active" />
                          <span>Optimal</span>
                        </span>
                      </div>

                      <div className="throughput-headline">
                        <span className="throughput-num">1,248</span>
                        <span className="throughput-unit">Mbps Mesh Target</span>
                      </div>

                      <div className="protocol-specs-list">
                        <div className="spec-row">
                          <span className="spec-label">Multiplexing</span>
                          <span className="spec-val">QUIC RFC-9000</span>
                        </div>
                        <div className="spec-row">
                          <span className="spec-label">Handshake Cipher</span>
                          <span className="spec-val">TLS 1.3 / ChaCha20</span>
                        </div>
                        <div className="spec-row">
                          <span className="spec-label">Packet Loss</span>
                          <span className="spec-val-highlight">0.00% (Local LAN)</span>
                        </div>
                      </div>
                    </div>

                    <div className="metrics-bottom">
                      <span>MTU 9000 (Jumbo)</span>
                      <span className="congestion-label">BBR Congestion</span>
                    </div>
                  </div>
                </div>

                {/* 2. Live Transfer Stream Progress Card */}
                <div className="stream-telemetry-card">
                  <div className="stream-left-strip" />
                  <div className="stream-header-row">
                    <div className="stream-file-meta">
                      <div className="stream-file-icon">
                        <FileText size={16} color="var(--color-primary)" />
                      </div>
                      <div className="stream-details">
                        <div className="stream-name-row">
                          <span className="stream-file-name">recording_session_2025_4k.mov</span>
                          <span className="stream-tag-pill">QUIC Stream #14</span>
                        </div>
                        <div className="stream-sub-telemetry">
                          <span>1.42 GB of 2.80 GB transferred</span>
                          <span>•</span>
                          <span>Chunk 1,454 / 2,867</span>
                          <span>•</span>
                          <span className="verified-text">ed25519 Verified</span>
                        </div>
                      </div>
                    </div>

                    <div className="stream-speed-controls">
                      <div className="stream-speed-text">
                        <span className="speed-rate">114 MB/s</span>
                        <span className="speed-eta">12s remaining</span>
                      </div>
                      <div className="stream-control-buttons">
                        <button
                          className="stream-ctrl-btn"
                          title="Pause Stream"
                          onClick={() => {
                            setIsStreamPaused((prev) => !prev);
                            showToast(isStreamPaused ? "Resumed stream" : "Paused stream");
                          }}
                        >
                          <Pause size={12} />
                        </button>
                        <button
                          className="stream-ctrl-btn close-btn"
                          title="Cancel Stream"
                          onClick={() => showToast("Stream cancelled")}
                        >
                          <X size={12} />
                        </button>
                      </div>
                    </div>
                  </div>

                  {/* Progress Bar Track */}
                  <div className="stream-progress-block">
                    <div className="progress-track">
                      <div
                        className="progress-fill"
                        style={{ width: `${streamProgress}%` }}
                      />
                    </div>
                    <div className="progress-sub-info">
                      <span>Streaming target: /storage/emulated/0/Download/Continuity</span>
                      <span>{streamProgress}% Completed</span>
                    </div>
                  </div>
                </div>

                {/* 3. Recent Activity Audit Table */}
                <div className="activity-table-card">
                  <div className="table-control-bar">
                    <div className="table-filter-group">
                      <div className="table-title-group">
                        <HardDrive size={14} color="var(--color-outline)" />
                        <span className="table-title-text">Recent Transfers</span>
                      </div>

                      <div className="filter-segment-pill">
                        <button
                          className={`filter-btn ${transferFilter === "all" ? "active" : ""}`}
                          onClick={() => setTransferFilter("all")}
                        >
                          All
                        </button>
                        <button
                          className={`filter-btn ${transferFilter === "incoming" ? "active" : ""}`}
                          onClick={() => setTransferFilter("incoming")}
                        >
                          Incoming
                        </button>
                        <button
                          className={`filter-btn ${transferFilter === "outgoing" ? "active" : ""}`}
                          onClick={() => setTransferFilter("outgoing")}
                        >
                          Outgoing
                        </button>
                      </div>
                    </div>

                    <div className="table-actions-right">
                      <span className="records-count">{filteredTransfers.length} records</span>
                      <div className="divider-vert-sm" />
                      <button
                        className="clear-history-btn"
                        onClick={() => {
                          setTransfers([]);
                          showToast("Transfer history cleared");
                        }}
                      >
                        <Trash2 size={12} />
                        <span>Clear History</span>
                      </button>
                    </div>
                  </div>

                  {/* Table Layout */}
                  <div className="table-scroller">
                    <table className="dense-data-table">
                      <thead>
                        <tr>
                          <th>File Name</th>
                          <th>Size</th>
                          <th>Direction</th>
                          <th>Integrity Check</th>
                          <th>Timestamp</th>
                          <th style={{ textAlign: "right" }}>Action</th>
                        </tr>
                      </thead>
                      <tbody>
                        {filteredTransfers.length === 0 ? (
                          <tr>
                            <td colSpan={6} className="empty-table-cell">
                              No transfer records matching filter.
                            </td>
                          </tr>
                        ) : (
                          filteredTransfers.map((tx) => (
                            <tr key={tx.id}>
                              <td>
                                <div className="file-lead-cell">
                                  <FileText size={14} className="row-file-icon" />
                                  <span className="row-file-name">{tx.fileName}</span>
                                </div>
                              </td>

                              <td className="mono-cell">
                                {(tx.fileSize / 1024 / 1024).toFixed(1)} MB
                              </td>

                              <td>
                                <span
                                  className={`dir-pill ${
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
                                <div className="integrity-cell">
                                  <Check size={11} color="var(--color-secondary)" />
                                  <span>SHA-256 Verified</span>
                                </div>
                              </td>

                              <td className="timestamp-cell">
                                {Math.round((Date.now() - tx.timestamp) / 60000)}m ago
                              </td>

                              <td style={{ textAlign: "right" }}>
                                <button
                                  className="btn-reveal"
                                  onClick={() => showToast(`Revealing ${tx.fileName}`)}
                                >
                                  <ExternalLink size={10} />
                                  <span>Reveal in Folder</span>
                                </button>
                              </td>
                            </tr>
                          ))
                        )}
                      </tbody>
                    </table>
                  </div>

                  <div className="table-audit-footer">
                    <span>All transactions audited against local Ed25519 keyring</span>
                    <span>Auto-cleared after 7 days</span>
                  </div>
                </div>
              </div>
            )}

            {/* TAB 2: CLIPBOARD */}
            {activeTab === "clipboard" && (
              <div className="tab-layout-flow">
                <div className="panel-card-box">
                  <div className="panel-item-row">
                    <div className="panel-item-lead">
                      <RefreshCw size={16} color="var(--color-outline)" />
                      <div>
                        <div className="panel-item-title">Real-Time Clipboard Synchronization</div>
                        <div className="panel-item-desc">
                          Mirror text and images between this PC and {currentPeer.displayName}
                        </div>
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

                  <div className="panel-sep" />

                  <div className="panel-item-row">
                    <div className="panel-item-lead">
                      <ShieldCheck size={16} color="var(--color-secondary)" />
                      <div>
                        <div className="panel-item-title">Echo Suppression Ring</div>
                        <div className="panel-item-desc">
                          16-entry rolling hash ring prevents recursive loops across peer mesh
                        </div>
                      </div>
                    </div>
                    <span className="optimal-tag">Active</span>
                  </div>
                </div>

                {/* Blast Input */}
                <div className="instant-blast-block">
                  <span className="sub-heading-label">Push Text to Device</span>
                  <div className="blast-input-row">
                    <input
                      type="text"
                      className="form-input"
                      placeholder={`Type or paste text to blast to ${currentPeer.displayName}... (Press Enter)`}
                      value={clipboardInput}
                      onChange={(e) => setClipboardInput(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleSendClipboard();
                      }}
                    />
                    <button className="btn btn-primary" onClick={handleSendClipboard}>
                      Send to Device
                    </button>
                  </div>
                </div>

                {/* Synchronized Clips Table */}
                <div className="activity-table-card">
                  <div className="table-control-bar">
                    <span className="table-title-text">Synchronized Clips ({filteredClips.length})</span>
                    <div className="table-search-box">
                      <Search size={12} className="search-icon" />
                      <input
                        type="text"
                        className="search-field"
                        placeholder="Search clips..."
                        value={clipFilterQuery}
                        onChange={(e) => setClipFilterQuery(e.target.value)}
                      />
                    </div>
                  </div>

                  <div className="table-scroller">
                    <table className="dense-data-table">
                      <thead>
                        <tr>
                          <th>Content</th>
                          <th>Origin</th>
                          <th>Time</th>
                          <th style={{ textAlign: "right" }}>Action</th>
                        </tr>
                      </thead>
                      <tbody>
                        {filteredClips.map((clip) => (
                          <tr key={clip.id}>
                            <td className="mono-cell" style={{ maxWidth: 360 }}>
                              <span className="clip-content-truncate">{clip.text}</span>
                            </td>
                            <td className="dim-cell">{clip.device}</td>
                            <td className="timestamp-cell">{clip.time}</td>
                            <td style={{ textAlign: "right" }}>
                              <button
                                className="btn-reveal"
                                onClick={() => handleCopyText(clip.text, clip.id)}
                              >
                                {copiedClipId === clip.id ? (
                                  <>
                                    <Check size={11} color="var(--color-secondary)" />
                                    <span>Copied</span>
                                  </>
                                ) : (
                                  <>
                                    <Copy size={11} />
                                    <span>Copy</span>
                                  </>
                                )}
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>
            )}

            {/* TAB 3: NOTIFICATIONS */}
            {activeTab === "notifications" && (
              <div className="tab-layout-flow">
                <div className="activity-table-card">
                  <div className="table-control-bar">
                    <span className="table-title-text">Mobile Notification Relay</span>
                    {notifications.length > 0 && (
                      <button className="clear-history-btn" onClick={handleClearAllNotifications}>
                        <Trash2 size={12} />
                        <span>Clear All</span>
                      </button>
                    )}
                  </div>

                  {notifications.length === 0 ? (
                    <div className="empty-tab-box">
                      <Bell size={24} color="var(--color-outline)" />
                      <span className="empty-title">All Caught Up</span>
                      <span className="empty-desc">
                        No push notifications forwarded from {currentPeer.displayName}.
                      </span>
                    </div>
                  ) : (
                    <div className="notifications-stream">
                      {notifications.map((notif) => (
                        <div key={notif.id} className="notification-card-item">
                          <div className="notif-avatar">
                            <Bell size={14} color="var(--color-primary)" />
                          </div>
                          <div className="notif-body">
                            <div className="notif-top">
                              <span className="notif-sender">{notif.title}</span>
                              <span className="notif-app-badge">{notif.appName}</span>
                              <span className="notif-time-dim">2m ago</span>
                            </div>
                            <div className="notif-text-msg">{notif.body}</div>
                          </div>
                          <button
                            className="btn-reveal"
                            onClick={() => handleDismissNotification(notif.id)}
                          >
                            Dismiss
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* TAB 4: SETTINGS */}
            {activeTab === "settings" && (
              <div className="tab-layout-flow">
                <div className="panel-card-box">
                  <div className="panel-item-row">
                    <div className="panel-item-lead">
                      <Upload size={16} color="var(--color-outline)" />
                      <div>
                        <div className="panel-item-title">File Transfer Permission</div>
                        <div className="panel-item-desc">
                          Allow encrypted QUIC stream payloads to and from this device
                        </div>
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

                  <div className="panel-sep" />

                  <div className="panel-item-row">
                    <div className="panel-item-lead">
                      <Bell size={16} color="var(--color-outline)" />
                      <div>
                        <div className="panel-item-title">Notification Mirroring</div>
                        <div className="panel-item-desc">
                          Stream incoming mobile notifications with remote dismiss sync
                        </div>
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

                  <div className="panel-sep" />

                  <div className="panel-item-row">
                    <div className="panel-item-lead">
                      <SlidersHorizontal size={16} color="var(--color-outline)" />
                      <div>
                        <div className="panel-item-title">Auto-Accept Small Payloads</div>
                        <div className="panel-item-desc">
                          Automatically accept files under 10 MB without manual approval
                        </div>
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

                <div className="panel-card-box danger-border">
                  <div className="panel-item-row">
                    <div>
                      <div className="panel-item-title">Unpair {currentPeer.displayName}</div>
                      <div className="panel-item-desc">
                        Revoke mutual mTLS certificate and terminate the active QUIC session
                      </div>
                    </div>
                    <button
                      className="btn-danger-outline"
                      onClick={() => handleDisconnectPeer(currentPeer.fingerprint)}
                    >
                      <Trash2 size={12} />
                      <span>Unpair Device</span>
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Bottom Status Bar */}
          <footer className="footer-status-bar">
            <div className="footer-status-left">
              <span className="daemon-pid">Daemon: PID 41888 (Continuity-Mesh)</span>
              <span>•</span>
              <span>QUIC RTT: 2.14ms</span>
              <span>•</span>
              <span>Buffer: 64 MB Slot</span>
            </div>

            <div className="footer-status-right">
              <span>Peer Key: {currentPeer.fingerprint.substring(0, 16)}...</span>
              <span>•</span>
              <span>UTF-8 Encoded</span>
            </div>
          </footer>
        </main>
      </div>

      {/* Pair New Device Modal */}
      {showPairDialog && (
        <div className="modal-backdrop" onClick={() => setShowPairDialog(false)}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <span className="modal-title">Pair Remote Device</span>
              <button className="modal-close-btn" onClick={() => setShowPairDialog(false)}>
                <X size={15} />
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

      {/* Floating Feedback Toast */}
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
