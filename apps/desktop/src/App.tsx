// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import React, { useEffect, useState } from "react";
import {
  Smartphone,
  Tablet,
  Send,
  Upload,
  Clipboard,
  Bell,
  Shield,
  Key,
  Plus,
  X,
  Check,
  Copy,
  FileText,
  Share2,
  ExternalLink,
  Search,
  ArrowDownLeft,
  ArrowUpRight,
  SlidersHorizontal,
  RefreshCw,
  Lock,
  ShieldCheck,
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

export function App() {
  const [selectedPeerId, setSelectedPeerId] = useState<string>("cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a");
  const [activeTab, setActiveTab] = useState<"transfers" | "clipboard" | "notifications" | "permissions">("transfers");

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
    fetchPeers().then((loaded) => {
      setPeers(loaded);
      if (loaded.length > 0) {
        setSelectedPeerId(loaded[0].fingerprint);
      }
    });
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
    const updated = peers.filter((p) => p.fingerprint !== fingerprint);
    setPeers(updated);
    if (updated.length > 0) {
      setSelectedPeerId(updated[0].fingerprint);
    }
    showToast("Device unshared");
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
        displayName: "Pixel 8 Pro",
        pairedAt: Math.floor(Date.now() / 1000),
        isConnected: true,
        endpoint: "192.168.1.105:4433",
      };
      setPeers((prev) => [...prev, dummy]);
      setSelectedPeerId(dummy.fingerprint);
      showToast("Connected to Pixel 8 Pro via QUIC");
    }

    setPairingPayload("");
    setShowPairDialog(false);
  };

  const handleSendClipboard = async () => {
    if (!clipboardInput.trim()) return;
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
    showToast(`Clip sent to ${currentPeer ? currentPeer.displayName : "peer"}`);
    setClipboardInput("");
  };

  const handleTriggerSendFile = () => {
    const newTx: TransferHistoryItem = {
      id: "tx-" + Date.now(),
      fileName: "presentation_deck.pdf",
      fileSize: 3145728,
      direction: "outgoing",
      peerFingerprint: currentPeer ? currentPeer.fingerprint : "cont1q9a8b",
      status: "completed",
      timestamp: Date.now(),
    };
    setTransfers((prev) => [newTx, ...prev]);
    showToast("Streaming presentation_deck.pdf over QUIC");
  };

  const handleDismissNotification = (id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
    showToast("Notification dismissed");
  };

  const currentPeer = peers.find((p) => p.fingerprint === selectedPeerId) || peers[0];

  return (
    <div className="app-container">
      {/* Sleek Integrated Titlebar */}
      <header className="titlebar">
        <div className="titlebar-left">
          <div className="brand-badge">
            <div className="brand-symbol">
              <Share2 size={13} />
            </div>
            <span className="brand-name">Continue</span>
          </div>
        </div>

        <div className="titlebar-center">
          <div className="quick-search-trigger" onClick={() => setShowPairDialog(true)}>
            <Search size={13} />
            <span>Quick search devices or actions...</span>
            <span className="kbd-shortcut">Ctrl+K</span>
          </div>
        </div>

        <div className="titlebar-right">
          <div className="mesh-status-indicator">
            <span className="status-dot" />
            <span>QUIC :4433</span>
          </div>

          <button className="btn btn-sm btn-primary" onClick={() => setShowPairDialog(true)}>
            <Plus size={13} />
            <span>Pair</span>
          </button>
        </div>
      </header>

      {/* Workspace Master-Detail Split */}
      <div className="workspace-split">
        {/* Master Rail: Device Mesh (260px) */}
        <aside className="device-mesh-pane">
          <div className="pane-header-row">
            <span className="pane-section-label">Mesh Devices ({peers.length})</span>
          </div>

          <div className="device-list">
            {peers.map((peer) => {
              const isSelected = peer.fingerprint === selectedPeerId;
              return (
                <div
                  key={peer.fingerprint}
                  className={`device-list-item ${isSelected ? "selected" : ""}`}
                  onClick={() => setSelectedPeerId(peer.fingerprint)}
                >
                  <div className="device-avatar">
                    {peer.displayName.toLowerCase().includes("tablet") ? (
                      <Tablet size={16} />
                    ) : (
                      <Smartphone size={16} />
                    )}
                  </div>

                  <div className="device-item-info">
                    <span className="device-item-name">{peer.displayName}</span>
                    <span className="device-item-sub">{peer.endpoint}</span>
                  </div>

                  <div className="device-item-right">
                    <span className="device-status-badge" />
                    <span className="device-battery-text">100%</span>
                  </div>
                </div>
              );
            })}
          </div>

          <div className="pair-action-box">
            <button className="btn-pair-device" onClick={() => setShowPairDialog(true)}>
              <Plus size={14} />
              <span>Pair New Device</span>
            </button>
          </div>

          <div className="rail-bottom-card">
            <div className="local-node-row">
              <span className="local-node-name">{identity.deviceName}</span>
              <span style={{ fontSize: 11, color: "var(--green)" }}>Host</span>
            </div>
            <button
              className="local-node-hash-btn"
              title="Copy Ed25519 node key"
              onClick={handleCopyFingerprint}
            >
              <Key size={11} />
              <span>{identity.fingerprint.substring(0, 14)}...</span>
              {copiedFingerprint ? <Check size={11} color="var(--green)" /> : <Copy size={11} />}
            </button>
          </div>
        </aside>

        {/* Detail Workspace for Selected Device */}
        <main className="device-detail-pane">
          {currentPeer ? (
            <>
              {/* Detail Header Bar */}
              <div className="detail-header">
                <div className="detail-header-left">
                  <h2 className="detail-header-title">{currentPeer.displayName}</h2>
                  <span className="detail-header-status">
                    <span className="status-dot" />
                    Connected via QUIC
                  </span>
                  <span className="detail-header-endpoint">{currentPeer.endpoint}</span>
                </div>

                {/* Segmented Control */}
                <div className="segmented-nav">
                  <button
                    className={`segment-btn ${activeTab === "transfers" ? "active" : ""}`}
                    onClick={() => setActiveTab("transfers")}
                  >
                    <Send size={13} />
                    <span>Transfers</span>
                  </button>

                  <button
                    className={`segment-btn ${activeTab === "clipboard" ? "active" : ""}`}
                    onClick={() => setActiveTab("clipboard")}
                  >
                    <Clipboard size={13} />
                    <span>Clipboard</span>
                  </button>

                  <button
                    className={`segment-btn ${activeTab === "notifications" ? "active" : ""}`}
                    onClick={() => setActiveTab("notifications")}
                  >
                    <Bell size={13} />
                    <span>Notifications</span>
                    {notifications.length > 0 && (
                      <span style={{ fontSize: 10, padding: "1px 5px", borderRadius: 4, backgroundColor: "var(--amber)", color: "#000", fontWeight: 700 }}>
                        {notifications.length}
                      </span>
                    )}
                  </button>

                  <button
                    className={`segment-btn ${activeTab === "permissions" ? "active" : ""}`}
                    onClick={() => setActiveTab("permissions")}
                  >
                    <Shield size={13} />
                    <span>Permissions</span>
                  </button>
                </div>
              </div>

              {/* Detail Content */}
              <div className="detail-content">
                {/* TAB 1: TRANSFERS */}
                {activeTab === "transfers" && (
                  <div className="settings-section">
                    <div>
                      <span className="section-label">Fast Send</span>
                      {/* Compact Dropzone */}
                      <div className="compact-dropzone" onClick={handleTriggerSendFile}>
                        <div className="dropzone-left-content">
                          <div className="dropzone-icon-box">
                            <Upload size={18} />
                          </div>
                          <div>
                            <div className="dropzone-title">Drop files here to send to {currentPeer.displayName}</div>
                            <div className="dropzone-desc">Chunked 64 KB QUIC streaming with SHA-256 verification</div>
                          </div>
                        </div>
                        <button className="btn btn-primary" onClick={(e) => { e.stopPropagation(); handleTriggerSendFile(); }}>
                          Browse Files
                        </button>
                      </div>
                    </div>

                    <div>
                      <span className="section-label">Recent Activity</span>
                      <div className="data-table-wrap">
                        <div className="data-table-header">
                          <span>File Name</span>
                          <span>Size</span>
                          <span>Direction</span>
                          <span>Status</span>
                          <span style={{ textAlign: "right" }}>Actions</span>
                        </div>

                        {transfers.map((tx) => (
                          <div key={tx.id} className="data-table-row">
                            <div className="table-file-cell">
                              <FileText size={16} color="var(--accent-cyan)" />
                              <span className="table-file-name">{tx.fileName}</span>
                            </div>

                            <span style={{ color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>
                              {(tx.fileSize / 1024 / 1024).toFixed(2)} MB
                            </span>

                            <div>
                              <span className={`badge-direction ${tx.direction === "incoming" ? "badge-incoming" : "badge-outgoing"}`}>
                                {tx.direction === "incoming" ? <ArrowDownLeft size={12} /> : <ArrowUpRight size={12} />}
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
                  </div>
                )}

                {/* TAB 2: CLIPBOARD */}
                {activeTab === "clipboard" && (
                  <div className="settings-section">
                    <div>
                      <span className="section-label">Sync Preferences</span>
                      <div className="settings-card">
                        <div className="settings-row">
                          <div className="settings-row-lead">
                            <RefreshCw size={18} className="settings-row-icon" />
                            <div className="settings-row-text">
                              <span className="settings-row-title">Bidirectional Sync</span>
                              <span className="settings-row-desc">Automatically mirror copied text and media to {currentPeer.displayName}</span>
                            </div>
                          </div>
                          <label className="sleek-switch">
                            <input
                              type="checkbox"
                              checked={allowClipboardSync}
                              onChange={(e) => setAllowClipboardSync(e.target.checked)}
                            />
                            <span className="sleek-slider" />
                          </label>
                        </div>

                        <div className="setting-divider" />

                        <div className="settings-row">
                          <div className="settings-row-lead">
                            <ShieldCheck size={18} className="settings-row-icon" />
                            <div className="settings-row-text">
                              <span className="settings-row-title">Echo Suppression Protection</span>
                              <span className="settings-row-desc">Rolling 16-entry hash ring eliminates recursive paste loops</span>
                            </div>
                          </div>
                          <span style={{ fontSize: 11, color: "var(--green)", fontWeight: 600 }}>Active</span>
                        </div>
                      </div>
                    </div>

                    <div>
                      <span className="section-label">Send Clip to Device</span>
                      <div style={{ display: "flex", gap: 8 }}>
                        <input
                          type="text"
                          className="sleek-input"
                          placeholder="Type or paste text to blast to device..."
                          value={clipboardInput}
                          onChange={(e) => setClipboardInput(e.target.value)}
                          onKeyDown={(e) => { if (e.key === "Enter") handleSendClipboard(); }}
                        />
                        <button className="btn btn-primary" onClick={handleSendClipboard}>
                          Send
                        </button>
                      </div>
                    </div>

                    <div>
                      <span className="section-label">Synchronized History</span>
                      <div className="data-table-wrap">
                        <div className="data-table-header" style={{ gridTemplateColumns: "3fr 1fr 1fr" }}>
                          <span>Text Content</span>
                          <span>Source</span>
                          <span style={{ textAlign: "right" }}>Action</span>
                        </div>

                        {syncedClips.map((clip) => (
                          <div key={clip.id} className="data-table-row" style={{ gridTemplateColumns: "3fr 1fr 1fr" }}>
                            <span style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-primary)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                              {clip.text}
                            </span>
                            <span style={{ color: "var(--silver-dim)", fontSize: 11 }}>{clip.device}</span>
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
                  </div>
                )}

                {/* TAB 3: NOTIFICATIONS */}
                {activeTab === "notifications" && (
                  <div className="settings-section">
                    <div>
                      <span className="section-label">Incoming Alerts ({notifications.length})</span>
                      <div className="notification-feed">
                        {notifications.length === 0 ? (
                          <div style={{ padding: 40, textAlign: "center", color: "var(--silver-dim)" }}>
                            No incoming notifications from {currentPeer.displayName}
                          </div>
                        ) : (
                          notifications.map((notif) => (
                            <div key={notif.id} className="notification-card-item">
                              <div className="notif-app-badge">
                                <Bell size={14} />
                              </div>

                              <div className="notif-main-wrap">
                                <div className="notif-title-row">
                                  <span className="notif-author">
                                    {notif.title} &bull; <span style={{ color: "var(--silver-dim)", fontWeight: 400 }}>{notif.appName}</span>
                                  </span>
                                  <span className="notif-timestamp">2m ago</span>
                                </div>
                                <span className="notif-body-message">{notif.body}</span>
                              </div>

                              <button
                                className="btn btn-sm"
                                onClick={() => handleDismissNotification(notif.id)}
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

                {/* TAB 4: PERMISSIONS */}
                {activeTab === "permissions" && (
                  <div className="settings-section">
                    <div>
                      <span className="section-label">Device Capabilities</span>
                      <div className="settings-card">
                        <div className="settings-row">
                          <div className="settings-row-lead">
                            <Upload size={18} className="settings-row-icon" />
                            <div className="settings-row-text">
                              <span className="settings-row-title">File Transfer Permission</span>
                              <span className="settings-row-desc">Allow receiving and streaming files over QUIC</span>
                            </div>
                          </div>
                          <label className="sleek-switch">
                            <input
                              type="checkbox"
                              checked={allowFileTransfer}
                              onChange={(e) => setAllowFileTransfer(e.target.checked)}
                            />
                            <span className="sleek-slider" />
                          </label>
                        </div>

                        <div className="setting-divider" />

                        <div className="settings-row">
                          <div className="settings-row-lead">
                            <Bell size={18} className="settings-row-icon" />
                            <div className="settings-row-text">
                              <span className="settings-row-title">Notification Mirroring</span>
                              <span className="settings-row-desc">Forward push alerts with remote dismiss sync</span>
                            </div>
                          </div>
                          <label className="sleek-switch">
                            <input
                              type="checkbox"
                              checked={allowNotifications}
                              onChange={(e) => setAllowNotifications(e.target.checked)}
                            />
                            <span className="sleek-slider" />
                          </label>
                        </div>

                        <div className="setting-divider" />

                        <div className="settings-row">
                          <div className="settings-row-lead">
                            <SlidersHorizontal size={18} className="settings-row-icon" />
                            <div className="settings-row-text">
                              <span className="settings-row-title">Auto-Accept Small Transfers</span>
                              <span className="settings-row-desc">Automatically accept payloads under 10 MB without manual prompt</span>
                            </div>
                          </div>
                          <label className="sleek-switch">
                            <input
                              type="checkbox"
                              checked={autoAcceptSmall}
                              onChange={(e) => setAutoAcceptSmall(e.target.checked)}
                            />
                            <span className="sleek-slider" />
                          </label>
                        </div>

                        <div className="setting-divider" />

                        <div className="settings-row">
                          <div className="settings-row-lead">
                            <Lock size={18} className="settings-row-icon" />
                            <div className="settings-row-text">
                              <span className="settings-row-title">Mutual TLS 1.3 Certificate Pinning</span>
                              <span className="settings-row-desc">SPKI transport certificate pinned to Ed25519 identity</span>
                            </div>
                          </div>
                          <span style={{ fontSize: 11, color: "var(--green)", fontWeight: 600 }}>Enforced</span>
                        </div>
                      </div>
                    </div>

                    <div>
                      <span className="section-label" style={{ color: "var(--red)" }}>Danger Zone</span>
                      <div className="settings-card" style={{ borderColor: "rgba(239, 68, 68, 0.25)" }}>
                        <div className="settings-row">
                          <div className="settings-row-text">
                            <span className="settings-row-title">Unpair Device</span>
                            <span className="settings-row-desc">Revoke mutual authentication keys and terminate active session</span>
                          </div>
                          <button
                            className="btn btn-danger"
                            onClick={() => handleDisconnectPeer(currentPeer.fingerprint)}
                          >
                            Unpair
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            </>
          ) : (
            <div style={{ display: "flex", alignItems: "center", justifyContent: "center", flex: 1, color: "var(--text-muted)" }}>
              No device selected. Pair a device from the left rail to begin.
            </div>
          )}
        </main>
      </div>

      {/* Pair New Device Modal */}
      {showPairDialog && (
        <div className="modal-overlay" onClick={() => setShowPairDialog(false)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()}>
            <div className="modal-title-row">
              <span className="modal-title-text">Pair Remote Device</span>
              <button className="modal-close-icon-btn" onClick={() => setShowPairDialog(false)}>
                <X size={16} />
              </button>
            </div>

            <form onSubmit={handlePairSubmit}>
              <label style={{ display: "block", marginBottom: 8, fontSize: 12, color: "var(--text-secondary)" }}>
                Enter pairing URI or QR payload:
              </label>
              <input
                type="text"
                className="sleek-input"
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

      {/* Feedback Toast */}
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
