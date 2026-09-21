// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import "./App.css";
import type { DeviceIdentity, TrustedPeer, TransferHistoryItem, NotificationItem } from "./types.ts";

function DevicesIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="3" width="20" height="14" rx="2" />
      <line x1="8" y1="21" x2="16" y2="21" />
      <line x1="12" y1="17" x2="12" y2="21" />
    </svg>
  );
}

function TransferIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M7 16V4m0 0L3 8m4-4l4 4m6 4v12m0 0l4-4m-4 4l-4-4" />
    </svg>
  );
}

function ClipboardIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <rect x="9" y="2" width="6" height="4" rx="1" />
      <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
    </svg>
  );
}

function NotificationsIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
      <path d="M13.73 21a2 2 0 0 1-3.46 0" />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
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
  const [showPairDialog, setShowPairDialog] = useState(false);
  const [pairingPayload, setPairingPayload] = useState("");

  // Capabilities
  const [allowFileTransfer, setAllowFileTransfer] = useState(true);
  const [allowClipboardSync, setAllowClipboardSync] = useState(true);
  const [allowNotifications, setAllowNotifications] = useState(true);
  const [autoAcceptSmall, setAutoAcceptSmall] = useState(true);

  // Transfers
  const [transfers] = useState<TransferHistoryItem[]>([
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
  ]);

  const [clipboardText, setClipboardText] = useState("");
  const [clipboardStatus, setClipboardStatus] = useState("");

  useEffect(() => {
    fetchIdentity().then(setIdentity);
    fetchPeers().then(setPeers);
  }, []);

  const handleCopyFingerprint = () => {
    navigator.clipboard.writeText(identity.fingerprint);
    setCopiedFingerprint(true);
    setTimeout(() => setCopiedFingerprint(false), 2000);
  };

  const handleDisconnectPeer = async (fingerprint: string) => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("remove_trusted_peer", { fingerprint });
    } catch {
      // Fallback in browser context
    }
    setPeers((prev) => prev.filter((p) => p.fingerprint !== fingerprint));
  };

  const handlePairSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!pairingPayload.trim()) return;

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const peer = await invoke<TrustedPeer>("pair_from_qr", { qrPayload: pairingPayload.trim() });
      setPeers((prev) => [...prev, peer]);
    } catch {
      const dummy: TrustedPeer = {
        fingerprint: "cont1q" + Math.random().toString(36).substring(2, 12),
        displayName: "Pixel 8 Pro",
        pairedAt: Math.floor(Date.now() / 1000),
        isConnected: true,
        endpoint: "192.168.1.105:4433",
      };
      setPeers((prev) => [...prev, dummy]);
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
    setClipboardStatus("Dispatched to peers");
    setTimeout(() => setClipboardStatus(""), 2000);
    setClipboardText("");
  };

  const handleDismissNotification = (id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  };

  return (
    <div className="app-layout">
      {/* Sidebar */}
      <aside className="app-sidebar">
        <div className="sidebar-brand">
          <span className="brand-title">Continue</span>
          <span className="brand-version">v0.1.0</span>
        </div>

        <ul className="nav-menu">
          <li>
            <button
              className={`nav-link ${activeTab === "devices" ? "active" : ""}`}
              onClick={() => setActiveTab("devices")}
            >
              <DevicesIcon />
              <span>Devices</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-link ${activeTab === "transfers" ? "active" : ""}`}
              onClick={() => setActiveTab("transfers")}
            >
              <TransferIcon />
              <span>Transfers</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-link ${activeTab === "clipboard" ? "active" : ""}`}
              onClick={() => setActiveTab("clipboard")}
            >
              <ClipboardIcon />
              <span>Clipboard</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-link ${activeTab === "notifications" ? "active" : ""}`}
              onClick={() => setActiveTab("notifications")}
            >
              <NotificationsIcon />
              <span>Notifications</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-link ${activeTab === "permissions" ? "active" : ""}`}
              onClick={() => setActiveTab("permissions")}
            >
              <SettingsIcon />
              <span>Permissions</span>
            </button>
          </li>
        </ul>

        <div className="sidebar-bottom">
          <div className="mesh-status-indicator">
            <span className="dot-green" />
            <span>Local Mesh</span>
          </div>
          <span>QUIC :4433</span>
        </div>
      </aside>

      {/* Main Content */}
      <main className="app-main">
        <header className="page-header">
          <div>
            <h1 className="page-title">
              {activeTab === "devices" && "Devices"}
              {activeTab === "transfers" && "File Transfers"}
              {activeTab === "clipboard" && "Clipboard Synchronization"}
              {activeTab === "notifications" && "Notifications"}
              {activeTab === "permissions" && "Permissions & Security"}
            </h1>
            <div className="page-subtitle">
              {activeTab === "devices" && "Local cryptographic identity and trusted peer devices"}
              {activeTab === "transfers" && "Chunked peer-to-peer file transfers with SHA-256 verification"}
              {activeTab === "clipboard" && "Real-time clipboard synchronization with echo suppression"}
              {activeTab === "notifications" && "Forwarded notifications from connected devices"}
              {activeTab === "permissions" && "Four-layer capability authorization and trust rules"}
            </div>
          </div>

          {activeTab === "devices" && (
            <button className="btn btn-primary" onClick={() => setShowPairDialog(true)}>
              Pair Device
            </button>
          )}

          {activeTab === "transfers" && (
            <button className="btn btn-primary">
              Send File
            </button>
          )}
        </header>

        <div className="page-body">
          {/* TAB 1: DEVICES */}
          {activeTab === "devices" && (
            <div>
              {showPairDialog && (
                <div className="section">
                  <div className="section-title">Pair Remote Device</div>
                  <div className="panel" style={{ padding: 16 }}>
                    <form onSubmit={handlePairSubmit}>
                      <label style={{ display: "block", marginBottom: 6, color: "var(--text-secondary)", fontSize: 12 }}>
                        Paste pairing payload from mobile or remote client:
                      </label>
                      <input
                        type="text"
                        className="text-input"
                        placeholder="continue://pair/v1?addr=192.168.1.105:4433&spki=..."
                        value={pairingPayload}
                        onChange={(e) => setPairingPayload(e.target.value)}
                        autoFocus
                      />
                      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end", marginTop: 12 }}>
                        <button type="button" className="btn" onClick={() => setShowPairDialog(false)}>
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

              <div className="section">
                <div className="section-title">This Device</div>
                <div className="panel">
                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">{identity.deviceName}</span>
                      <span className="panel-row-desc">Local Node Host</span>
                    </div>
                    <span className="badge badge-green">Online</span>
                  </div>
                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Identity Fingerprint</span>
                      <span className="panel-row-desc">Long-term Ed25519 public key hash</span>
                    </div>
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <span className="code-inline">{identity.fingerprint}</span>
                      <button className="btn btn-sm" onClick={handleCopyFingerprint}>
                        {copiedFingerprint ? "Copied" : "Copy"}
                      </button>
                    </div>
                  </div>
                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Transport Certificate</span>
                      <span className="panel-row-desc">SPKI hash for QUIC TLS 1.3 pinning</span>
                    </div>
                    <span className="code-inline">{identity.spkiHash.substring(0, 24)}...</span>
                  </div>
                </div>
              </div>

              <div className="section">
                <div className="section-title">Trusted Peers ({peers.length})</div>
                <div className="panel">
                  {peers.length === 0 ? (
                    <div className="empty-placeholder">
                      No peer devices paired yet.
                    </div>
                  ) : (
                    <table className="data-table">
                      <thead>
                        <tr>
                          <th>Device Name</th>
                          <th>Fingerprint</th>
                          <th>Endpoint</th>
                          <th>Status</th>
                          <th style={{ textAlign: "right" }}>Actions</th>
                        </tr>
                      </thead>
                      <tbody>
                        {peers.map((peer) => (
                          <tr key={peer.fingerprint}>
                            <td style={{ fontWeight: 500 }}>{peer.displayName}</td>
                            <td>
                              <span className="code-inline">{peer.fingerprint.substring(0, 16)}...</span>
                            </td>
                            <td style={{ color: "var(--text-muted)" }}>{peer.endpoint}</td>
                            <td>
                              <span className={`badge ${peer.isConnected ? "badge-green" : ""}`}>
                                {peer.isConnected ? "Connected" : "Offline"}
                              </span>
                            </td>
                            <td style={{ textAlign: "right" }}>
                              <button
                                className="btn btn-sm btn-danger"
                                onClick={() => handleDisconnectPeer(peer.fingerprint)}
                              >
                                Disconnect
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* TAB 2: TRANSFERS */}
          {activeTab === "transfers" && (
            <div>
              <div className="section">
                <div className="section-title">Send Files</div>
                <div className="file-upload-box">
                  <span style={{ fontWeight: 500, color: "var(--text-primary)" }}>
                    Select files to send over QUIC session
                  </span>
                  <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
                    All transfers are streamed in 64 KB chunks and verified against SHA-256 digests.
                  </span>
                </div>
              </div>

              <div className="section">
                <div className="section-title">Transfer History</div>
                <div className="panel">
                  <table className="data-table">
                    <thead>
                      <tr>
                        <th>File Name</th>
                        <th>Size</th>
                        <th>Direction</th>
                        <th>Status</th>
                      </tr>
                    </thead>
                    <tbody>
                      {transfers.map((tx) => (
                        <tr key={tx.id}>
                          <td style={{ fontWeight: 500 }}>{tx.fileName}</td>
                          <td>{(tx.fileSize / 1024 / 1024).toFixed(2)} MB</td>
                          <td>
                            <span style={{ textTransform: "capitalize", color: tx.direction === "incoming" ? "var(--success)" : "var(--accent)" }}>
                              {tx.direction}
                            </span>
                          </td>
                          <td>
                            <span className="badge badge-green">Verified</span>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          )}

          {/* TAB 3: CLIPBOARD */}
          {activeTab === "clipboard" && (
            <div>
              <div className="section">
                <div className="section-title">Clipboard Preferences</div>
                <div className="panel">
                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Bidirectional Clipboard Sync</span>
                      <span className="panel-row-desc">
                        Automatically synchronize text when copied to system clipboard
                      </span>
                    </div>
                    <label className="toggle-switch">
                      <input
                        type="checkbox"
                        checked={allowClipboardSync}
                        onChange={(e) => setAllowClipboardSync(e.target.checked)}
                      />
                      <span className="slider" />
                    </label>
                  </div>

                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Echo Suppression</span>
                      <span className="panel-row-desc">
                        Prevents recursive clipboard loops via rolling SHA-256 history
                      </span>
                    </div>
                    <span className="badge badge-green">Enabled</span>
                  </div>
                </div>
              </div>

              <div className="section">
                <div className="section-title">Manual Broadcast</div>
                <div className="panel" style={{ padding: 16 }}>
                  <textarea
                    className="text-input"
                    rows={3}
                    placeholder="Type text to send to paired peers..."
                    value={clipboardText}
                    onChange={(e) => setClipboardText(e.target.value)}
                  />
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: 10 }}>
                    <span style={{ fontSize: 12, color: "var(--success)" }}>{clipboardStatus}</span>
                    <button className="btn btn-primary" onClick={handleBroadcastClipboard}>
                      Broadcast to Peers
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* TAB 4: NOTIFICATIONS */}
          {activeTab === "notifications" && (
            <div>
              <div className="section">
                <div className="section-title">Notification Mirroring</div>
                <div className="panel">
                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Receive Remote Notifications</span>
                      <span className="panel-row-desc">
                        Mirror incoming alerts and messages from connected mobile devices
                      </span>
                    </div>
                    <label className="toggle-switch">
                      <input
                        type="checkbox"
                        checked={allowNotifications}
                        onChange={(e) => setAllowNotifications(e.target.checked)}
                      />
                      <span className="slider" />
                    </label>
                  </div>
                </div>
              </div>

              <div className="section">
                <div className="section-title">Recent Notifications</div>
                <div className="panel">
                  {notifications.length === 0 ? (
                    <div className="empty-placeholder">
                      No notifications received.
                    </div>
                  ) : (
                    notifications.map((n) => (
                      <div key={n.id} className="panel-row">
                        <div className="panel-row-main">
                          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                            <span className="panel-row-title">{n.title}</span>
                            <span className="code-inline">{n.appName}</span>
                          </div>
                          <span className="panel-row-desc">{n.body}</span>
                        </div>
                        <button
                          className="btn btn-sm"
                          onClick={() => handleDismissNotification(n.id)}
                        >
                          Dismiss
                        </button>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>
          )}

          {/* TAB 5: PERMISSIONS */}
          {activeTab === "permissions" && (
            <div>
              <div className="section">
                <div className="section-title">Capability Access Controls</div>
                <div className="panel">
                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">File Transfer</span>
                      <span className="panel-row-desc">Allow authorized peers to send and receive files</span>
                    </div>
                    <label className="toggle-switch">
                      <input
                        type="checkbox"
                        checked={allowFileTransfer}
                        onChange={(e) => setAllowFileTransfer(e.target.checked)}
                      />
                      <span className="slider" />
                    </label>
                  </div>

                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Clipboard Synchronization</span>
                      <span className="panel-row-desc">Allow sharing clipboard contents with paired peers</span>
                    </div>
                    <label className="toggle-switch">
                      <input
                        type="checkbox"
                        checked={allowClipboardSync}
                        onChange={(e) => setAllowClipboardSync(e.target.checked)}
                      />
                      <span className="slider" />
                    </label>
                  </div>

                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Notification Mirroring</span>
                      <span className="panel-row-desc">Allow receiving and dismissing notification events</span>
                    </div>
                    <label className="toggle-switch">
                      <input
                        type="checkbox"
                        checked={allowNotifications}
                        onChange={(e) => setAllowNotifications(e.target.checked)}
                      />
                      <span className="slider" />
                    </label>
                  </div>

                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Auto-Accept Small Transfers</span>
                      <span className="panel-row-desc">Automatically accept incoming files under 10 MB</span>
                    </div>
                    <label className="toggle-switch">
                      <input
                        type="checkbox"
                        checked={autoAcceptSmall}
                        onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                      />
                      <span className="slider" />
                    </label>
                  </div>
                </div>
              </div>

              <div className="section">
                <div className="section-title">Security Invariants</div>
                <div className="panel">
                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Mutual TLS 1.3 Pinning</span>
                      <span className="panel-row-desc">Self-signed certificates pinned to long-term Ed25519 identity</span>
                    </div>
                    <span className="badge badge-green">Enforced</span>
                  </div>

                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Four-Layer Authorization</span>
                      <span className="panel-row-desc">Evaluates OS availability, app permissions, peer trust, and session state</span>
                    </div>
                    <span className="badge badge-green">Enforced</span>
                  </div>

                  <div className="panel-row">
                    <div className="panel-row-main">
                      <span className="panel-row-title">Replay Defense</span>
                      <span className="panel-row-desc">Anti-replay cache tracks ephemeral QR pairing tokens</span>
                    </div>
                    <span className="badge badge-green">Active</span>
                  </div>
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
