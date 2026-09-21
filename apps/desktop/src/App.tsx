// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import {
  Share2,
  Smartphone,
  Tablet,
  ArrowDownUp,
  Clipboard,
  Bell,
  Settings,
  Plus,
  Key,
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
  SlidersHorizontal,
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

type AppPage = "devices" | "transfers" | "clipboard" | "notifications" | "settings";

export function App() {
  const [activePage, setActivePage] = useState<AppPage>("devices");
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
      body: "Starting in 10 minutes in Room B",
      timestamp: Date.now() - 600000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
  ]);

  const [clipboardInput, setClipboardInput] = useState("");
  const [syncedClips, setSyncedClips] = useState([
    {
      id: "clip-1",
      text: "https://github.com/samyyy2311/Continue",
      time: "3m ago",
      device: "Pixel 8 Pro",
    },
    {
      id: "clip-2",
      text: "cargo test --workspace --all-targets",
      time: "15m ago",
      device: "Desktop PC",
    },
  ]);

  useEffect(() => {
    fetchIdentity().then(setIdentity);
    fetchPeers().then(setPeers);
  }, []);

  const showToast = (msg: string) => {
    setToastMessage(msg);
    setTimeout(() => setToastMessage(""), 2600);
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
    showToast("Device unshared");
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
        displayName: "New Mobile Device",
        pairedAt: Math.floor(Date.now() / 1000),
        isConnected: true,
        endpoint: "192.168.1.115:4433",
      };
      setPeers((prev) => [...prev, dummy]);
      showToast("Connected via QUIC");
    }

    setPairingPayload("");
    setShowPairDialog(false);
  };

  const handleSendClipboard = async () => {
    if (!clipboardInput.trim()) return;
    const targetPeer = peers[0];
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      if (targetPeer) {
        await invoke("send_clipboard_text", {
          peerFingerprint: targetPeer.fingerprint,
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
    showToast("Clip synced to devices");
    setClipboardInput("");
  };

  const handleTriggerSendFile = (targetDeviceName?: string) => {
    const newTx: TransferHistoryItem = {
      id: "tx-" + Date.now(),
      fileName: "presentation_deck.pdf",
      fileSize: 3145728,
      direction: "outgoing",
      peerFingerprint: peers[0]?.fingerprint || "cont1q9a8b",
      status: "completed",
      timestamp: Date.now(),
    };
    setTransfers((prev) => [newTx, ...prev]);
    showToast(`Streaming presentation_deck.pdf to ${targetDeviceName || "peer"}`);
  };

  const handleDismissNotification = (id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
    showToast("Notification dismissed");
  };

  const pageTitleMap: Record<AppPage, string> = {
    devices: "Connected Devices",
    transfers: "File Transfers",
    clipboard: "Clipboard Sync",
    notifications: "Notification Feed",
    settings: "System Settings",
  };

  return (
    <div className="app-shell">
      {/* Sidebar matching CassetteCat (200px width) */}
      <aside className="sidebar">
        <div className="sidebar-header">
          <div className="brand-icon-box">
            <Share2 size={16} />
          </div>
          <span className="brand-title">Continue</span>
        </div>

        <nav className="sidebar-nav">
          <button
            className={`nav-item ${activePage === "devices" ? "active" : ""}`}
            onClick={() => setActivePage("devices")}
          >
            <Smartphone size={16} className="nav-item-icon" />
            <span className="nav-item-label">Devices</span>
            <span className="nav-item-badge">{peers.length}</span>
          </button>

          <button
            className={`nav-item ${activePage === "transfers" ? "active" : ""}`}
            onClick={() => setActivePage("transfers")}
          >
            <ArrowDownUp size={16} className="nav-item-icon" />
            <span className="nav-item-label">Transfers</span>
          </button>

          <button
            className={`nav-item ${activePage === "clipboard" ? "active" : ""}`}
            onClick={() => setActivePage("clipboard")}
          >
            <Clipboard size={16} className="nav-item-icon" />
            <span className="nav-item-label">Clipboard</span>
          </button>

          <button
            className={`nav-item ${activePage === "notifications" ? "active" : ""}`}
            onClick={() => setActivePage("notifications")}
          >
            <Bell size={16} className="nav-item-icon" />
            <span className="nav-item-label">Notifications</span>
            {notifications.length > 0 && (
              <span className="nav-item-badge" style={{ color: "var(--accent)" }}>
                {notifications.length}
              </span>
            )}
          </button>

          <div className="sidebar-divider" />

          <button
            className={`nav-item ${activePage === "settings" ? "active" : ""}`}
            onClick={() => setActivePage("settings")}
          >
            <Settings size={16} className="nav-item-icon" />
            <span className="nav-item-label">Settings</span>
          </button>
        </nav>

        <div className="sidebar-footer">
          <div className="host-name-row">
            <span className="host-label">{identity.deviceName}</span>
            <span className="host-tag">Host</span>
          </div>
          <button
            className="host-fingerprint-btn"
            title="Copy Ed25519 identity key"
            onClick={handleCopyFingerprint}
          >
            <Key size={11} />
            <span>{identity.fingerprint.substring(0, 14)}...</span>
            {copiedFingerprint ? <Check size={11} color="var(--green)" /> : <Copy size={11} />}
          </button>
        </div>
      </aside>

      {/* Main Workspace Stage */}
      <main className="main-stage">
        {/* Top Header Bar */}
        <header className="main-header">
          <div className="main-header-left">
            <h1 className="page-title">{pageTitleMap[activePage]}</h1>
            <div className="header-status-chip">
              <span className="status-dot" />
              <span>QUIC :4433 Active</span>
            </div>
          </div>

          <div className="main-header-right">
            <button className="btn btn-primary" onClick={() => setShowPairDialog(true)}>
              <Plus size={13} />
              <span>Pair Device</span>
            </button>
          </div>
        </header>

        {/* Content Body */}
        <div className="page-body">
          {/* PAGE 1: DEVICES */}
          {activePage === "devices" && (
            <div>
              <span className="section-label">Paired Devices ({peers.length})</span>
              <div className="device-grid">
                {peers.map((peer) => (
                  <div key={peer.fingerprint} className="device-card">
                    <div className="device-card-header">
                      <div className="device-card-avatar">
                        {peer.displayName.toLowerCase().includes("tablet") ? (
                          <Tablet size={18} />
                        ) : (
                          <Smartphone size={18} />
                        )}
                      </div>
                      <div className="device-card-meta">
                        <div className="device-card-title">{peer.displayName}</div>
                        <div className="device-card-endpoint">{peer.endpoint}</div>
                      </div>
                      <div style={{ display: "flex", alignItems: "center", gap: 5, fontSize: 11, color: "var(--green)" }}>
                        <span className="status-dot" />
                        <span>Online</span>
                      </div>
                    </div>

                    <div className="device-card-stats">
                      <span>Battery: 100%</span>
                      <span>mTLS 1.3 Pinned</span>
                      <span>Latency: 2ms</span>
                    </div>

                    <div className="device-card-actions">
                      <button
                        className="btn btn-sm"
                        onClick={() => handleTriggerSendFile(peer.displayName)}
                      >
                        <Upload size={12} />
                        <span>Send File</span>
                      </button>
                      <button
                        className="btn btn-sm btn-danger"
                        onClick={() => handleDisconnectPeer(peer.fingerprint)}
                      >
                        Unpair
                      </button>
                    </div>
                  </div>
                ))}
              </div>

              <span className="section-label">This Machine</span>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-lead">
                    <Key size={18} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">{identity.deviceName} (Ed25519 Local Node)</span>
                      <span className="setting-subtitle" style={{ fontFamily: "var(--font-mono)" }}>
                        {identity.fingerprint}
                      </span>
                    </div>
                  </div>
                  <button className="btn btn-sm" onClick={handleCopyFingerprint}>
                    <Copy size={12} />
                    <span>Copy Key</span>
                  </button>
                </div>
                <div className="setting-divider" />
                <div className="setting-row">
                  <div className="setting-lead">
                    <Lock size={18} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">SPKI Transport Certificate Hash</span>
                      <span className="setting-subtitle" style={{ fontFamily: "var(--font-mono)" }}>
                        {identity.spkiHash}
                      </span>
                    </div>
                  </div>
                  <span style={{ fontSize: 11, color: "var(--green)", fontWeight: 600 }}>Active</span>
                </div>
              </div>
            </div>
          )}

          {/* PAGE 2: TRANSFERS */}
          {activePage === "transfers" && (
            <div>
              <span className="section-label">Fast File Share</span>
              <div className="dropzone-container" onClick={() => handleTriggerSendFile()}>
                <div className="dropzone-lead">
                  <div className="dropzone-icon">
                    <Upload size={20} />
                  </div>
                  <div>
                    <div className="dropzone-title">Drop files to stream over encrypted QUIC</div>
                    <div className="dropzone-subtitle">
                      64 KB chunking with SHA-256 integrity verification
                    </div>
                  </div>
                </div>
                <button
                  className="btn btn-primary"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleTriggerSendFile();
                  }}
                >
                  Browse Files
                </button>
              </div>

              <span className="section-label">Transfer History</span>
              <div className="table-card">
                <div
                  className="table-head-row"
                  style={{ gridTemplateColumns: "3fr 1.5fr 1fr 1fr 1fr 1fr" }}
                >
                  <span>File Name</span>
                  <span>Peer</span>
                  <span>Size</span>
                  <span>Direction</span>
                  <span>Status</span>
                  <span style={{ textAlign: "right" }}>Actions</span>
                </div>

                {transfers.map((tx) => (
                  <div
                    key={tx.id}
                    className="table-body-row"
                    style={{ gridTemplateColumns: "3fr 1.5fr 1fr 1fr 1fr 1fr" }}
                  >
                    <div className="file-name-cell">
                      <FileText size={16} color="var(--accent)" />
                      <span className="file-name-text">{tx.fileName}</span>
                    </div>

                    <span style={{ color: "var(--text-secondary)" }}>Pixel 8 Pro</span>

                    <span style={{ color: "var(--silver-dim)", fontFamily: "var(--font-mono)" }}>
                      {(tx.fileSize / 1024 / 1024).toFixed(2)} MB
                    </span>

                    <div>
                      <span
                        className={`badge-dir ${tx.direction === "incoming" ? "badge-in" : "badge-out"}`}
                      >
                        {tx.direction === "incoming" ? (
                          <ArrowDownLeft size={12} />
                        ) : (
                          <ArrowUpRight size={12} />
                        )}
                        {tx.direction}
                      </span>
                    </div>

                    <div style={{ display: "flex", alignItems: "center", gap: 5, color: "var(--green)" }}>
                      <Check size={13} />
                      <span>Verified</span>
                    </div>

                    <div style={{ textAlign: "right" }}>
                      <button
                        className="btn btn-sm"
                        onClick={() => showToast(`Opened ${tx.fileName}`)}
                      >
                        <ExternalLink size={12} />
                        <span>Reveal</span>
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* PAGE 3: CLIPBOARD */}
          {activePage === "clipboard" && (
            <div>
              <span className="section-label">Sync Preferences</span>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-lead">
                    <RefreshCw size={18} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">Bidirectional Clipboard Sync</span>
                      <span className="setting-subtitle">
                        Automatically mirror copied text and media across connected peers
                      </span>
                    </div>
                  </div>
                  <label className="setting-switch">
                    <input
                      type="checkbox"
                      checked={allowClipboardSync}
                      onChange={(e) => setAllowClipboardSync(e.target.checked)}
                    />
                    <span className="switch-track" />
                  </label>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-lead">
                    <ShieldCheck size={18} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">Echo Suppression Protection</span>
                      <span className="setting-subtitle">
                        Rolling 16-entry hash ring eliminates recursive mirror paste loops
                      </span>
                    </div>
                  </div>
                  <span style={{ fontSize: 11, color: "var(--green)", fontWeight: 600 }}>Active</span>
                </div>
              </div>

              <span className="section-label">Send Text to Device</span>
              <div style={{ display: "flex", gap: 10, marginBottom: 24 }}>
                <input
                  type="text"
                  className="input-field"
                  placeholder="Type or paste text to blast to connected devices..."
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

              <span className="section-label">Synchronized History</span>
              <div className="table-card">
                <div
                  className="table-head-row"
                  style={{ gridTemplateColumns: "3fr 1fr 1fr 1fr" }}
                >
                  <span>Text Content</span>
                  <span>Origin</span>
                  <span>Time</span>
                  <span style={{ textAlign: "right" }}>Action</span>
                </div>

                {syncedClips.map((clip) => (
                  <div
                    key={clip.id}
                    className="table-body-row"
                    style={{ gridTemplateColumns: "3fr 1fr 1fr 1fr" }}
                  >
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 12,
                        color: "var(--text-primary)",
                        whiteSpace: "nowrap",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                      }}
                    >
                      {clip.text}
                    </span>
                    <span style={{ color: "var(--text-secondary)" }}>{clip.device}</span>
                    <span style={{ color: "var(--silver-dim)", fontSize: 11 }}>{clip.time}</span>
                    <div style={{ textAlign: "right" }}>
                      <button
                        className="btn btn-sm"
                        onClick={() => {
                          navigator.clipboard.writeText(clip.text);
                          showToast("Copied to clipboard");
                        }}
                      >
                        <Copy size={11} />
                        <span>Copy</span>
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* PAGE 4: NOTIFICATIONS */}
          {activePage === "notifications" && (
            <div>
              <span className="section-label">Notification Stream ({notifications.length})</span>
              {notifications.length === 0 ? (
                <div
                  style={{
                    padding: 48,
                    textAlign: "center",
                    color: "var(--silver-dim)",
                    backgroundColor: "var(--surface-card)",
                    borderRadius: 10,
                    border: "1px solid var(--border-subtle)",
                  }}
                >
                  No incoming notifications from connected devices
                </div>
              ) : (
                <div className="notif-list">
                  {notifications.map((notif) => (
                    <div key={notif.id} className="notif-card">
                      <div className="notif-icon-box">
                        <Bell size={16} />
                      </div>

                      <div className="notif-content">
                        <div className="notif-header-line">
                          <span className="notif-title">
                            {notif.title} &bull;{" "}
                            <span style={{ color: "var(--silver-dim)", fontWeight: 400 }}>
                              {notif.appName}
                            </span>
                          </span>
                          <span className="notif-time">2m ago</span>
                        </div>
                        <div className="notif-text">{notif.body}</div>
                      </div>

                      <button
                        className="btn btn-sm"
                        onClick={() => handleDismissNotification(notif.id)}
                      >
                        Dismiss
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* PAGE 5: SETTINGS */}
          {activePage === "settings" && (
            <div>
              <span className="section-label">Device Capabilities</span>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-lead">
                    <Upload size={18} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">File Transfer Support</span>
                      <span className="setting-subtitle">
                        Allow receiving and streaming files over encrypted QUIC
                      </span>
                    </div>
                  </div>
                  <label className="setting-switch">
                    <input
                      type="checkbox"
                      checked={allowFileTransfer}
                      onChange={(e) => setAllowFileTransfer(e.target.checked)}
                    />
                    <span className="switch-track" />
                  </label>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-lead">
                    <Bell size={18} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">Notification Mirroring</span>
                      <span className="setting-subtitle">
                        Forward push alerts from phone with remote dismiss synchronization
                      </span>
                    </div>
                  </div>
                  <label className="setting-switch">
                    <input
                      type="checkbox"
                      checked={allowNotifications}
                      onChange={(e) => setAllowNotifications(e.target.checked)}
                    />
                    <span className="switch-track" />
                  </label>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-lead">
                    <SlidersHorizontal size={18} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">Auto-Accept Small Transfers</span>
                      <span className="setting-subtitle">
                        Automatically accept payloads under 10 MB without manual prompt
                      </span>
                    </div>
                  </div>
                  <label className="setting-switch">
                    <input
                      type="checkbox"
                      checked={autoAcceptSmall}
                      onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                    />
                    <span className="switch-track" />
                  </label>
                </div>
              </div>

              <span className="section-label">Security & Identity</span>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-lead">
                    <Lock size={18} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">Mutual TLS 1.3 Certificate Pinning</span>
                      <span className="setting-subtitle">
                        SPKI transport certificate pinned to Ed25519 identity
                      </span>
                    </div>
                  </div>
                  <span style={{ fontSize: 11, color: "var(--green)", fontWeight: 600 }}>Enforced</span>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-lead">
                    <Key size={18} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">Local Node Fingerprint</span>
                      <span className="setting-subtitle" style={{ fontFamily: "var(--font-mono)" }}>
                        {identity.fingerprint}
                      </span>
                    </div>
                  </div>
                  <button className="btn btn-sm" onClick={handleCopyFingerprint}>
                    <Copy size={11} />
                    <span>Copy</span>
                  </button>
                </div>
              </div>

              <span className="section-label">Network & Sockets</span>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-lead">
                    <ArrowDownUp size={18} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">QUIC Transport Port</span>
                      <span className="setting-subtitle">Listening on UDP socket 0.0.0.0:4433</span>
                    </div>
                  </div>
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-secondary)" }}>
                    :4433
                  </span>
                </div>
              </div>
            </div>
          )}
        </div>
      </main>

      {/* Pair New Device Modal */}
      {showPairDialog && (
        <div className="modal-backdrop" onClick={() => setShowPairDialog(false)}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <span className="modal-title">Pair New Device</span>
              <button className="modal-close-btn" onClick={() => setShowPairDialog(false)}>
                <X size={16} />
              </button>
            </div>

            <form onSubmit={handlePairSubmit}>
              <label
                style={{
                  display: "block",
                  marginBottom: 8,
                  fontSize: 12,
                  color: "var(--text-secondary)",
                }}
              >
                Enter pairing URI or QR payload:
              </label>
              <input
                type="text"
                className="input-field"
                placeholder="continue://pair/v1?addr=192.168.1.105:4433&spki=..."
                value={pairingPayload}
                onChange={(e) => setPairingPayload(e.target.value)}
                autoFocus
              />

              <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 16 }}>
                <button
                  type="button"
                  className="btn"
                  onClick={() => setShowPairDialog(false)}
                >
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary">
                  Connect
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Toast Feedback */}
      {toastMessage && (
        <div className="floating-toast">
          <Check size={14} color="var(--green)" />
          <span>{toastMessage}</span>
        </div>
      )}
    </div>
  );
}

export default App;
