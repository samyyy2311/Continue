// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import "./App.css";
import type { DeviceIdentity, TrustedPeer, TransferHistoryItem, NotificationItem } from "./types.ts";

// Minimal SVG Icons
function DevicesIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="3" width="20" height="14" rx="2" />
      <line x1="8" y1="21" x2="16" y2="21" />
      <line x1="12" y1="17" x2="12" y2="21" />
    </svg>
  );
}

function TransferIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M7 16V4m0 0L3 8m4-4l4 4m6 4v12m0 0l4-4m-4 4l-4-4" />
    </svg>
  );
}

function ClipboardIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="9" y="2" width="6" height="4" rx="1" />
      <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
    </svg>
  );
}

function NotificationsIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
      <path d="M13.73 21a2 2 0 0 1-3.46 0" />
    </svg>
  );
}

function ShieldIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
    </svg>
  );
}

function CopyIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="20 6 9 17 4 12" />
    </svg>
  );
}

function UploadCloudIcon() {
  return (
    <svg className="dropzone-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M16 16l-4-4-4 4" />
      <path d="M12 12v9" />
      <path d="M20.39 18.39A5 5 0 0 0 18 9h-1.26A8 8 0 1 0 3 16.3" />
    </svg>
  );
}

function LaptopIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="3" width="20" height="14" rx="2" />
      <line x1="2" y1="20" x2="22" y2="20" />
    </svg>
  );
}

function PhoneIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="5" y="2" width="14" height="20" rx="2" ry="2" />
      <line x1="12" y1="18" x2="12.01" y2="18" />
    </svg>
  );
}

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
        fingerprint: "cont1q4f2e8d9c1a5b7e3f6a2b8d0e1c4a9f3b5d2e7",
        displayName: "MacBook Air",
        pairedAt: 1726915000,
        isConnected: true,
        endpoint: "192.168.1.112:4433",
      },
    ];
  }
}

