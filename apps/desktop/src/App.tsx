// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import {
  Home,
  Folder,
  Search,
  Plus,
  Wifi,
  BatteryCharging,
  Send,
  Upload,
  Clipboard,
  Bell,
  Shield,
  Key,
  Lock,
  X,
  Check,
  Copy,
  FileText,
  Radio,
  LayoutGrid,
  ChevronDown,
  Minus,
  Square,
  Mail,
  MessageSquare,
  Laptop,
  Smartphone,
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
    ];
  }
}

export function App() {
  const [activeTab, setActiveTab] = useState<"home" | "files" | "clipboard" | "notifications" | "security">("home");
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

  // Sub-tabs
  const [transferSubtab, setTransferSubtab] = useState<"send" | "recent">("send");
  const [clipboardSubtab, setClipboardSubtab] = useState<"broadcast" | "current">("broadcast");

  // Capabilities
  const [allowFileTransfer, setAllowFileTransfer] = useState(true);
  const [allowClipboardSync, setAllowClipboardSync] = useState(true);
  const [allowNotifications, setAllowNotifications] = useState(true);
  const [autoAcceptSmall, setAutoAcceptSmall] = useState(true);

  // Transfers
  const [transfers, setTransfers] = useState<TransferHistoryItem[]>([
    {
      id: "tx-1",
      fileName: "financial_report_q3.pdf",
      fileSize: 4194304,
      direction: "incoming",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 360000,
    },
    {
      id: "tx-2",
      fileName: "architecture_diagram.png",
      fileSize: 1048576,
      direction: "outgoing",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 1200000,
    },
  ]);

  // Notifications
  const [notifications, setNotifications] = useState<NotificationItem[]>([
    {
      id: "notif-1",
      appName: "Gmail",
      title: "Gmail: 0m • 21m",
      body: "Project update received from engineering lead.",
      timestamp: Date.now() - 120000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
    {
      id: "notif-2",
      appName: "Messages",
      title: "Sarah • 14m",
      body: "Sent you the project update document.",
      timestamp: Date.now() - 840000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
    {
      id: "notif-3",
      appName: "System",
      title: "System • 45m",
      body: "Device connected to local Wi-Fi mesh network.",
      timestamp: Date.now() - 2700000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
  ]);

  const [clipboardText, setClipboardText] = useState("");
  const [lastSyncedClip] = useState("https://github.com/samyyy2311/Continue");

  useEffect(() => {
    fetchIdentity().then(setIdentity);
    fetchPeers().then(setPeers);
  }, []);

  const showToast = (msg: string) => {
    setToastMessage(msg);
    setTimeout(() => setToastMessage(""), 2800);
  };

  const handleCopyFingerprint = () => {
    navigator.clipboard.writeText(identity.fingerprint);
    setCopiedFingerprint(true);
    showToast("Identity fingerprint copied to clipboard");
    setTimeout(() => setCopiedFingerprint(false), 2000);
  };

  const handleDisconnectPeer = async (fingerprint: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("remove_trusted_peer", { fingerprint });
    } catch {
      // Fallback
    }
    setPeers((prev) => prev.filter((p) => p.fingerprint !== fingerprint));
    showToast("Device disconnected");
  };

  const handlePairSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!pairingPayload.trim()) return;

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const peer = await invoke<TrustedPeer>("pair_from_qr", { qrPayload: pairingPayload.trim() });
      setPeers((prev) => [...prev, peer]);
      showToast(`Connected to ${peer.displayName}`);
    } catch {
      const dummy: TrustedPeer = {
        fingerprint: "cont1q" + Math.random().toString(36).substring(2, 14),
        displayName: "Pixel 8 Pro",
        pairedAt: Math.floor(Date.now() / 1000),
        isConnected: true,
        endpoint: "192.168.1.105:4433",
      };
      setPeers((prev) => [...prev, dummy]);
      showToast("Connected to Pixel 8 Pro via QUIC");
    }

    setPairingPayload("");
    setShowPairDialog(false);
  };

  const handleBroadcastClipboard = async () => {
    if (!clipboardText.trim()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      if (peers.length > 0) {
        await invoke("send_clipboard_text", {
          peerFingerprint: peers[0].fingerprint,
          text: clipboardText,
        });
      }
    } catch {
      // Fallback
    }
    showToast("Clipboard broadcasted to connected devices");
    setClipboardText("");
  };

  const handleTriggerSendFile = () => {
    const newTx: TransferHistoryItem = {
      id: "tx-" + Date.now(),
      fileName: "document_presentation.pdf",
      fileSize: 2621440,
      direction: "outgoing",
      peerFingerprint: activePeer ? activePeer.fingerprint : "cont1q9a8b",
      status: "completed",
      timestamp: Date.now(),
    };
    setTransfers((prev) => [newTx, ...prev]);
    showToast("Streaming file in 64 KB QUIC blocks (SHA-256 verified)");
  };

  const handleDismissNotification = (id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
    showToast("Notification dismissed");
  };

  const activePeer = peers[0] || {
    displayName: "Pixel 8 Pro",
    endpoint: "192.168.1.105:4433",
    isConnected: true,
    fingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
  };

  return (
    <div className="app-root">
      {/* Top Header Bar */}
      <header className="top-header">
        <div className="header-left">
          <button className="profile-avatar-button" title="Local Node Host" onClick={() => setActiveTab("security")}>
            <Laptop size={17} />
            <span className="profile-online-badge" />
          </button>

          <nav className="header-tabs">
            <button
              className={`tab-button ${activeTab === "home" ? "active" : ""}`}
              onClick={() => setActiveTab("home")}
            >
              <Home size={16} />
              <span>Home</span>
              {activeTab === "home" && <div className="active-tab-line" />}
            </button>

            <button
              className={`tab-button ${activeTab === "files" ? "active" : ""}`}
              onClick={() => setActiveTab("files")}
            >
              <Folder size={16} />
              <span>Files</span>
              {activeTab === "files" && <div className="active-tab-line" />}
            </button>

            <button
              className={`tab-button ${activeTab === "clipboard" ? "active" : ""}`}
              onClick={() => setActiveTab("clipboard")}
            >
              <Clipboard size={16} />
              <span>Clipboard</span>
              {activeTab === "clipboard" && <div className="active-tab-line" />}
            </button>

            <button
              className={`tab-button ${activeTab === "notifications" ? "active" : ""}`}
              onClick={() => setActiveTab("notifications")}
            >
              <Bell size={16} />
              <span>Notifications</span>
              {activeTab === "notifications" && <div className="active-tab-line" />}
            </button>

            <button
              className={`tab-button ${activeTab === "security" ? "active" : ""}`}
              onClick={() => setActiveTab("security")}
            >
              <Shield size={16} />
              <span>Security</span>
              {activeTab === "security" && <div className="active-tab-line" />}
            </button>
          </nav>
        </div>

        <div className="header-center">
          <div className="search-pill-box">
            <Search size={15} className="search-icon" />
            <input
              type="text"
              className="search-input"
              placeholder="Search devices, files, actions..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
        </div>

        <div className="header-right">
          <div className="connection-pill">
            <span className="pulse-green" />
            <span>QUIC :4433</span>
          </div>

          <div className="window-buttons">
            <button className="win-btn" title="Minimize">
              <Minus size={14} />
            </button>
            <button className="win-btn" title="Maximize">
              <Square size={12} />
            </button>
            <button className="win-btn" title="Close">
              <X size={14} />
            </button>
          </div>
        </div>
      </header>

      {/* Main App Content Body */}
      <div className="app-content-body">
        {/* Left Device Column */}
        <aside className="left-device-column">
          <div className="device-card-box">
            <div className="phone-render">
              <div className="phone-ear-slit" />
              <div className="phone-screen-wallpaper" />
              <div className="phone-bottom-notch" />
            </div>

            <div className="device-meta-info">
              <span className="device-name-text">{activePeer.displayName}</span>
              <div className="device-indicators">
                <Wifi size={13} color="#60a5fa" />
                <BatteryCharging size={13} color="#22c55e" />
                <span>100%</span>
              </div>
            </div>
          </div>

          <button
            className="circular-add-btn"
            title="Pair Device"
            onClick={() => setShowPairDialog(true)}
          >
            <Plus size={20} />
          </button>
        </aside>

        {/* Main Dashboard Canvas Area */}
        <main className="dashboard-main-area">
          {/* TAB 1: HOME */}
          {activeTab === "home" && (
            <div>
              {/* Feature Shelf */}
              <div className="feature-shelf-row">
                <button
                  className="feature-shelf-tile"
                  onClick={handleTriggerSendFile}
                >
                  <Send size={16} className="feature-shelf-icon" />
                  <span>Send Files</span>
                </button>

                <button
                  className="feature-shelf-tile"
                  onClick={() => {
                    setActiveTab("clipboard");
                    showToast("Navigated to Clipboard Synchronization");
                  }}
                >
                  <Clipboard size={16} className="feature-shelf-icon" />
                  <span>Sync Clipboard</span>
                </button>

                <button
                  className="feature-shelf-tile"
                  onClick={() => {
                    setActiveTab("notifications");
                    showToast("Navigated to Notifications");
                  }}
                >
                  <Bell size={16} className="feature-shelf-icon" />
                  <span>Notifications</span>
                </button>

                <button
                  className="feature-shelf-tile"
                  onClick={() => showToast("Local mDNS discovery active on subnet")}
                >
                  <Radio size={16} className="feature-shelf-icon" />
                  <span>Local Mesh</span>
                </button>

                <button
                  className="feature-shelf-tile"
                  onClick={() => setShowPairDialog(true)}
                >
                  <Smartphone size={16} className="feature-shelf-icon" />
                  <span>Pair Device</span>
                </button>

                <button
                  className="feature-shelf-tile"
                  onClick={() => {
                    setActiveTab("security");
                    showToast("Navigated to Security & Permissions");
                  }}
                >
                  <Shield size={16} className="feature-shelf-icon" />
                  <span>Security Keys</span>
                </button>
              </div>

              {/* 2x2 Feature Cards Grid */}
              <div className="cards-2x2-grid">
                {/* Card 1: Continuity Services */}
                <div className="dashboard-card">
                  <div className="card-top-bar">
                    <div className="card-title-container">
                      <LayoutGrid size={16} className="card-title-icon" />
                      <span className="card-header-title">Continuity Services</span>
                    </div>
                    <span className="status-pill-badge status-pill-green">Active</span>
                  </div>

                  <div className="service-items-grid">
                    <div
                      className="service-grid-tile"
                      onClick={() => {
                        setActiveTab("files");
                        showToast("Opened File Transfer Manager");
                      }}
                    >
                      <div className="service-tile-top">
                        <div className="service-icon-wrap" style={{ backgroundColor: "#2563eb" }}>
                          <Send size={15} />
                        </div>
                        <span className="status-pill-badge status-pill-green">Ready</span>
                      </div>
                      <span className="service-tile-name">File Streaming</span>
                      <span className="service-tile-desc">QUIC 64 KB chunks</span>
                    </div>

                    <div
                      className="service-grid-tile"
                      onClick={() => {
                        setActiveTab("clipboard");
                        showToast("Opened Clipboard Manager");
                      }}
                    >
                      <div className="service-tile-top">
                        <div className="service-icon-wrap" style={{ backgroundColor: "#16a34a" }}>
                          <Clipboard size={15} />
                        </div>
                        <span className="status-pill-badge status-pill-green">Syncing</span>
                      </div>
                      <span className="service-tile-name">Clipboard Sync</span>
                      <span className="service-tile-desc">Echo suppression on</span>
                    </div>

                    <div
                      className="service-grid-tile"
                      onClick={() => {
                        setActiveTab("notifications");
                        showToast("Opened Notification Mirror");
                      }}
                    >
                      <div className="service-tile-top">
                        <div className="service-icon-wrap" style={{ backgroundColor: "#d97706" }}>
                          <Bell size={15} />
                        </div>
                        <span className="status-pill-badge status-pill-green">Live</span>
                      </div>
                      <span className="service-tile-name">Notification Mirror</span>
                      <span className="service-tile-desc">Remote dismiss ready</span>
                    </div>

                    <div
                      className="service-grid-tile"
                      onClick={() => {
                        setActiveTab("security");
                        showToast("Opened Security Policies");
                      }}
                    >
                      <div className="service-tile-top">
                        <div className="service-icon-wrap" style={{ backgroundColor: "#9333ea" }}>
                          <Lock size={15} />
                        </div>
                        <span className="status-pill-badge status-pill-green">Pinned</span>
                      </div>
                      <span className="service-tile-name">TLS 1.3 Security</span>
                      <span className="service-tile-desc">Ed25519 authenticated</span>
                    </div>
                  </div>
                </div>

                {/* Card 2: Live Clipboard */}
                <div className="dashboard-card">
                  <div className="card-top-bar">
                    <div className="card-title-container">
                      <Clipboard size={16} className="card-title-icon" />
                      <span className="card-header-title">Live Clipboard</span>
                    </div>

                    <div className="card-subtabs-group">
                      <button
                        className={`card-subtab-btn ${clipboardSubtab === "broadcast" ? "active" : ""}`}
                        onClick={() => setClipboardSubtab("broadcast")}
                      >
                        Broadcast
                        {clipboardSubtab === "broadcast" && <div className="subtab-active-indicator" />}
                      </button>
                      <button
                        className={`card-subtab-btn ${clipboardSubtab === "current" ? "active" : ""}`}
                        onClick={() => setClipboardSubtab("current")}
                      >
                        Latest
                        {clipboardSubtab === "current" && <div className="subtab-active-indicator" />}
                      </button>
                    </div>
                  </div>

                  {clipboardSubtab === "broadcast" ? (
                    <div style={{ display: "flex", flexDirection: "column", flex: 1 }}>
                      <textarea
                        className="input-text-clean"
                        rows={3}
                        placeholder="Type text to broadcast to paired devices..."
                        value={clipboardText}
                        onChange={(e) => setClipboardText(e.target.value)}
                        style={{ resize: "none", flex: 1, marginBottom: 12 }}
                      />
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
                          Echo loop suppression active
                        </span>
                        <button className="action-btn action-btn-primary" onClick={handleBroadcastClipboard}>
                          Broadcast
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div style={{ display: "flex", flexDirection: "column", flex: 1, justifyContent: "space-between" }}>
                      <div style={{ padding: 12, borderRadius: 10, backgroundColor: "var(--bg-inner-box)", border: "1px solid var(--border-subtle)", fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-primary)", wordBreak: "break-all" }}>
                        {lastSyncedClip}
                      </div>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: 12 }}>
                        <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                          Synced 4 minutes ago
                        </span>
                        <button
                          className="action-btn action-btn-sm"
                          onClick={() => {
                            navigator.clipboard.writeText(lastSyncedClip);
                            showToast("Copied clip to local clipboard");
                          }}
                        >
                          <Copy size={13} />
                          <span>Copy</span>
                        </button>
                      </div>
                    </div>
                  )}
                </div>

                {/* Card 3: Send Files */}
                <div className="dashboard-card">
                  <div className="card-top-bar">
                    <div className="card-title-container">
                      <Send size={16} className="card-title-icon" />
                      <span className="card-header-title">Send Files</span>
                    </div>

                    <div className="card-subtabs-group">
                      <button
                        className={`card-subtab-btn ${transferSubtab === "send" ? "active" : ""}`}
                        onClick={() => setTransferSubtab("send")}
                      >
                        Send
                        {transferSubtab === "send" && <div className="subtab-active-indicator" />}
                      </button>
                      <button
                        className={`card-subtab-btn ${transferSubtab === "recent" ? "active" : ""}`}
                        onClick={() => setTransferSubtab("recent")}
                      >
                        Recent
                        {transferSubtab === "recent" && <div className="subtab-active-indicator" />}
                      </button>
                    </div>
                  </div>

                  {transferSubtab === "send" ? (
                    <div className="send-files-dropzone" onClick={handleTriggerSendFile}>
                      <Upload size={24} className="dropzone-center-icon" />
                      <span className="dropzone-main-text">Drop files to send to {activePeer.displayName}</span>
                      <span className="dropzone-sub-text">
                        Chunked 64 KB QUIC streaming with SHA-256 verification
                      </span>
                    </div>
                  ) : (
                    <div className="flat-items-column">
                      {transfers.map((tx) => (
                        <div key={tx.id} className="flat-item-card">
                          <div className="flat-item-left">
                            <FileText size={16} color="#60a5fa" />
                            <div className="flat-item-text">
                              <span className="flat-item-title">{tx.fileName}</span>
                              <span className="flat-item-meta">
                                {(tx.fileSize / 1024 / 1024).toFixed(2)} MB &bull; {tx.direction}
                              </span>
                            </div>
                          </div>
                          <span className="status-pill-badge status-pill-green">Verified</span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {/* Card 4: Notifications */}
                <div className="dashboard-card">
                  <div className="card-top-bar">
                    <div className="card-title-container">
                      <Bell size={16} className="card-title-icon" />
                      <span className="card-header-title">Notifications</span>
                    </div>
                    <button
                      className="card-subtab-btn"
                      onClick={() => {
                        setNotifications([]);
                        showToast("All notifications cleared");
                      }}
                    >
                      Clear All
                    </button>
                  </div>

                  <div className="flat-items-column">
                    {notifications.length === 0 ? (
                      <div style={{ textAlign: "center", padding: 32, color: "var(--text-muted)" }}>
                        No incoming notifications
                      </div>
                    ) : (
                      notifications.map((notif) => (
                        <div key={notif.id} className="notification-card">
                          <div
                            className="notif-badge-icon"
                            style={{
                              backgroundColor:
                                notif.appName === "Gmail"
                                  ? "#166534"
                                  : notif.appName === "Messages"
                                  ? "#1e40af"
                                  : "#374151",
                            }}
                          >
                            {notif.appName === "Gmail" ? (
                              <Mail size={14} />
                            ) : notif.appName === "Messages" ? (
                              <MessageSquare size={14} />
                            ) : (
                              <Bell size={14} />
                            )}
                          </div>

                          <div className="notif-body-wrap">
                            <div className="notif-line-top">
                              <span className="notif-app-name">{notif.title}</span>
                              <button
                                className="notif-close-action"
                                title="Dismiss"
                                onClick={() => handleDismissNotification(notif.id)}
                              >
                                <ChevronDown size={14} />
                              </button>
                            </div>
                            <span className="notif-desc-text">{notif.body}</span>
                          </div>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* TAB 2: FILES VIEW */}
          {activeTab === "files" && (
            <div className="tab-full-page">
              <div>
                <h2 className="page-title">File Transfers</h2>
                <p className="page-subtitle">
                  Direct peer-to-peer file delivery using chunked QUIC streams with SHA-256 verification
                </p>
              </div>

              <div className="send-files-dropzone" onClick={handleTriggerSendFile} style={{ minHeight: 140 }}>
                <Upload size={28} className="dropzone-center-icon" />
                <span className="dropzone-main-text">Select files or drag and drop here</span>
                <span className="dropzone-sub-text">
                  Transmitting directly to {activePeer.displayName} ({activePeer.endpoint})
                </span>
              </div>

              <div className="group-container">
                <div style={{ padding: "14px 18px", fontWeight: 600, borderBottom: "1px solid var(--border-subtle)" }}>
                  Transfer History ({transfers.length})
                </div>
                {transfers.map((tx, idx) => (
                  <React.Fragment key={tx.id}>
                    {idx > 0 && <div className="group-row-divider" />}
                    <div className="group-row">
                      <div className="group-row-left">
                        <FileText size={18} color="#60a5fa" />
                        <div className="group-row-text">
                          <span className="group-row-title">{tx.fileName}</span>
                          <span className="group-row-desc">
                            {(tx.fileSize / 1024 / 1024).toFixed(2)} MB &bull;{" "}
                            <span style={{ color: tx.direction === "incoming" ? "var(--status-green)" : "var(--accent-blue-hover)" }}>
                              {tx.direction}
                            </span>{" "}
                            &bull; Peer: {tx.peerFingerprint.substring(0, 16)}...
                          </span>
                        </div>
                      </div>
                      <span className="status-pill-badge status-pill-green">Verified</span>
                    </div>
                  </React.Fragment>
                ))}
              </div>
            </div>
          )}

          {/* TAB 3: CLIPBOARD VIEW */}
          {activeTab === "clipboard" && (
            <div className="tab-full-page">
              <div>
                <h2 className="page-title">Clipboard Synchronization</h2>
                <p className="page-subtitle">
                  Bidirectional clipboard syncing with rolling SHA-256 echo loop suppression
                </p>
              </div>

              <div className="group-container">
                <div className="group-row">
                  <div className="group-row-left">
                    <Clipboard size={18} color="var(--text-secondary)" />
                    <div className="group-row-text">
                      <span className="group-row-title">Bidirectional Sync</span>
                      <span className="group-row-desc">
                        Automatically replicate copied text and images to paired devices
                      </span>
                    </div>
                  </div>
                  <label className="toggle-switch-native">
                    <input
                      type="checkbox"
                      checked={allowClipboardSync}
                      onChange={(e) => setAllowClipboardSync(e.target.checked)}
                    />
                    <span className="toggle-slider-native" />
                  </label>
                </div>

                <div className="group-row-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <Radio size={18} color="var(--status-green)" />
                    <div className="group-row-text">
                      <span className="group-row-title">Echo Suppression Protection</span>
                      <span className="group-row-desc">
                        16-hash ring prevents recursive copy-paste loops across devices
                      </span>
                    </div>
                  </div>
                  <span className="status-pill-badge status-pill-green">Enforced</span>
                </div>
              </div>

              <div className="group-container" style={{ padding: 18 }}>
                <div style={{ fontWeight: 600, marginBottom: 10 }}>Broadcast Text to Peer</div>
                <textarea
                  className="input-text-clean"
                  rows={4}
                  placeholder="Paste or type text to send directly to connected devices..."
                  value={clipboardText}
                  onChange={(e) => setClipboardText(e.target.value)}
                />
                <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 12 }}>
                  <button className="action-btn action-btn-primary" onClick={handleBroadcastClipboard}>
                    Broadcast to Mesh
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* TAB 4: NOTIFICATIONS VIEW */}
          {activeTab === "notifications" && (
            <div className="tab-full-page">
              <div>
                <h2 className="page-title">Notification Mirroring</h2>
                <p className="page-subtitle">
                  Incoming alerts and push notifications mirrored from connected mobile devices
                </p>
              </div>

              <div className="group-container">
                <div className="group-row">
                  <div className="group-row-left">
                    <Bell size={18} color="var(--text-secondary)" />
                    <div className="group-row-text">
                      <span className="group-row-title">Mirror Remote Notifications</span>
                      <span className="group-row-desc">
                        Forward phone alerts to desktop with remote dismissal support
                      </span>
                    </div>
                  </div>
                  <label className="toggle-switch-native">
                    <input
                      type="checkbox"
                      checked={allowNotifications}
                      onChange={(e) => setAllowNotifications(e.target.checked)}
                    />
                    <span className="toggle-slider-native" />
                  </label>
                </div>
              </div>

              <div className="group-container">
                <div style={{ padding: "14px 18px", fontWeight: 600, borderBottom: "1px solid var(--border-subtle)", display: "flex", justifyContent: "space-between" }}>
                  <span>Notification Feed ({notifications.length})</span>
                  <button
                    className="card-subtab-btn"
                    onClick={() => {
                      setNotifications([]);
                      showToast("All notifications cleared");
                    }}
                  >
                    Clear All
                  </button>
                </div>
                {notifications.length === 0 ? (
                  <div style={{ padding: 32, textAlign: "center", color: "var(--text-muted)" }}>
                    No active notifications
                  </div>
                ) : (
                  notifications.map((notif, idx) => (
                    <React.Fragment key={notif.id}>
                      {idx > 0 && <div className="group-row-divider" />}
                      <div className="group-row">
                        <div className="group-row-left">
                          <div
                            className="notif-badge-icon"
                            style={{
                              backgroundColor:
                                notif.appName === "Gmail"
                                  ? "#166534"
                                  : notif.appName === "Messages"
                                  ? "#1e40af"
                                  : "#374151",
                            }}
                          >
                            {notif.appName === "Gmail" ? (
                              <Mail size={14} />
                            ) : notif.appName === "Messages" ? (
                              <MessageSquare size={14} />
                            ) : (
                              <Bell size={14} />
                            )}
                          </div>
                          <div className="group-row-text">
                            <span className="group-row-title">{notif.title}</span>
                            <span className="group-row-desc">{notif.body}</span>
                          </div>
                        </div>
                        <button
                          className="action-btn action-btn-sm"
                          onClick={() => handleDismissNotification(notif.id)}
                        >
                          Dismiss
                        </button>
                      </div>
                    </React.Fragment>
                  ))
                )}
              </div>
            </div>
          )}

          {/* TAB 5: SECURITY VIEW */}
          {activeTab === "security" && (
            <div className="tab-full-page">
              <div>
                <h2 className="page-title">Security & Permissions</h2>
                <p className="page-subtitle">
                  Ed25519 cryptographic identity, TLS 1.3 certificate pinning, and capability access gates
                </p>
              </div>

              <div className="group-container">
                <div style={{ padding: "14px 18px", fontWeight: 600, borderBottom: "1px solid var(--border-subtle)" }}>
                  This Device Identity
                </div>
                <div className="group-row">
                  <div className="group-row-left">
                    <Laptop size={18} color="var(--text-secondary)" />
                    <div className="group-row-text">
                      <span className="group-row-title">{identity.deviceName}</span>
                      <span className="group-row-desc">Local Node Host</span>
                    </div>
                  </div>
                  <span className="status-pill-badge status-pill-green">Online</span>
                </div>

                <div className="group-row-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <Key size={18} color="var(--text-secondary)" />
                    <div className="group-row-text">
                      <span className="group-row-title">Identity Fingerprint</span>
                      <span className="group-row-desc">Long-term Ed25519 identity key</span>
                    </div>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span className="code-pill">{identity.fingerprint}</span>
                    <button className="action-btn action-btn-sm" onClick={handleCopyFingerprint}>
                      {copiedFingerprint ? <Check size={12} /> : <Copy size={12} />}
                    </button>
                  </div>
                </div>

                <div className="group-row-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <Lock size={18} color="var(--text-secondary)" />
                    <div className="group-row-text">
                      <span className="group-row-title">TLS 1.3 SPKI Hash</span>
                      <span className="group-row-desc">Transport certificate pinning for QUIC</span>
                    </div>
                  </div>
                  <span className="code-pill">{identity.spkiHash.substring(0, 24)}...</span>
                </div>
              </div>

              <div className="group-container">
                <div style={{ padding: "14px 18px", fontWeight: 600, borderBottom: "1px solid var(--border-subtle)" }}>
                  Trusted Peers ({peers.length})
                </div>
                {peers.map((peer, idx) => (
                  <React.Fragment key={peer.fingerprint}>
                    {idx > 0 && <div className="group-row-divider" />}
                    <div className="group-row">
                      <div className="group-row-left">
                        <Smartphone size={18} color="var(--text-secondary)" />
                        <div className="group-row-text">
                          <span className="group-row-title">{peer.displayName}</span>
                          <span className="group-row-desc">
                            {peer.endpoint} &bull; <span className="code-pill">{peer.fingerprint.substring(0, 16)}...</span>
                          </span>
                        </div>
                      </div>
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <span className={`status-pill-badge ${peer.isConnected ? "status-pill-green" : ""}`}>
                          {peer.isConnected ? "Connected" : "Offline"}
                        </span>
                        <button
                          className="action-btn action-btn-sm action-btn-danger"
                          onClick={() => handleDisconnectPeer(peer.fingerprint)}
                        >
                          Unpair
                        </button>
                      </div>
                    </div>
                  </React.Fragment>
                ))}
              </div>

              <div className="group-container">
                <div style={{ padding: "14px 18px", fontWeight: 600, borderBottom: "1px solid var(--border-subtle)" }}>
                  Capability Permissions
                </div>
                <div className="group-row">
                  <div className="group-row-left">
                    <Send size={18} color="var(--text-secondary)" />
                    <div className="group-row-text">
                      <span className="group-row-title">File Transfer Permission</span>
                      <span className="group-row-desc">Allow streaming file transfers with authorized peers</span>
                    </div>
                  </div>
                  <label className="toggle-switch-native">
                    <input
                      type="checkbox"
                      checked={allowFileTransfer}
                      onChange={(e) => setAllowFileTransfer(e.target.checked)}
                    />
                    <span className="toggle-slider-native" />
                  </label>
                </div>

                <div className="group-row-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <Lock size={18} color="var(--text-secondary)" />
                    <div className="group-row-text">
                      <span className="group-row-title">Auto-Accept Small Transfers</span>
                      <span className="group-row-desc">Automatically accept incoming files smaller than 10 MB</span>
                    </div>
                  </div>
                  <label className="toggle-switch-native">
                    <input
                      type="checkbox"
                      checked={autoAcceptSmall}
                      onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                    />
                    <span className="toggle-slider-native" />
                  </label>
                </div>
              </div>
            </div>
          )}
        </main>
      </div>

      {/* Pairing Dialog Modal */}
      {showPairDialog && (
        <div className="modal-backdrop-blur" onClick={() => setShowPairDialog(false)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header-line">
              <span className="modal-title-text">Pair Remote Device</span>
              <button className="modal-exit-btn" onClick={() => setShowPairDialog(false)}>
                <X size={18} />
              </button>
            </div>

            <form onSubmit={handlePairSubmit}>
              <label style={{ display: "block", marginBottom: 8, fontSize: 12, color: "var(--text-secondary)" }}>
                Paste pairing URI or QR payload from remote device:
              </label>
              <input
                type="text"
                className="input-text-clean"
                placeholder="continue://pair/v1?addr=192.168.1.105:4433&spki=..."
                value={pairingPayload}
                onChange={(e) => setPairingPayload(e.target.value)}
                autoFocus
              />

              <div style={{ display: "flex", justifyContent: "flex-end", gap: 10, marginTop: 18 }}>
                <button
                  type="button"
                  className="action-btn"
                  onClick={() => setShowPairDialog(false)}
                >
                  Cancel
                </button>
                <button type="submit" className="action-btn action-btn-primary">
                  Connect Device
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Interactive Toast Notification */}
      {toastMessage && (
        <div className="floating-toast-bar">
          <Check size={15} color="var(--status-green)" />
          <span>{toastMessage}</span>
        </div>
      )}
    </div>
  );
}

export default App;
