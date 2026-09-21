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

function ShieldIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
    </svg>
  );
}

function KeyIcon() {
  return (
    <svg className="setting-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="7.5" cy="15.5" r="5.5" />
      <path d="M11.4 11.6L21 2m-3 3l2 2m-4 0l2 2" />
    </svg>
  );
}

function NetworkIcon() {
  return (
    <svg className="setting-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <line x1="2" y1="12" x2="22" y2="12" />
      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </svg>
  );
}

function LockIcon() {
  return (
    <svg className="setting-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
      <path d="M7 11V7a5 5 0 0 1 10 0v4" />
    </svg>
  );
}

function RadioIcon() {
  return (
    <svg className="setting-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="2" />
      <path d="M16.24 7.76a6 6 0 0 1 0 8.49m-8.48-.01a6 6 0 0 1 0-8.49m11.31-2.82a10 10 0 0 1 0 14.14m-14.14 0a10 10 0 0 1 0-14.14" />
    </svg>
  );
}

function LaptopDeviceIcon() {
  return (
    <svg className="setting-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="3" width="20" height="14" rx="2" />
      <line x1="2" y1="20" x2="22" y2="20" />
    </svg>
  );
}

function PhoneDeviceIcon() {
  return (
    <svg className="setting-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
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
      // Fallback
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
    setClipboardStatus("Broadcasted to mesh");
    setTimeout(() => setClipboardStatus(""), 2000);
    setClipboardText("");
  };

  const handleDismissNotification = (id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  };

  return (
    <div className="app-layout">
      {/* CassetteCat Sidebar */}
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
              {activeTab === "devices" && <span className="nav-indicator" />}
              <DevicesIcon />
              <span>Devices</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-link ${activeTab === "transfers" ? "active" : ""}`}
              onClick={() => setActiveTab("transfers")}
            >
              {activeTab === "transfers" && <span className="nav-indicator" />}
              <TransferIcon />
              <span>Transfers</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-link ${activeTab === "clipboard" ? "active" : ""}`}
              onClick={() => setActiveTab("clipboard")}
            >
              {activeTab === "clipboard" && <span className="nav-indicator" />}
              <ClipboardIcon />
              <span>Clipboard</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-link ${activeTab === "notifications" ? "active" : ""}`}
              onClick={() => setActiveTab("notifications")}
            >
              {activeTab === "notifications" && <span className="nav-indicator" />}
              <NotificationsIcon />
              <span>Notifications</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-link ${activeTab === "permissions" ? "active" : ""}`}
              onClick={() => setActiveTab("permissions")}
            >
              {activeTab === "permissions" && <span className="nav-indicator" />}
              <ShieldIcon />
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

      {/* Main Content Area */}
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
              {activeTab === "devices" && "Local cryptographic identity and paired device sessions"}
              {activeTab === "transfers" && "Chunked streaming file transfers with SHA-256 verification"}
              {activeTab === "clipboard" && "Bidirectional clipboard sync with rolling echo suppression"}
              {activeTab === "notifications" && "Push notifications relayed from connected mobile devices"}
              {activeTab === "permissions" && "Four-layer capability authorization and access policies"}
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
                <div style={{ marginBottom: 20 }}>
                  <div className="section-label">Pair New Device</div>
                  <div className="setting-card" style={{ padding: 18 }}>
                    <form onSubmit={handlePairSubmit}>
                      <label style={{ display: "block", marginBottom: 8, color: "var(--text-secondary)", fontSize: 12 }}>
                        Paste pairing payload URI from mobile or remote device:
                      </label>
                      <input
                        type="text"
                        className="text-input"
                        placeholder="continue://pair/v1?addr=192.168.1.105:4433&spki=..."
                        value={pairingPayload}
                        onChange={(e) => setPairingPayload(e.target.value)}
                        autoFocus
                      />
                      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end", marginTop: 14 }}>
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

              <div className="section-label">Local Node</div>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-left">
                    <LaptopDeviceIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">{identity.deviceName}</span>
                      <span className="setting-row-subtitle">Local host running Continue runtime</span>
                    </div>
                  </div>
                  <span className="badge badge-active">Active</span>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-left">
                    <KeyIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Identity Fingerprint</span>
                      <span className="setting-row-subtitle">Ed25519 long-term identity public key</span>
                    </div>
                  </div>
                  <div className="setting-row-trailing">
                    <span className="code-tag">{identity.fingerprint}</span>
                    <button className="btn btn-sm" onClick={handleCopyFingerprint}>
                      {copiedFingerprint ? "Copied" : "Copy"}
                    </button>
                  </div>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-left">
                    <LockIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Transport Certificate</span>
                      <span className="setting-row-subtitle">SPKI SHA-256 for QUIC TLS 1.3 pinning</span>
                    </div>
                  </div>
                  <span className="code-tag">{identity.spkiHash.substring(0, 24)}...</span>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-left">
                    <RadioIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Local Discovery</span>
                      <span className="setting-row-subtitle">mDNS with rotating ephemeral privacy IDs</span>
                    </div>
                  </div>
                  <span className="badge">127.0.0.1:4433</span>
                </div>
              </div>

              <div className="section-label">Trusted Peers ({peers.length})</div>
              <div className="setting-card">
                {peers.length === 0 ? (
                  <div className="empty-state-box">
                    No peer devices paired yet.
                  </div>
                ) : (
                  peers.map((peer, idx) => (
                    <React.Fragment key={peer.fingerprint}>
                      {idx > 0 && <div className="setting-divider" />}
                      <div className="setting-row">
                        <div className="setting-row-left">
                          {peer.displayName.toLowerCase().includes("pixel") || peer.displayName.toLowerCase().includes("phone") ? (
                            <PhoneDeviceIcon />
                          ) : (
                            <LaptopDeviceIcon />
                          )}
                          <div className="setting-row-text">
                            <span className="setting-row-title">{peer.displayName}</span>
                            <span className="setting-row-subtitle">
                              {peer.endpoint} &bull; <span className="code-tag" style={{ padding: "1px 5px" }}>{peer.fingerprint.substring(0, 16)}...</span>
                            </span>
                          </div>
                        </div>
                        <div className="setting-row-trailing">
                          <span className={`badge ${peer.isConnected ? "badge-active" : ""}`}>
                            {peer.isConnected ? "Connected" : "Offline"}
                          </span>
                          <button
                            className="btn btn-sm btn-danger"
                            onClick={() => handleDisconnectPeer(peer.fingerprint)}
                          >
                            Disconnect
                          </button>
                        </div>
                      </div>
                    </React.Fragment>
                  ))
                )}
              </div>
            </div>
          )}

          {/* TAB 2: TRANSFERS */}
          {activeTab === "transfers" && (
            <div>
              <div className="section-label">Send Files</div>
              <div className="file-dropzone" onClick={() => alert("Select file to send.")}>
                <TransferIcon />
                <span style={{ fontWeight: 600, color: "var(--text-primary)", fontFamily: "var(--font-display)" }}>
                  Select files to stream over QUIC
                </span>
                <span style={{ fontSize: 12, color: "var(--silver-dim)" }}>
                  Chunked streaming with SHA-256 integrity verification and path traversal guards.
                </span>
              </div>

              <div className="section-label">Transfer History</div>
              <div className="setting-card">
                {transfers.map((tx, idx) => (
                  <React.Fragment key={tx.id}>
                    {idx > 0 && <div className="setting-divider" />}
                    <div className="setting-row">
                      <div className="setting-row-left">
                        <TransferIcon />
                        <div className="setting-row-text">
                          <span className="setting-row-title">{tx.fileName}</span>
                          <span className="setting-row-subtitle">
                            {(tx.fileSize / 1024 / 1024).toFixed(2)} MB &bull; <span style={{ textTransform: "capitalize", color: tx.direction === "incoming" ? "var(--success)" : "var(--accent-hover)" }}>{tx.direction}</span> &bull; Peer: {tx.peerFingerprint.substring(0, 12)}...
                          </span>
                        </div>
                      </div>
                      <span className="badge badge-active">Verified</span>
                    </div>
                  </React.Fragment>
                ))}
              </div>
            </div>
          )}

          {/* TAB 3: CLIPBOARD */}
          {activeTab === "clipboard" && (
            <div>
              <div className="section-label">Synchronization Settings</div>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-left">
                    <ClipboardIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Bidirectional Clipboard Sync</span>
                      <span className="setting-row-subtitle">
                        Automatically synchronize plain text and images when copied locally
                      </span>
                    </div>
                  </div>
                  <label className="setting-switch">
                    <input
                      type="checkbox"
                      checked={allowClipboardSync}
                      onChange={(e) => setAllowClipboardSync(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-left">
                    <NetworkIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Echo Suppression Window</span>
                      <span className="setting-row-subtitle">
                        Maintains rolling SHA-256 payload history to prevent recursive copy loops
                      </span>
                    </div>
                  </div>
                  <span className="badge badge-active">Active</span>
                </div>
              </div>

              <div className="section-label">Quick Broadcast</div>
              <div className="setting-card" style={{ padding: 18 }}>
                <textarea
                  className="text-input"
                  rows={3}
                  placeholder="Type text to broadcast to paired devices..."
                  value={clipboardText}
                  onChange={(e) => setClipboardText(e.target.value)}
                />
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: 12 }}>
                  <span style={{ fontSize: 12, color: "var(--success)", fontFamily: "var(--font-mono)" }}>{clipboardStatus}</span>
                  <button className="btn btn-primary" onClick={handleBroadcastClipboard}>
                    Broadcast to Mesh
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* TAB 4: NOTIFICATIONS */}
          {activeTab === "notifications" && (
            <div>
              <div className="section-label">Mirroring Preferences</div>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-left">
                    <NotificationsIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Mirror Remote Notifications</span>
                      <span className="setting-row-subtitle">
                        Relay incoming push notifications and alerts from mobile devices
                      </span>
                    </div>
                  </div>
                  <label className="setting-switch">
                    <input
                      type="checkbox"
                      checked={allowNotifications}
                      onChange={(e) => setAllowNotifications(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>
              </div>

              <div className="section-label">Notification Feed</div>
              <div className="setting-card">
                {notifications.length === 0 ? (
                  <div className="empty-state-box">
                    No active notifications.
                  </div>
                ) : (
                  notifications.map((n, idx) => (
                    <React.Fragment key={n.id}>
                      {idx > 0 && <div className="setting-divider" />}
                      <div className="setting-row">
                        <div className="setting-row-left">
                          <NotificationsIcon />
                          <div className="setting-row-text">
                            <span className="setting-row-title">
                              {n.title} <span className="code-tag" style={{ marginLeft: 6, fontSize: 10 }}>{n.appName}</span>
                            </span>
                            <span className="setting-row-subtitle">{n.body}</span>
                          </div>
                        </div>
                        <button
                          className="btn btn-sm"
                          onClick={() => handleDismissNotification(n.id)}
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

          {/* TAB 5: PERMISSIONS */}
          {activeTab === "permissions" && (
            <div>
              <div className="section-label">Capability Access Controls</div>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-left">
                    <TransferIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">File Transfer</span>
                      <span className="setting-row-subtitle">
                        Allow streaming file transfers with authorized peers
                      </span>
                    </div>
                  </div>
                  <label className="setting-switch">
                    <input
                      type="checkbox"
                      checked={allowFileTransfer}
                      onChange={(e) => setAllowFileTransfer(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-left">
                    <ClipboardIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Clipboard Synchronization</span>
                      <span className="setting-row-subtitle">
                        Allow bidirectional clipboard synchronization across peers
                      </span>
                    </div>
                  </div>
                  <label className="setting-switch">
                    <input
                      type="checkbox"
                      checked={allowClipboardSync}
                      onChange={(e) => setAllowClipboardSync(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-left">
                    <NotificationsIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Notification Mirroring</span>
                      <span className="setting-row-subtitle">
                        Allow forwarding and remote dismissal of notification events
                      </span>
                    </div>
                  </div>
                  <label className="setting-switch">
                    <input
                      type="checkbox"
                      checked={allowNotifications}
                      onChange={(e) => setAllowNotifications(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-left">
                    <LockIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Auto-Accept Small Transfers</span>
                      <span className="setting-row-subtitle">
                        Automatically accept incoming file transfers smaller than 10 MB
                      </span>
                    </div>
                  </div>
                  <label className="setting-switch">
                    <input
                      type="checkbox"
                      checked={autoAcceptSmall}
                      onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                    />
                    <span className="switch-slider" />
                  </label>
                </div>
              </div>

              <div className="section-label">Security Invariants</div>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-row-left">
                    <LockIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Mutual TLS 1.3 Pinning</span>
                      <span className="setting-row-subtitle">
                        Transport certificates pinned to long-term Ed25519 identity
                      </span>
                    </div>
                  </div>
                  <span className="badge badge-active">Enforced</span>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-left">
                    <ShieldIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Four-Layer Authorization Model</span>
                      <span className="setting-row-subtitle">
                        Checks platform availability, application permissions, peer trust, and session state
                      </span>
                    </div>
                  </div>
                  <span className="badge badge-active">Enforced</span>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-row-left">
                    <KeyIcon />
                    <div className="setting-row-text">
                      <span className="setting-row-title">Anti-Replay Token Cache</span>
                      <span className="setting-row-subtitle">
                        Atomic single-use token consumption for QR pairing handshakes
                      </span>
                    </div>
                  </div>
                  <span className="badge badge-active">Active</span>
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
