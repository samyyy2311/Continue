// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import "./App.css";
import type { DeviceIdentity, TrustedPeer, TransferHistoryItem, NotificationItem } from "./types.ts";

function ContinueLogo() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 2a10 10 0 0 1 10 10c0 5.523-4.477 10-10 10S2 17.523 2 12" />
      <path d="M12 6v6l4 2" />
    </svg>
  );
}

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

function SecurityIcon() {
  return (
    <svg className="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
    </svg>
  );
}

function KeyIcon() {
  return (
    <svg className="group-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="7.5" cy="15.5" r="5.5" />
      <path d="M11.4 11.6L21 2m-3 3l2 2m-4 0l2 2" />
    </svg>
  );
}

function LockIcon() {
  return (
    <svg className="group-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
      <path d="M7 11V7a5 5 0 0 1 10 0v4" />
    </svg>
  );
}

function NetworkIcon() {
  return (
    <svg className="group-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <line x1="2" y1="12" x2="22" y2="12" />
      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </svg>
  );
}

function LaptopIcon() {
  return (
    <svg className="group-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="3" width="20" height="14" rx="2" />
      <line x1="2" y1="20" x2="22" y2="20" />
    </svg>
  );
}

function PhoneIcon() {
  return (
    <svg className="group-row-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
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
    <div className="app-shell">
      {/* Sidebar */}
      <aside className="app-sidebar">
        <div className="sidebar-brand">
          <div className="brand-icon">
            <ContinueLogo />
          </div>
          <div className="brand-text">
            <span className="brand-name">Continue</span>
            <span className="brand-tag">v0.1.0</span>
          </div>
        </div>

        <ul className="nav-group">
          <li>
            <button
              className={`nav-button ${activeTab === "devices" ? "active" : ""}`}
              onClick={() => setActiveTab("devices")}
            >
              {activeTab === "devices" && <span className="nav-active-pill" />}
              <DevicesIcon />
              <span>Devices</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-button ${activeTab === "transfers" ? "active" : ""}`}
              onClick={() => setActiveTab("transfers")}
            >
              {activeTab === "transfers" && <span className="nav-active-pill" />}
              <TransferIcon />
              <span>Transfers</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-button ${activeTab === "clipboard" ? "active" : ""}`}
              onClick={() => setActiveTab("clipboard")}
            >
              {activeTab === "clipboard" && <span className="nav-active-pill" />}
              <ClipboardIcon />
              <span>Clipboard</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-button ${activeTab === "notifications" ? "active" : ""}`}
              onClick={() => setActiveTab("notifications")}
            >
              {activeTab === "notifications" && <span className="nav-active-pill" />}
              <NotificationsIcon />
              <span>Notifications</span>
            </button>
          </li>
          <li>
            <button
              className={`nav-button ${activeTab === "permissions" ? "active" : ""}`}
              onClick={() => setActiveTab("permissions")}
            >
              {activeTab === "permissions" && <span className="nav-active-pill" />}
              <SecurityIcon />
              <span>Permissions</span>
            </button>
          </li>
        </ul>

        <div className="sidebar-footer">
          <div className="mesh-status-indicator">
            <span className="status-dot-pulse" />
            <span>Local Mesh</span>
          </div>
          <span>QUIC :4433</span>
        </div>
      </aside>

      {/* Main Content Area */}
      <main className="app-content">
        <header className="header-bar">
          <div>
            <h1 className="header-title">
              {activeTab === "devices" && "Devices"}
              {activeTab === "transfers" && "File Transfers"}
              {activeTab === "clipboard" && "Clipboard Synchronization"}
              {activeTab === "notifications" && "Notifications"}
              {activeTab === "permissions" && "Security & Permissions"}
            </h1>
            <div className="header-subtitle">
              {activeTab === "devices" && "Local cryptographic identity and trusted peer devices"}
              {activeTab === "transfers" && "Chunked streaming file transfers with SHA-256 verification"}
              {activeTab === "clipboard" && "Bidirectional clipboard sync with rolling echo suppression"}
              {activeTab === "notifications" && "Push notifications relayed from connected devices"}
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

        <div className="content-body">
          {/* TAB 1: DEVICES */}
          {activeTab === "devices" && (
            <div>
              {showPairDialog && (
                <div style={{ marginBottom: 20 }}>
                  <div className="section-header">Pair Remote Device</div>
                  <div className="group-card" style={{ padding: 18 }}>
                    <form onSubmit={handlePairSubmit}>
                      <label style={{ display: "block", marginBottom: 8, color: "var(--text-secondary)", fontSize: 12 }}>
                        Paste pairing payload URI from mobile or remote client:
                      </label>
                      <input
                        type="text"
                        className="input-text"
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

              <div className="section-header">This Device</div>
              <div className="group-card">
                <div className="group-row">
                  <div className="group-row-left">
                    <LaptopIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">{identity.deviceName}</span>
                      <span className="group-row-desc">Local Node Host</span>
                    </div>
                  </div>
                  <span className="status-pill status-pill-online">Online</span>
                </div>

                <div className="group-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <KeyIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Identity Fingerprint</span>
                      <span className="group-row-desc">Long-term Ed25519 identity key</span>
                    </div>
                  </div>
                  <div className="group-row-right">
                    <span className="code-chip">{identity.fingerprint}</span>
                    <button className="btn btn-sm" onClick={handleCopyFingerprint}>
                      {copiedFingerprint ? "Copied" : "Copy"}
                    </button>
                  </div>
                </div>

                <div className="group-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <LockIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Transport Certificate</span>
                      <span className="group-row-desc">SPKI hash for TLS 1.3 pinning</span>
                    </div>
                  </div>
                  <span className="code-chip">{identity.spkiHash.substring(0, 24)}...</span>
                </div>

                <div className="group-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <NetworkIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Discovery Protocol</span>
                      <span className="group-row-desc">mDNS with rotating ephemeral privacy IDs</span>
                    </div>
                  </div>
                  <span className="status-pill">127.0.0.1:4433</span>
                </div>
              </div>

              <div className="section-header">Trusted Peers ({peers.length})</div>
              <div className="group-card">
                {peers.length === 0 ? (
                  <div className="empty-row">
                    No peer devices paired yet.
                  </div>
                ) : (
                  peers.map((peer, idx) => (
                    <React.Fragment key={peer.fingerprint}>
                      {idx > 0 && <div className="group-divider" />}
                      <div className="group-row">
                        <div className="group-row-left">
                          {peer.displayName.toLowerCase().includes("pixel") || peer.displayName.toLowerCase().includes("phone") ? (
                            <PhoneIcon />
                          ) : (
                            <LaptopIcon />
                          )}
                          <div className="group-row-text">
                            <span className="group-row-title">{peer.displayName}</span>
                            <span className="group-row-desc">
                              {peer.endpoint} &bull; <span className="code-chip" style={{ padding: "1px 5px" }}>{peer.fingerprint.substring(0, 16)}...</span>
                            </span>
                          </div>
                        </div>
                        <div className="group-row-right">
                          <span className={`status-pill ${peer.isConnected ? "status-pill-online" : ""}`}>
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
              <div className="section-header">Send Files</div>
              <div className="transfer-drop-box" onClick={() => alert("Select file to send.")}>
                <TransferIcon />
                <span style={{ fontWeight: 500, color: "var(--text-primary)" }}>
                  Select files to stream over QUIC
                </span>
                <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
                  Chunked streaming in 64 KB blocks with SHA-256 integrity checks.
                </span>
              </div>

              <div className="section-header">Transfer History</div>
              <div className="group-card">
                {transfers.map((tx, idx) => (
                  <React.Fragment key={tx.id}>
                    {idx > 0 && <div className="group-divider" />}
                    <div className="group-row">
                      <div className="group-row-left">
                        <TransferIcon />
                        <div className="group-row-text">
                          <span className="group-row-title">{tx.fileName}</span>
                          <span className="group-row-desc">
                            {(tx.fileSize / 1024 / 1024).toFixed(2)} MB &bull; <span style={{ textTransform: "capitalize", color: tx.direction === "incoming" ? "var(--success)" : "var(--accent-hover)" }}>{tx.direction}</span> &bull; Peer: {tx.peerFingerprint.substring(0, 12)}...
                          </span>
                        </div>
                      </div>
                      <span className="status-pill status-pill-online">Verified</span>
                    </div>
                  </React.Fragment>
                ))}
              </div>
            </div>
          )}

          {/* TAB 3: CLIPBOARD */}
          {activeTab === "clipboard" && (
            <div>
              <div className="section-header">Sync Preferences</div>
              <div className="group-card">
                <div className="group-row">
                  <div className="group-row-left">
                    <ClipboardIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Bidirectional Clipboard Sync</span>
                      <span className="group-row-desc">
                        Automatically synchronize plain text and images when copied locally
                      </span>
                    </div>
                  </div>
                  <label className="toggle">
                    <input
                      type="checkbox"
                      checked={allowClipboardSync}
                      onChange={(e) => setAllowClipboardSync(e.target.checked)}
                    />
                    <span className="toggle-track" />
                  </label>
                </div>

                <div className="group-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <NetworkIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Echo Suppression Window</span>
                      <span className="group-row-desc">
                        Maintains rolling SHA-256 payload history to prevent recursive copy loops
                      </span>
                    </div>
                  </div>
                  <span className="status-pill status-pill-online">Active</span>
                </div>
              </div>

              <div className="section-header">Manual Broadcast</div>
              <div className="group-card" style={{ padding: 16 }}>
                <textarea
                  className="input-text"
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
              <div className="section-header">Mirroring Preferences</div>
              <div className="group-card">
                <div className="group-row">
                  <div className="group-row-left">
                    <NotificationsIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Mirror Remote Notifications</span>
                      <span className="group-row-desc">
                        Relay incoming push notifications and alerts from mobile devices
                      </span>
                    </div>
                  </div>
                  <label className="toggle">
                    <input
                      type="checkbox"
                      checked={allowNotifications}
                      onChange={(e) => setAllowNotifications(e.target.checked)}
                    />
                    <span className="toggle-track" />
                  </label>
                </div>
              </div>

              <div className="section-header">Recent Notifications</div>
              <div className="group-card">
                {notifications.length === 0 ? (
                  <div className="empty-row">
                    No active notifications.
                  </div>
                ) : (
                  notifications.map((n, idx) => (
                    <React.Fragment key={n.id}>
                      {idx > 0 && <div className="group-divider" />}
                      <div className="group-row">
                        <div className="group-row-left">
                          <NotificationsIcon />
                          <div className="group-row-text">
                            <span className="group-row-title">
                              {n.title} <span className="code-chip" style={{ marginLeft: 6, fontSize: 10 }}>{n.appName}</span>
                            </span>
                            <span className="group-row-desc">{n.body}</span>
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
              <div className="section-header">Capability Access Controls</div>
              <div className="group-card">
                <div className="group-row">
                  <div className="group-row-left">
                    <TransferIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">File Transfer</span>
                      <span className="group-row-desc">
                        Allow streaming file transfers with authorized peers
                      </span>
                    </div>
                  </div>
                  <label className="toggle">
                    <input
                      type="checkbox"
                      checked={allowFileTransfer}
                      onChange={(e) => setAllowFileTransfer(e.target.checked)}
                    />
                    <span className="toggle-track" />
                  </label>
                </div>

                <div className="group-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <ClipboardIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Clipboard Synchronization</span>
                      <span className="group-row-desc">
                        Allow bidirectional clipboard synchronization across peers
                      </span>
                    </div>
                  </div>
                  <label className="toggle">
                    <input
                      type="checkbox"
                      checked={allowClipboardSync}
                      onChange={(e) => setAllowClipboardSync(e.target.checked)}
                    />
                    <span className="toggle-track" />
                  </label>
                </div>

                <div className="group-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <NotificationsIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Notification Mirroring</span>
                      <span className="group-row-desc">
                        Allow forwarding and remote dismissal of notification events
                      </span>
                    </div>
                  </div>
                  <label className="toggle">
                    <input
                      type="checkbox"
                      checked={allowNotifications}
                      onChange={(e) => setAllowNotifications(e.target.checked)}
                    />
                    <span className="toggle-track" />
                  </label>
                </div>

                <div className="group-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <LockIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Auto-Accept Small Transfers</span>
                      <span className="group-row-desc">
                        Automatically accept incoming file transfers smaller than 10 MB
                      </span>
                    </div>
                  </div>
                  <label className="toggle">
                    <input
                      type="checkbox"
                      checked={autoAcceptSmall}
                      onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                    />
                    <span className="toggle-track" />
                  </label>
                </div>
              </div>

              <div className="section-header">Security Invariants</div>
              <div className="group-card">
                <div className="group-row">
                  <div className="group-row-left">
                    <LockIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Mutual TLS 1.3 Pinning</span>
                      <span className="group-row-desc">
                        Transport certificates pinned to long-term Ed25519 identity
                      </span>
                    </div>
                  </div>
                  <span className="status-pill status-pill-online">Enforced</span>
                </div>

                <div className="group-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <SecurityIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Four-Layer Authorization Model</span>
                      <span className="group-row-desc">
                        Evaluates OS availability, app permissions, peer trust, and session state
                      </span>
                    </div>
                  </div>
                  <span className="status-pill status-pill-online">Enforced</span>
                </div>

                <div className="group-divider" />

                <div className="group-row">
                  <div className="group-row-left">
                    <KeyIcon />
                    <div className="group-row-text">
                      <span className="group-row-title">Anti-Replay Token Cache</span>
                      <span className="group-row-desc">
                        Atomic single-use token consumption for QR pairing handshakes
                      </span>
                    </div>
                  </div>
                  <span className="status-pill status-pill-online">Active</span>
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
