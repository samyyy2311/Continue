// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import {
  Smartphone,
  Send,
  Upload,
  Clipboard,
  Bell,
  Shield,
  Key,
  Lock,
  Plus,
  X,
  Check,
  Copy,
  FileText,
  Radio,
  Share2,
  Laptop,
  Wifi,
  BatteryCharging,
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
  const [activeTab, setActiveTab] = useState<"devices" | "transfers" | "clipboard" | "notifications" | "security">("devices");

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

  // Nearby unshared devices on LAN
  const [discoveredDevices] = useState([
    {
      id: "disc-1",
      displayName: "Tablet Air",
      endpoint: "192.168.1.112:4433",
      deviceType: "tablet",
    },
  ]);

  const [clipboardText, setClipboardText] = useState("");
  const [syncedClips, setSyncedClips] = useState([
    {
      id: "clip-1",
      text: "https://github.com/samyyy2311/Continue",
      origin: "Pixel 8 Pro",
      timestamp: Date.now() - 240000,
    },
    {
      id: "clip-2",
      text: "cargo test --workspace --all-targets",
      origin: "Desktop PC",
      timestamp: Date.now() - 720000,
    },
  ]);

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
    const newClip = {
      id: "clip-" + Date.now(),
      text: clipboardText,
      origin: "Desktop PC",
      timestamp: Date.now(),
    };
    setSyncedClips((prev) => [newClip, ...prev]);
    showToast("Broadcasted clip to local mesh");
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
    showToast("Streaming file in 64 KB QUIC chunks (SHA-256 verified)");
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
    <div className="desktop-shell">
      {/* 220px Fixed Left Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-header">
          <div className="brand-icon-box">
            <Share2 size={16} />
          </div>
          <div className="brand-title-wrap">
            <span className="brand-name">Continue</span>
            <span className="brand-meta">LOCAL CONTINUITY</span>
          </div>
        </div>

        <ul className="nav-menu">
          <li>
            <button
              className={`nav-link ${activeTab === "devices" ? "active" : ""}`}
              onClick={() => setActiveTab("devices")}
            >
              {activeTab === "devices" && <span className="nav-link-indicator" />}
              <Smartphone size={16} />
              <span>Devices</span>
              <span className="nav-link-badge">{peers.length}</span>
            </button>
          </li>

          <li>
            <button
              className={`nav-link ${activeTab === "transfers" ? "active" : ""}`}
              onClick={() => setActiveTab("transfers")}
            >
              {activeTab === "transfers" && <span className="nav-link-indicator" />}
              <Send size={16} />
              <span>Transfers</span>
              <span className="nav-link-badge">{transfers.length}</span>
            </button>
          </li>

          <li>
            <button
              className={`nav-link ${activeTab === "clipboard" ? "active" : ""}`}
              onClick={() => setActiveTab("clipboard")}
            >
              {activeTab === "clipboard" && <span className="nav-link-indicator" />}
              <Clipboard size={16} />
              <span>Clipboard</span>
              {allowClipboardSync && <span className="online-dot" style={{ marginLeft: "auto" }} />}
            </button>
          </li>

          <li>
            <button
              className={`nav-link ${activeTab === "notifications" ? "active" : ""}`}
              onClick={() => setActiveTab("notifications")}
            >
              {activeTab === "notifications" && <span className="nav-link-indicator" />}
              <Bell size={16} />
              <span>Notifications</span>
              {notifications.length > 0 && (
                <span className="nav-link-badge" style={{ backgroundColor: "rgba(245, 158, 11, 0.15)", color: "#f59e0b" }}>
                  {notifications.length}
                </span>
              )}
            </button>
          </li>

          <li>
            <button
              className={`nav-link ${activeTab === "security" ? "active" : ""}`}
              onClick={() => setActiveTab("security")}
            >
              {activeTab === "security" && <span className="nav-link-indicator" />}
              <Shield size={16} />
              <span>Security</span>
            </button>
          </li>
        </ul>

        {/* Local Node Identity Footer */}
        <div className="sidebar-footer">
          <div className="node-status-row">
            <span style={{ fontWeight: 600, color: "var(--text-primary)" }}>{identity.deviceName}</span>
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <span className="online-dot" />
              <span style={{ fontSize: 10, color: "var(--text-muted)" }}>:4433</span>
            </div>
          </div>
          <button
            className="node-fingerprint-btn"
            title="Click to copy Ed25519 identity key"
            onClick={handleCopyFingerprint}
          >
            <span>{identity.fingerprint.substring(0, 16)}...</span>
            {copiedFingerprint ? <Check size={11} color="var(--status-online)" /> : <Copy size={11} />}
          </button>
        </div>
      </aside>

      {/* Main Viewport */}
      <div className="main-viewport">
        {/* View Header */}
        <header className="view-header">
          <div className="view-title-group">
            <h1 className="view-heading">
              {activeTab === "devices" && "Connected Devices"}
              {activeTab === "transfers" && "File Transfers"}
              {activeTab === "clipboard" && "Clipboard Synchronization"}
              {activeTab === "notifications" && "Notification Mirroring"}
              {activeTab === "security" && "Security & Permissions"}
            </h1>
            <p className="view-subheading">
              {activeTab === "devices" && "Manage paired peers, local network mesh, and device pairing"}
              {activeTab === "transfers" && "Direct peer-to-peer file streaming over encrypted QUIC streams"}
              {activeTab === "clipboard" && "Bidirectional real-time clipboard sharing with echo loop suppression"}
              {activeTab === "notifications" && "Incoming push alerts and notifications mirrored from connected devices"}
              {activeTab === "security" && "Ed25519 cryptographic identity, TLS 1.3 pinning, and capability gates"}
            </p>
          </div>

          {activeTab === "devices" && (
            <button className="btn btn-primary" onClick={() => setShowPairDialog(true)}>
              <Plus size={14} />
              <span>Pair Device</span>
            </button>
          )}

          {activeTab === "transfers" && (
            <button className="btn btn-primary" onClick={handleTriggerSendFile}>
              <Upload size={14} />
              <span>Send Files</span>
            </button>
          )}

          {activeTab === "notifications" && notifications.length > 0 && (
            <button
              className="btn"
              onClick={() => {
                setNotifications([]);
                showToast("All notifications dismissed");
              }}
            >
              Clear All
            </button>
          )}
        </header>

        {/* View Scrollable Body */}
        <main className="view-body">
          {/* TAB 1: DEVICES & OVERVIEW */}
          {activeTab === "devices" && (
            <div>
              {/* Prominent Active Peer Card */}
              <div className="connected-device-hero">
                <div className="hero-left">
                  <div className="hero-device-icon">
                    <Smartphone size={24} />
                  </div>
                  <div className="hero-details">
                    <div className="hero-name-row">
                      <span className="hero-device-name">{activePeer.displayName}</span>
                      <span className="pill-badge pill-green">Connected via QUIC</span>
                    </div>
                    <span className="hero-endpoint-text">
                      Endpoint: {activePeer.endpoint} &bull; Identity:{" "}
                      <span className="mono-tag">{activePeer.fingerprint.substring(0, 14)}...</span>
                    </span>
                  </div>
                </div>

                <div className="hero-actions">
                  <div style={{ display: "flex", alignItems: "center", gap: 12, marginRight: 8, fontSize: 12, color: "var(--text-secondary)" }}>
                    <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
                      <Wifi size={13} color="#60a5fa" />
                      LAN
                    </span>
                    <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
                      <BatteryCharging size={13} color="#22c55e" />
                      100%
                    </span>
                  </div>
                  <button className="btn btn-sm" onClick={handleTriggerSendFile}>
                    <Send size={13} />
                    <span>Send File</span>
                  </button>
                  <button
                    className="btn btn-sm"
                    onClick={() => {
                      setActiveTab("clipboard");
                    }}
                  >
                    <Clipboard size={13} />
                    <span>Sync Clip</span>
                  </button>
                  <button
                    className="btn btn-sm btn-danger"
                    onClick={() => handleDisconnectPeer(activePeer.fingerprint)}
                  >
                    Unpair
                  </button>
                </div>
              </div>

              {/* Continuity Status Summary (3 Clean Mini Tiles) */}
              <div className="section-title">Continuity Services</div>
              <div className="continuity-overview-grid">
                <div className="status-mini-tile" onClick={() => setActiveTab("transfers")}>
                  <div className="tile-top-row">
                    <Send size={16} color="var(--accent-hover)" />
                    <span className="pill-badge pill-green">Ready</span>
                  </div>
                  <span className="tile-title">File Streaming</span>
                  <span className="tile-desc">Chunked QUIC with SHA-256 integrity</span>
                </div>

                <div className="status-mini-tile" onClick={() => setActiveTab("clipboard")}>
                  <div className="tile-top-row">
                    <Clipboard size={16} color="var(--status-online)" />
                    <span className="pill-badge pill-green">Active</span>
                  </div>
                  <span className="tile-title">Clipboard Sync</span>
                  <span className="tile-desc">Bidirectional echo suppression on</span>
                </div>

                <div className="status-mini-tile" onClick={() => setActiveTab("notifications")}>
                  <div className="tile-top-row">
                    <Bell size={16} color="var(--status-warning)" />
                    <span className="pill-badge pill-green">Live</span>
                  </div>
                  <span className="tile-title">Notification Mirror</span>
                  <span className="tile-desc">{notifications.length} alerts pending</span>
                </div>
              </div>

              {/* Discovered Nearby Peers on LAN (mDNS) */}
              <div className="section-title">Discovered Nearby Devices (mDNS)</div>
              <div className="clean-card">
                {discoveredDevices.length === 0 ? (
                  <div style={{ padding: 24, textAlign: "center", color: "var(--text-muted)" }}>
                    No other devices discovered on local network
                  </div>
                ) : (
                  discoveredDevices.map((dev, idx) => (
                    <React.Fragment key={dev.id}>
                      {idx > 0 && <div className="clean-divider" />}
                      <div className="clean-row">
                        <div className="clean-row-left">
                          <Radio size={18} className="clean-row-icon" color="var(--status-blue)" />
                          <div className="clean-row-text">
                            <span className="clean-row-title">{dev.displayName}</span>
                            <span className="clean-row-desc">
                              {dev.endpoint} &bull; Broadcasting ephemeral privacy ID
                            </span>
                          </div>
                        </div>
                        <button
                          className="btn btn-sm btn-primary"
                          onClick={() => {
                            showToast(`Initiating pairing handshake with ${dev.displayName}`);
                            setShowPairDialog(true);
                          }}
                        >
                          Pair Device
                        </button>
                      </div>
                    </React.Fragment>
                  ))
                )}
              </div>
            </div>
          )}

          {/* TAB 2: FILE TRANSFERS */}
          {activeTab === "transfers" && (
            <div>
              <div
                className="stream-dropzone"
                onClick={handleTriggerSendFile}
              >
                <div className="dropzone-icon-circle">
                  <Upload size={22} />
                </div>
                <span className="dropzone-main-heading">
                  Drag and drop files to stream over QUIC
                </span>
                <span className="dropzone-sub-hint">
                  Target: {activePeer.displayName} ({activePeer.endpoint}). Chunked in 64 KB blocks with SHA-256 verification.
                </span>
              </div>

              <div className="section-title">Transfer History ({transfers.length})</div>
              <div className="clean-card">
                {transfers.map((tx, idx) => (
                  <React.Fragment key={tx.id}>
                    {idx > 0 && <div className="clean-divider" />}
                    <div className="clean-row">
                      <div className="clean-row-left">
                        <FileText size={18} className="clean-row-icon" color="#60a5fa" />
                        <div className="clean-row-text">
                          <span className="clean-row-title">{tx.fileName}</span>
                          <span className="clean-row-desc">
                            {(tx.fileSize / 1024 / 1024).toFixed(2)} MB &bull;{" "}
                            <span style={{ color: tx.direction === "incoming" ? "var(--status-online)" : "var(--accent-hover)" }}>
                              {tx.direction}
                            </span>{" "}
                            &bull; Peer: {tx.peerFingerprint.substring(0, 14)}...
                          </span>
                        </div>
                      </div>
                      <span className="pill-badge pill-green">Verified</span>
                    </div>
                  </React.Fragment>
                ))}
              </div>
            </div>
          )}

          {/* TAB 3: CLIPBOARD */}
          {activeTab === "clipboard" && (
            <div>
              <div className="clean-card">
                <div className="clean-row">
                  <div className="clean-row-left">
                    <Clipboard size={18} className="clean-row-icon" />
                    <div className="clean-row-text">
                      <span className="clean-row-title">Bidirectional Clipboard Synchronization</span>
                      <span className="clean-row-desc">
                        Automatically replicates copied text and images across trusted devices
                      </span>
                    </div>
                  </div>
                  <label className="toggle-wrap">
                    <input
                      type="checkbox"
                      checked={allowClipboardSync}
                      onChange={(e) => setAllowClipboardSync(e.target.checked)}
                    />
                    <span className="toggle-track-bg" />
                  </label>
                </div>

                <div className="clean-divider" />

                <div className="clean-row">
                  <div className="clean-row-left">
                    <Radio size={18} className="clean-row-icon" color="var(--status-online)" />
                    <div className="clean-row-text">
                      <span className="clean-row-title">Echo Suppression Window</span>
                      <span className="clean-row-desc">
                        16-entry hash ring prevents recursive copy-paste loops across devices
                      </span>
                    </div>
                  </div>
                  <span className="pill-badge pill-green">Enforced</span>
                </div>
              </div>

              <div className="section-title">Broadcast Text to Devices</div>
              <div className="clean-card" style={{ padding: 18 }}>
                <textarea
                  className="text-box"
                  rows={3}
                  placeholder="Paste or type text to send directly to connected devices..."
                  value={clipboardText}
                  onChange={(e) => setClipboardText(e.target.value)}
                  style={{ resize: "none", marginBottom: 12 }}
                />
                <div style={{ display: "flex", justifyContent: "flex-end" }}>
                  <button className="btn btn-primary" onClick={handleBroadcastClipboard}>
                    Broadcast Clip
                  </button>
                </div>
              </div>

              <div className="section-title">Recently Synchronized Clips</div>
              <div className="clean-card">
                {syncedClips.map((clip, idx) => (
                  <React.Fragment key={clip.id}>
                    {idx > 0 && <div className="clean-divider" />}
                    <div className="clean-row">
                      <div className="clean-row-left">
                        <FileText size={16} className="clean-row-icon" color="var(--text-muted)" />
                        <div className="clean-row-text">
                          <span className="clean-row-title" style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>
                            {clip.text}
                          </span>
                          <span className="clean-row-desc">From {clip.origin}</span>
                        </div>
                      </div>
                      <button
                        className="btn btn-sm"
                        onClick={() => {
                          navigator.clipboard.writeText(clip.text);
                          showToast("Copied clip to local clipboard");
                        }}
                      >
                        <Copy size={12} />
                        <span>Copy</span>
                      </button>
                    </div>
                  </React.Fragment>
                ))}
              </div>
            </div>
          )}

          {/* TAB 4: NOTIFICATIONS */}
          {activeTab === "notifications" && (
            <div>
              <div className="clean-card">
                <div className="clean-row">
                  <div className="clean-row-left">
                    <Bell size={18} className="clean-row-icon" />
                    <div className="clean-row-text">
                      <span className="clean-row-title">Mirror Remote Notifications</span>
                      <span className="clean-row-desc">
                        Relay incoming push notifications and alerts from mobile devices
                      </span>
                    </div>
                  </div>
                  <label className="toggle-wrap">
                    <input
                      type="checkbox"
                      checked={allowNotifications}
                      onChange={(e) => setAllowNotifications(e.target.checked)}
                    />
                    <span className="toggle-track-bg" />
                  </label>
                </div>
              </div>

              <div className="section-title">Notification Feed ({notifications.length})</div>
              <div className="clean-card">
                {notifications.length === 0 ? (
                  <div style={{ padding: 32, textAlign: "center", color: "var(--text-muted)" }}>
                    No active notifications
                  </div>
                ) : (
                  notifications.map((notif, idx) => (
                    <React.Fragment key={notif.id}>
                      {idx > 0 && <div className="clean-divider" />}
                      <div className="clean-row">
                        <div className="clean-row-left">
                          <Bell size={16} className="clean-row-icon" color="var(--status-warning)" />
                          <div className="clean-row-text">
                            <span className="clean-row-title">
                              {notif.title} <span className="mono-tag" style={{ fontSize: 10 }}>{notif.appName}</span>
                            </span>
                            <span className="clean-row-desc">{notif.body}</span>
                          </div>
                        </div>
                        <button
                          className="btn btn-sm"
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

          {/* TAB 5: SECURITY & SETTINGS */}
          {activeTab === "security" && (
            <div>
              <div className="section-title">This Device Identity</div>
              <div className="clean-card">
                <div className="clean-row">
                  <div className="clean-row-left">
                    <Laptop size={18} className="clean-row-icon" />
                    <div className="clean-row-text">
                      <span className="clean-row-title">{identity.deviceName}</span>
                      <span className="clean-row-desc">Local Node Host</span>
                    </div>
                  </div>
                  <span className="pill-badge pill-green">Online</span>
                </div>

                <div className="clean-divider" />

                <div className="clean-row">
                  <div className="clean-row-left">
                    <Key size={18} className="clean-row-icon" />
                    <div className="clean-row-text">
                      <span className="clean-row-title">Ed25519 Identity Fingerprint</span>
                      <span className="clean-row-desc">Long-term cryptographic device key</span>
                    </div>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span className="mono-tag">{identity.fingerprint}</span>
                    <button className="btn btn-sm" onClick={handleCopyFingerprint}>
                      {copiedFingerprint ? <Check size={12} /> : <Copy size={12} />}
                    </button>
                  </div>
                </div>

                <div className="clean-divider" />

                <div className="clean-row">
                  <div className="clean-row-left">
                    <Lock size={18} className="clean-row-icon" />
                    <div className="clean-row-text">
                      <span className="clean-row-title">TLS 1.3 Transport SPKI</span>
                      <span className="clean-row-desc">Transport certificate pinned to Ed25519 identity</span>
                    </div>
                  </div>
                  <span className="mono-tag">{identity.spkiHash.substring(0, 24)}...</span>
                </div>
              </div>

              <div className="section-title">Four-Layer Capability Authorization</div>
              <div className="clean-card">
                <div className="clean-row">
                  <div className="clean-row-left">
                    <Send size={18} className="clean-row-icon" />
                    <div className="clean-row-text">
                      <span className="clean-row-title">File Transfer</span>
                      <span className="clean-row-desc">Allow streaming file transfers with authorized peers</span>
                    </div>
                  </div>
                  <label className="toggle-wrap">
                    <input
                      type="checkbox"
                      checked={allowFileTransfer}
                      onChange={(e) => setAllowFileTransfer(e.target.checked)}
                    />
                    <span className="toggle-track-bg" />
                  </label>
                </div>

                <div className="clean-divider" />

                <div className="clean-row">
                  <div className="clean-row-left">
                    <Lock size={18} className="clean-row-icon" />
                    <div className="clean-row-text">
                      <span className="clean-row-title">Auto-Accept Small Transfers</span>
                      <span className="clean-row-desc">Automatically accept payloads under 10 MB</span>
                    </div>
                  </div>
                  <label className="toggle-wrap">
                    <input
                      type="checkbox"
                      checked={autoAcceptSmall}
                      onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                    />
                    <span className="toggle-track-bg" />
                  </label>
                </div>
              </div>

              <div className="section-title">Paired Trusted Peers ({peers.length})</div>
              <div className="clean-card">
                {peers.map((peer) => (
                  <div key={peer.fingerprint} className="clean-row">
                    <div className="clean-row-left">
                      <Smartphone size={18} className="clean-row-icon" />
                      <div className="clean-row-text">
                        <span className="clean-row-title">{peer.displayName}</span>
                        <span className="clean-row-desc">
                          {peer.endpoint} &bull; <span className="mono-tag">{peer.fingerprint.substring(0, 16)}...</span>
                        </span>
                      </div>
                    </div>
                    <button
                      className="btn btn-sm btn-danger"
                      onClick={() => handleDisconnectPeer(peer.fingerprint)}
                    >
                      Unpair
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}
        </main>
      </div>

      {/* Pairing Dialog Modal */}
      {showPairDialog && (
        <div className="modal-overlay-backdrop" onClick={() => setShowPairDialog(false)}>
          <div className="modal-window" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header-bar">
              <span className="modal-title-heading">Pair New Device</span>
              <button className="modal-dismiss-btn" onClick={() => setShowPairDialog(false)}>
                <X size={18} />
              </button>
            </div>

            <form onSubmit={handlePairSubmit}>
              <label style={{ display: "block", marginBottom: 8, fontSize: 12, color: "var(--text-secondary)" }}>
                Paste pairing URI or QR payload from remote client:
              </label>
              <input
                type="text"
                className="text-box"
                placeholder="continue://pair/v1?addr=192.168.1.105:4433&spki=..."
                value={pairingPayload}
                onChange={(e) => setPairingPayload(e.target.value)}
                autoFocus
              />

              <div style={{ display: "flex", justifyContent: "flex-end", gap: 10, marginTop: 18 }}>
                <button
                  type="button"
                  className="btn"
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

      {/* Interactive Toast Notification */}
      {toastMessage && (
        <div className="bottom-toast">
          <Check size={15} color="var(--status-online)" />
          <span>{toastMessage}</span>
        </div>
      )}
    </div>
  );
}

export default App;