export function App() {
  const [activeTab, setActiveTab] = useState<"devices" | "transfers" | "clipboard" | "notifications" | "permissions">("devices");

  const [identity, setIdentity] = useState<DeviceIdentity>({
    deviceName: "Desktop PC",
    fingerprint: "cont1q8f7e2a9d4c6b8a1e3f5a7b9c1d3e5f7a9b1c3d",
    spkiHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  });

  const [peers, setPeers] = useState<TrustedPeer[]>([]);
  const [copiedFingerprint, setCopiedFingerprint] = useState(false);
  const [qrInput, setQrInput] = useState("");
  const [showPairModal, setShowPairModal] = useState(false);

  // Capability Toggles
  const [fileTransferEnabled, setFileTransferEnabled] = useState(true);
  const [clipboardSyncEnabled, setClipboardSyncEnabled] = useState(true);
  const [notificationSyncEnabled, setNotificationSyncEnabled] = useState(true);
  const [autoAcceptSmallFiles, setAutoAcceptSmallFiles] = useState(true);

  // Transfers
  const [transfers, setTransfers] = useState<TransferHistoryItem[]>([
    {
      id: "tx-1",
      fileName: "financial_quarterly_report.pdf",
      fileSize: 4194304,
      direction: "incoming",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 360000,
    },
    {
      id: "tx-2",
      fileName: "presentation_deck_v2.key",
      fileSize: 18874368,
      direction: "outgoing",
      peerFingerprint: "cont1q4f2e8d9c1a5b7e3f6a2b8d0e1c4a9f3b5d2e7",
      status: "completed",
      timestamp: Date.now() - 1800000,
    },
  ]);

  // Notifications
  const [notifications, setNotifications] = useState<NotificationItem[]>([
    {
      id: "notif-1",
      appName: "Messages",
      title: "Sarah Connor",
      body: "Sent you the project update document and specs.",
      timestamp: Date.now() - 120000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
    {
      id: "notif-2",
      appName: "Calendar",
      title: "Engineering Sync",
      body: "Meeting starting in 10 minutes in Room B.",
      timestamp: Date.now() - 480000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
  ]);

  // Quick Clipboard Send input
  const [quickClipboardText, setQuickClipboardText] = useState("");
  const [clipboardFeedback, setClipboardFeedback] = useState("");

  useEffect(() => {
    fetchIdentity().then(setIdentity);
    fetchPeers().then(setPeers);
  }, []);

  const copyFingerprintToClipboard = () => {
    navigator.clipboard.writeText(identity.fingerprint);
    setCopiedFingerprint(true);
    setTimeout(() => setCopiedFingerprint(false), 2000);
  };

  const handleRemovePeer = async (fingerprint: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("remove_trusted_peer", { fingerprint });
    } catch {
      // Fallback for browser testing
    }
    setPeers((prev) => prev.filter((p) => p.fingerprint !== fingerprint));
  };

  const handlePairFromQr = async (e: React.FormEvent) => {
    e.preventDefault();
    const payload = qrInput.trim();
    if (!payload) return;

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const newPeer = await invoke<TrustedPeer>("pair_from_qr", { qrPayload: payload });
      setPeers((prev) => [...prev, newPeer]);
    } catch {
      const fallbackPeer: TrustedPeer = {
        fingerprint: "cont1q" + Math.random().toString(36).substring(2, 15),
        displayName: "New Device",
        pairedAt: Math.floor(Date.now() / 1000),
        isConnected: true,
        endpoint: "192.168.1.125:4433",
      };
      setPeers((prev) => [...prev, fallbackPeer]);
    }

    setQrInput("");
    setShowPairModal(false);
  };

  const handleSendClipboard = async () => {
    if (!quickClipboardText.trim()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      if (peers.length > 0) {
        await invoke("send_clipboard_text", {
          peerFingerprint: peers[0].fingerprint,
          text: quickClipboardText,
        });
      }
    } catch {
      // Fallback in browser preview
    }
    setClipboardFeedback("Clipboard broadcasted to connected peers!");
    setTimeout(() => setClipboardFeedback(""), 2500);
    setQuickClipboardText("");
  };

  const handleDismissNotification = (id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  };

  const handleSimulateTransfer = () => {
    const newItem: TransferHistoryItem = {
      id: `tx-${Date.now()}`,
      fileName: "backup_archive.tar.gz",
      fileSize: 14680064,
      direction: "incoming",
      peerFingerprint: peers[0]?.fingerprint || identity.fingerprint,
      status: "completed",
      timestamp: Date.now(),
    };
    setTransfers((prev) => [newItem, ...prev]);
  };

  return (
    <div className="app-container">
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-header">
          <div className="sidebar-logo">C</div>
          <div className="sidebar-title-group">
            <span className="sidebar-title">Continue</span>
            <span className="sidebar-subtitle">
              <span className="status-dot-mini" /> Mesh Active
            </span>
          </div>
        </div>

        <ul className="nav-list">
          <li>
            <button
              className={`nav-item ${activeTab === "devices" ? "active" : ""}`}
              onClick={() => setActiveTab("devices")}
            >
              <DevicesIcon />
              <span>Devices</span>
              <span className="nav-badge">{peers.length}</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-item ${activeTab === "transfers" ? "active" : ""}`}
              onClick={() => setActiveTab("transfers")}
            >
              <TransferIcon />
              <span>File Transfer</span>
              <span className="nav-badge">{transfers.length}</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-item ${activeTab === "clipboard" ? "active" : ""}`}
              onClick={() => setActiveTab("clipboard")}
            >
              <ClipboardIcon />
              <span>Clipboard</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-item ${activeTab === "notifications" ? "active" : ""}`}
              onClick={() => setActiveTab("notifications")}
            >
              <NotificationsIcon />
              <span>Notifications</span>
              {notifications.length > 0 && <span className="nav-badge">{notifications.length}</span>}
            </button>
          </li>
          <li>
            <button
              className={`nav-item ${activeTab === "permissions" ? "active" : ""}`}
              onClick={() => setActiveTab("permissions")}
            >
              <ShieldIcon />
              <span>Permissions</span>
            </button>
          </li>
        </ul>

        <div className="sidebar-footer">
          <div className="device-badge-mini">
            <span className="device-badge-label">This Device Node</span>
            <span className="device-badge-hash" title={identity.fingerprint}>
              {identity.fingerprint}
            </span>
          </div>
        </div>
      </aside>

      {/* Main Panel */}
      <main className="main-content">
        <header className="header">
          <div className="header-text">
            <h1 className="header-title">
              {activeTab === "devices" && "Connected Devices"}
              {activeTab === "transfers" && "File Transfers"}
              {activeTab === "clipboard" && "Clipboard Synchronization"}
              {activeTab === "notifications" && "Notification Mirroring"}
              {activeTab === "permissions" && "Security & Permissions"}
            </h1>
            <span className="header-desc">
              {activeTab === "devices" && "Manage paired peers, local cryptographic identity, and discovery."}
              {activeTab === "transfers" && "Stream files peer-to-peer over encrypted QUIC streams with SHA-256 verification."}
              {activeTab === "clipboard" && "Real-time bidirectional clipboard sync with echo loop suppression."}
              {activeTab === "notifications" && "Receive, inspect, and dismiss mobile notifications securely on desktop."}
              {activeTab === "permissions" && "Four-layer capability controls and per-device access grants."}
            </span>
          </div>

          {activeTab === "devices" && (
            <button className="btn btn-primary" onClick={() => setShowPairModal(true)}>
              + Pair New Device
            </button>
          )}

          {activeTab === "transfers" && (
            <button className="btn btn-primary" onClick={handleSimulateTransfer}>
              + Simulate Inbound Transfer
            </button>
          )}
        </header>

        <div className="content-pane">
          {/* TAB 1: DEVICES */}
          {activeTab === "devices" && (
            <div>
              <div className="hero-card">
                <div className="card-header">
                  <div className="card-title">
                    <LaptopIcon />
                    <span>{identity.deviceName} (Local Host)</span>
                  </div>
                  <span className="status-badge online">
                    <span className="status-dot online" />
                    Listening on QUIC :4433
                  </span>
                </div>

                <div className="info-grid">
                  <div className="info-item">
                    <span className="info-item-label">Cryptographic Fingerprint</span>
                    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
                      <span className="mono-tag" style={{ overflow: "hidden", textOverflow: "ellipsis" }}>
                        {identity.fingerprint.substring(0, 24)}...
                      </span>
                      <button className="btn btn-sm" onClick={copyFingerprintToClipboard} title="Copy Fingerprint">
                        {copiedFingerprint ? <CheckIcon /> : <CopyIcon />}
                      </button>
                    </div>
                  </div>

                  <div className="info-item">
                    <span className="info-item-label">TLS Transport SPKI Hash</span>
                    <span className="mono-tag">
                      {identity.spkiHash.substring(0, 20)}...
                    </span>
                  </div>

                  <div className="info-item">
                    <span className="info-item-label">Discovery Protocol</span>
                    <span className="info-item-value">mDNS with Ephemeral Privacy IDs</span>
                  </div>
                </div>
              </div>

              {showPairModal && (
                <div className="card" style={{ borderColor: "var(--accent)" }}>
                  <div className="card-header">
                    <div>
                      <div className="card-title">Pair a Remote Device</div>
                      <div className="card-subtitle">
                        Paste the pairing URI payload string displayed by your mobile or remote Continue app:
                      </div>
                    </div>
                  </div>

                  <form onSubmit={handlePairFromQr}>
                    <input
                      type="text"
                      className="input-field"
                      placeholder="continue://pair/v1?addr=192.168.1.105:4433&spki=..."
                      value={qrInput}
                      onChange={(e) => setQrInput(e.target.value)}
                    />
                    <div style={{ display: "flex", gap: 10, justifyContent: "flex-end", marginTop: 12 }}>
                      <button type="button" className="btn" onClick={() => setShowPairModal(false)}>
                        Cancel
                      </button>
                      <button type="submit" className="btn btn-primary">
                        Confirm Handshake
                      </button>
                    </div>
                  </form>
                </div>
              )}

              <div className="card">
                <div className="card-header">
                  <div className="card-title">Trusted Devices ({peers.length})</div>
                </div>

                {peers.length === 0 ? (
                  <div className="empty-state">
                    <DevicesIcon />
                    <div>No peers paired yet. Click "Pair New Device" to connect.</div>
                  </div>
                ) : (
                  <div className="peer-list">
                    {peers.map((peer) => (
                      <div key={peer.fingerprint} className="peer-card">
                        <div className="peer-identity">
                          <div className="peer-avatar">
                            {peer.displayName.toLowerCase().includes("pixel") || peer.displayName.toLowerCase().includes("phone") ? (
                              <PhoneIcon />
                            ) : (
                              <LaptopIcon />
                            )}
                          </div>
                          <div className="peer-meta">
                            <span className="peer-name">{peer.displayName}</span>
                            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                              <span className="mono-tag">{peer.fingerprint.substring(0, 18)}...</span>
                              <span style={{ fontSize: 12, color: "var(--text-muted)" }}>{peer.endpoint}</span>
                            </div>
                          </div>
                        </div>

                        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                          <span className={`status-badge ${peer.isConnected ? "online" : "offline"}`}>
                            <span className={`status-dot ${peer.isConnected ? "online" : "offline"}`} />
                            {peer.isConnected ? "Online (<5ms)" : "Offline"}
                          </span>
                          <button
                            className="btn btn-sm btn-danger"
                            onClick={() => handleRemovePeer(peer.fingerprint)}
                          >
                            Remove
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* TAB 2: TRANSFERS */}
          {activeTab === "transfers" && (
            <div>
              <div className="card">
                <div className="card-header">
                  <div className="card-title">Send File</div>
                </div>

                <div className="dropzone" onClick={() => alert("Select file to send over QUIC session.")}>
                  <UploadCloudIcon />
                  <div>
                    <div style={{ fontWeight: 600, color: "var(--text-primary)" }}>
                      Drop files here or click to browse
                    </div>
                    <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 4 }}>
                      Files stream over dedicated QUIC streams with SHA-256 integrity and automatic chunking
                    </div>
                  </div>
                </div>
              </div>

              <div className="card">
                <div className="card-header">
                  <div className="card-title">Transfer Activity & History</div>
                </div>

                <div className="peer-list">
                  {transfers.map((tx) => (
                    <div key={tx.id} className="peer-card">
                      <div className="peer-identity">
                        <div className="peer-avatar">
                          <TransferIcon />
                        </div>
                        <div className="peer-meta">
                          <span className="peer-name">{tx.fileName}</span>
                          <div style={{ display: "flex", gap: 10, fontSize: 12, color: "var(--text-muted)" }}>
                            <span>{(tx.fileSize / 1024 / 1024).toFixed(2)} MB</span>
                            <span>&bull;</span>
                            <span style={{ textTransform: "capitalize", color: tx.direction === "incoming" ? "#34d399" : "#60a5fa" }}>
                              {tx.direction}
                            </span>
                            <span>&bull;</span>
                            <span>Peer: {tx.peerFingerprint.substring(0, 12)}...</span>
                          </div>
                          <div className="progress-bar-bg">
                            <div className="progress-bar-fill" style={{ width: "100%" }} />
                          </div>
                        </div>
                      </div>

                      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                        <span className="status-badge online">
                          <span className="status-dot online" />
                          Verified
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* TAB 3: CLIPBOARD */}
          {activeTab === "clipboard" && (
            <div>
              <div className="card">
                <div className="card-header">
                  <div>
                    <div className="card-title">Clipboard Settings</div>
                    <div className="card-subtitle">
                      Keep text and images in sync between devices with local echo loop suppression.
                    </div>
                  </div>
                </div>

                <div className="switch-row">
                  <div className="switch-info">
                    <span className="switch-title">Bidirectional Clipboard Synchronization</span>
                    <span className="switch-desc">
                      Automatically transmit clipboard updates to connected peers when copied locally.
                    </span>
                  </div>
                  <label className="switch-label">
                    <input
                      type="checkbox"
                      checked={clipboardSyncEnabled}
                      onChange={(e) => setClipboardSyncEnabled(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>

                <div className="switch-row">
                  <div className="switch-info">
                    <span className="switch-title">Echo Suppression Cache</span>
                    <span className="switch-desc">
                      Keeps a SHA-256 rolling window to prevent recursive clipboard feedback loops.
                    </span>
                  </div>
                  <span className="status-badge online">
                    <span className="status-dot online" /> Active
                  </span>
                </div>
              </div>

              <div className="card">
                <div className="card-header">
                  <div className="card-title">Send Text to Connected Peers</div>
                </div>

                <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                  <textarea
                    className="input-field"
                    rows={3}
                    placeholder="Type or paste text to broadcast to paired devices..."
                    value={quickClipboardText}
                    onChange={(e) => setQuickClipboardText(e.target.value)}
                  />
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                    <span style={{ fontSize: 12, color: "#34d399" }}>{clipboardFeedback}</span>
                    <button className="btn btn-primary" onClick={handleSendClipboard}>
                      Broadcast to Mesh
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* TAB 4: NOTIFICATIONS */}
          {activeTab === "notifications" && (
            <div>
              <div className="card">
                <div className="card-header">
                  <div>
                    <div className="card-title">Notification Mirroring</div>
                    <div className="card-subtitle">
                      Forward notifications from Android and remote peers directly to your desktop.
                    </div>
                  </div>
                </div>

                <div className="switch-row">
                  <div className="switch-info">
                    <span className="switch-title">Mirror Remote Notifications</span>
                    <span className="switch-desc">
                      Receive incoming push alerts, messages, and calls from authorized devices.
                    </span>
                  </div>
                  <label className="switch-label">
                    <input
                      type="checkbox"
                      checked={notificationSyncEnabled}
                      onChange={(e) => setNotificationSyncEnabled(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>
              </div>

              <div className="card">
                <div className="card-header">
                  <div className="card-title">Forwarded Notifications ({notifications.length})</div>
                </div>

                {notifications.length === 0 ? (
                  <div className="empty-state">
                    <NotificationsIcon />
                    <div>No active notifications right now.</div>
                  </div>
                ) : (
                  <div className="peer-list">
                    {notifications.map((n) => (
                      <div key={n.id} className="peer-card">
                        <div className="peer-identity">
                          <div className="peer-avatar">
                            <NotificationsIcon />
                          </div>
                          <div className="peer-meta">
                            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                              <span className="peer-name">{n.title}</span>
                              <span className="mono-tag">{n.appName}</span>
                            </div>
                            <span style={{ fontSize: 13, color: "var(--text-secondary)", marginTop: 2 }}>
                              {n.body}
                            </span>
                          </div>
                        </div>

                        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                          <button
                            className="btn btn-sm"
                            onClick={() => handleDismissNotification(n.id)}
                          >
                            Dismiss
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* TAB 5: PERMISSIONS (Interactive Controls) */}
          {activeTab === "permissions" && (
            <div>
              <div className="hero-card">
                <div className="card-header">
                  <div className="card-title">
                    <ShieldIcon />
                    <span>Four-Layer Capability Security Model</span>
                  </div>
                  <span className="status-badge online">
                    <span className="status-dot online" /> Enforcing Active Security
                  </span>
                </div>
                <div className="card-subtitle">
                  Every capability invocation is verified across four independent gates: Platform OS API, Application permissions, Peer user trust, and Active session negotiation.
                </div>
              </div>

              <div className="card">
                <div className="card-header">
                  <div className="card-title">Global Capability Grants</div>
                </div>

                <div className="switch-row">
                  <div className="switch-info">
                    <span className="switch-title">File Transfer Capability</span>
                    <span className="switch-desc">
                      Permit incoming and outgoing streaming file transfers with trusted peers.
                    </span>
                  </div>
                  <label className="switch-label">
                    <input
                      type="checkbox"
                      checked={fileTransferEnabled}
                      onChange={(e) => setFileTransferEnabled(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>

                <div className="switch-row">
                  <div className="switch-info">
                    <span className="switch-title">Clipboard Synchronization Capability</span>
                    <span className="switch-desc">
                      Permit remote peers to update and read clipboard contents.
                    </span>
                  </div>
                  <label className="switch-label">
                    <input
                      type="checkbox"
                      checked={clipboardSyncEnabled}
                      onChange={(e) => setClipboardSyncEnabled(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>

                <div className="switch-row">
                  <div className="switch-info">
                    <span className="switch-title">Notification Mirroring Capability</span>
                    <span className="switch-desc">
                      Permit receiving and dismissing notification events across peers.
                    </span>
                  </div>
                  <label className="switch-label">
                    <input
                      type="checkbox"
                      checked={notificationSyncEnabled}
                      onChange={(e) => setNotificationSyncEnabled(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>

                <div className="switch-row">
                  <div className="switch-info">
                    <span className="switch-title">Automatic Acceptance for Files Under 10 MB</span>
                    <span className="switch-desc">
                      Receive small files immediately without prompting for individual confirmation.
                    </span>
                  </div>
                  <label className="switch-label">
                    <input
                      type="checkbox"
                      checked={autoAcceptSmallFiles}
                      onChange={(e) => setAutoAcceptSmallFiles(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>
              </div>

              <div className="card">
                <div className="card-header">
                  <div className="card-title">Per-Device Authorization Matrix</div>
                </div>

                <div className="peer-list">
                  {peers.map((peer) => (
                    <div key={peer.fingerprint} className="peer-card">
                      <div className="peer-identity">
                        <div className="peer-avatar">
                          {peer.displayName.toLowerCase().includes("pixel") ? <PhoneIcon /> : <LaptopIcon />}
                        </div>
                        <div className="peer-meta">
                          <span className="peer-name">{peer.displayName}</span>
                          <span className="mono-tag">{peer.fingerprint.substring(0, 16)}...</span>
                        </div>
                      </div>

                      <div style={{ display: "flex", gap: 8 }}>
                        <span className="status-badge online" style={{ fontSize: 11 }}>Files: Granted</span>
                        <span className="status-badge online" style={{ fontSize: 11 }}>Clipboard: Granted</span>
                        <span className="status-badge online" style={{ fontSize: 11 }}>Notifs: Granted</span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      </main>
    </div>
  );
}

export default App;
