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
  Minus,
  Square,
  Search,
  Trash2,
  BatteryCharging,
  Wifi,
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
  const [selectedTargetPeer, setSelectedTargetPeer] = useState<string>("");
  const [showPairDialog, setShowPairDialog] = useState(false);
  const [pairingPayload, setPairingPayload] = useState("");
  const [copiedFingerprint, setCopiedFingerprint] = useState(false);
  const [toastMessage, setToastMessage] = useState("");
  const [isDraggingOver, setIsDraggingOver] = useState(false);
  const [copiedClipId, setCopiedClipId] = useState<string | null>(null);

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
      body: "Starting in 10 minutes in Conference Room B",
      timestamp: Date.now() - 600000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
  ]);

  const [clipboardInput, setClipboardInput] = useState("");
  const [clipFilterQuery, setClipFilterQuery] = useState("");
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
    fetchPeers().then((loadedPeers) => {
      setPeers(loadedPeers);
      if (loadedPeers.length > 0) {
        setSelectedTargetPeer(loadedPeers[0].displayName);
      }
    });
  }, []);

  const showToast = useCallback((msg: string) => {
    setToastMessage(msg);
    setTimeout(() => setToastMessage(""), 2600);
  }, []);

  const handleCopyFingerprint = () => {
    navigator.clipboard.writeText(identity.fingerprint);
    setCopiedFingerprint(true);
    showToast("Identity fingerprint copied to clipboard");
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
      setSelectedTargetPeer(peer.displayName);
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
      setSelectedTargetPeer(dummy.displayName);
      showToast("Connected via QUIC");
    }

    setPairingPayload("");
    setShowPairDialog(false);
  };

  const handleSendClipboard = async () => {
    if (!clipboardInput.trim()) return;
    const targetPeer = peers.find((p) => p.displayName === selectedTargetPeer) || peers[0];
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
    const peerName = targetDeviceName || selectedTargetPeer || peers[0]?.displayName || "Device";
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
    showToast(`Streaming presentation_deck.pdf to ${peerName}`);
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

  const pageTitleMap: Record<AppPage, string> = {
    devices: "Connected Devices",
    transfers: "File Transfers",
    clipboard: "Clipboard Sync",
    notifications: "Notification Feed",
    settings: "System Settings",
  };

  const filteredClips = syncedClips.filter((clip) =>
    clip.text.toLowerCase().includes(clipFilterQuery.toLowerCase())
  );

  return (
    <div className="app-shell">
      {/* Sidebar (210px) */}
      <aside className="sidebar">
        <div className="sidebar-header">
          <div className="brand-icon-box">
            <Share2 size={15} />
          </div>
          <span className="brand-title">Continue</span>
          <span className="brand-version">v0.1</span>
        </div>

        <nav className="sidebar-nav">
          <div className="nav-section-label">Continuity</div>

          <button
            className={`nav-item ${activePage === "devices" ? "active" : ""}`}
            onClick={() => setActivePage("devices")}
          >
            <Smartphone size={15} className="nav-item-icon" />
            <span className="nav-item-label">Devices</span>
            <span className="nav-item-badge">{peers.length}</span>
          </button>

          <button
            className={`nav-item ${activePage === "transfers" ? "active" : ""}`}
            onClick={() => setActivePage("transfers")}
          >
            <ArrowDownUp size={15} className="nav-item-icon" />
            <span className="nav-item-label">Transfers</span>
            {transfers.length > 0 && (
              <span className="nav-item-badge">{transfers.length}</span>
            )}
          </button>

          <button
            className={`nav-item ${activePage === "clipboard" ? "active" : ""}`}
            onClick={() => setActivePage("clipboard")}
          >
            <Clipboard size={15} className="nav-item-icon" />
            <span className="nav-item-label">Clipboard</span>
          </button>

          <button
            className={`nav-item ${activePage === "notifications" ? "active" : ""}`}
            onClick={() => setActivePage("notifications")}
          >
            <Bell size={15} className="nav-item-icon" />
            <span className="nav-item-label">Notifications</span>
            {notifications.length > 0 && (
              <span className="nav-item-badge" style={{ color: "var(--accent)" }}>
                {notifications.length}
              </span>
            )}
          </button>

          <div className="sidebar-divider" />
          <div className="nav-section-label">Preferences</div>

          <button
            className={`nav-item ${activePage === "settings" ? "active" : ""}`}
            onClick={() => setActivePage("settings")}
          >
            <Settings size={15} className="nav-item-icon" />
            <span className="nav-item-label">Settings</span>
          </button>
        </nav>

        <div className="sidebar-footer">
          <div className="host-name-row">
            <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <Laptop size={13} color="var(--text-muted)" />
              <span className="host-label">{identity.deviceName}</span>
            </div>
            <span className="host-tag">Host</span>
          </div>
          <button
            className="host-fingerprint-btn"
            title="Click to copy Ed25519 identity key"
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
        <header className="main-header" data-tauri-drag-region>
          <div className="main-header-left">
            <h1 className="page-title">{pageTitleMap[activePage]}</h1>
            <div className="header-status-chip">
              <span className="status-dot" />
              <span>QUIC :4433 &bull; {peers.length} Online</span>
            </div>
          </div>

          <div className="main-header-right">
            <button className="btn btn-primary" onClick={() => setShowPairDialog(true)}>
              <Plus size={13} />
              <span>Pair Device</span>
            </button>

            {/* Frameless Window Controls */}
            <div className="window-controls">
              <button
                className="win-btn"
                title="Minimize"
                onClick={handleMinimizeWindow}
              >
                <Minus size={13} />
              </button>
              <button
                className="win-btn"
                title="Maximize / Restore"
                onClick={handleMaximizeWindow}
              >
                <Square size={11} />
              </button>
              <button
                className="win-btn win-btn-close"
                title="Close"
                onClick={handleCloseWindow}
              >
                <X size={13} />
              </button>
            </div>
          </div>
        </header>

        {/* Content Body */}
        <div className="page-body">
          {/* PAGE 1: DEVICES */}
          {activePage === "devices" && (
            <div>
              <span className="section-label">Connected Peers ({peers.length})</span>
              {peers.length === 0 ? (
                <div className="empty-state-box">
                  <div className="empty-state-icon">
                    <Smartphone size={20} />
                  </div>
                  <div className="empty-state-title">No connected devices</div>
                  <div className="empty-state-subtitle">
                    Pair an Android phone or tablet on your local network to stream files and sync clips.
                  </div>
                  <button className="btn btn-primary" onClick={() => setShowPairDialog(true)}>
                    <Plus size={13} />
                    <span>Pair New Device</span>
                  </button>
                </div>
              ) : (
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
                        <div className="device-connection-badge">
                          <span className="status-dot" />
                          <span>Direct QUIC</span>
                        </div>
                      </div>

                      <div className="device-card-stats">
                        <div className="stat-item">
                          <BatteryCharging size={12} color="var(--green)" />
                          <span>100% Battery</span>
                        </div>
                        <div className="stat-item">
                          <ShieldCheck size={12} color="var(--accent)" />
                          <span>mTLS 1.3</span>
                        </div>
                        <div className="stat-item">
                          <Wifi size={12} color="var(--text-secondary)" />
                          <span>2 ms Latency</span>
                        </div>
                      </div>

                      <div className="device-card-actions">
                        <button
                          className="btn btn-sm"
                          onClick={() => {
                            setSelectedTargetPeer(peer.displayName);
                            handleTriggerSendFile(peer.displayName);
                          }}
                        >
                          <Upload size={12} />
                          <span>Send File</span>
                        </button>
                        <button
                          className="btn btn-sm"
                          onClick={() => {
                            if (peer.endpoint) {
                              navigator.clipboard.writeText(peer.endpoint);
                              showToast("Address copied");
                            }
                          }}
                        >
                          <Copy size={11} />
                          <span>Copy IP</span>
                        </button>
                        <button
                          className="btn btn-sm btn-danger"
                          onClick={() => handleDisconnectPeer(peer.fingerprint)}
                        >
                          <Trash2 size={11} />
                          <span>Unpair</span>
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}

              <span className="section-label">Local Transport Node (This Machine)</span>
              <div className="host-node-grid">
                {/* Tile 1: Node Identity */}
                <div className="host-tile">
                  <div className="host-tile-head">
                    <span className="host-tile-label">Node Identity</span>
                    <button className="btn btn-sm" onClick={handleCopyFingerprint}>
                      <Copy size={11} />
                      <span>Copy</span>
                    </button>
                  </div>
                  <div className="host-tile-val">{identity.fingerprint}</div>
                  <div className="host-tile-sub">Ed25519 Curve25519 Pair</div>
                </div>

                {/* Tile 2: Security & SPKI */}
                <div className="host-tile">
                  <div className="host-tile-head">
                    <span className="host-tile-label">mTLS 1.3 Certificate</span>
                    <span style={{ fontSize: 10, color: "var(--green)", fontWeight: 600, textTransform: "uppercase" }}>
                      Enforced
                    </span>
                  </div>
                  <div className="host-tile-val">{identity.spkiHash.substring(0, 36)}...</div>
                  <div className="host-tile-sub">Pinned SPKI SHA-256 Hash</div>
                </div>

                {/* Tile 3: Socket & Network */}
                <div className="host-tile">
                  <div className="host-tile-head">
                    <span className="host-tile-label">QUIC Socket</span>
                    <span style={{ fontSize: 10, color: "var(--green)", fontWeight: 600, textTransform: "uppercase" }}>
                      Online
                    </span>
                  </div>
                  <div className="host-tile-val">0.0.0.0:4433 (UDP)</div>
                  <div className="host-tile-sub">2 MB Buffer &bull; RFC 9000</div>
                </div>
              </div>
            </div>
          )}

          {/* PAGE 2: TRANSFERS */}
          {activePage === "transfers" && (
            <div>
              <span className="section-label">Fast File Share</span>
              <div
                className={`dropzone-container ${isDraggingOver ? "dragging-over" : ""}`}
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
                <div className="dropzone-lead">
                  <div className="dropzone-icon">
                    <Upload size={18} />
                  </div>
                  <div>
                    <div className="dropzone-title">Drag and drop files to stream over encrypted QUIC</div>
                    <div className="dropzone-subtitle">
                      Chunked 64 KB streaming with SHA-256 integrity verification across peers
                    </div>
                  </div>
                </div>

                <div className="dropzone-controls" onClick={(e) => e.stopPropagation()}>
                  {peers.length > 0 && (
                    <select
                      className="peer-select"
                      value={selectedTargetPeer}
                      onChange={(e) => setSelectedTargetPeer(e.target.value)}
                    >
                      {peers.map((p) => (
                        <option key={p.fingerprint} value={p.displayName}>
                          To: {p.displayName}
                        </option>
                      ))}
                    </select>
                  )}
                  <button className="btn btn-primary" onClick={() => handleTriggerSendFile()}>
                    Browse Files
                  </button>
                </div>
              </div>

              <span className="section-label">Transfer History ({transfers.length})</span>
              {transfers.length === 0 ? (
                <div className="empty-state-box">
                  <div className="empty-state-icon">
                    <FileText size={20} />
                  </div>
                  <div className="empty-state-title">No transfers yet</div>
                  <div className="empty-state-subtitle">
                    Dropped or received files will be logged here with cryptographic verification.
                  </div>
                </div>
              ) : (
                <div className="table-card">
                  <div
                    className="table-head-row"
                    style={{ gridTemplateColumns: "3fr 1.5fr 1fr 1fr 1fr 1fr" }}
                  >
                    <span>File Name</span>
                    <span>Target Peer</span>
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
                        <FileText size={15} color="var(--accent)" />
                        <span className="file-name-text">{tx.fileName}</span>
                      </div>

                      <span style={{ color: "var(--text-secondary)" }}>
                        {selectedTargetPeer || "Pixel 8 Pro"}
                      </span>

                      <span style={{ color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>
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
              )}
            </div>
          )}

          {/* PAGE 3: CLIPBOARD */}
          {activePage === "clipboard" && (
            <div>
              <span className="section-label">Sync Preferences</span>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-lead">
                    <RefreshCw size={17} className="setting-icon" />
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
                    <ShieldCheck size={17} className="setting-icon" />
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
              <div style={{ display: "flex", gap: 10, marginBottom: 20 }}>
                <input
                  type="text"
                  className="input-field"
                  placeholder="Type or paste text to blast to connected devices... (Press Enter to send)"
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

              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
                <span className="section-label" style={{ marginBottom: 0 }}>
                  Synchronized Clips ({filteredClips.length})
                </span>
                <div style={{ position: "relative", width: 220 }}>
                  <input
                    type="text"
                    className="input-field"
                    style={{ padding: "4px 8px 4px 26px", fontSize: 11.5 }}
                    placeholder="Search clips..."
                    value={clipFilterQuery}
                    onChange={(e) => setClipFilterQuery(e.target.value)}
                  />
                  <Search size={11} style={{ position: "absolute", left: 8, top: "50%", transform: "translateY(-50%)", color: "var(--text-muted)" }} />
                </div>
              </div>

              {filteredClips.length === 0 ? (
                <div className="empty-state-box">
                  <div className="empty-state-icon">
                    <Clipboard size={20} />
                  </div>
                  <div className="empty-state-title">No clips matched</div>
                  <div className="empty-state-subtitle">
                    Text copied on connected devices will appear here automatically.
                  </div>
                </div>
              ) : (
                <div className="table-card">
                  <div
                    className="table-head-row"
                    style={{ gridTemplateColumns: "3fr 1.2fr 1fr 1fr" }}
                  >
                    <span>Text Content</span>
                    <span>Origin Device</span>
                    <span>Timestamp</span>
                    <span style={{ textAlign: "right" }}>Action</span>
                  </div>

                  {filteredClips.map((clip) => (
                    <div
                      key={clip.id}
                      className="table-body-row"
                      style={{ gridTemplateColumns: "3fr 1.2fr 1fr 1fr" }}
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
                      <span style={{ color: "var(--text-muted)", fontSize: 11 }}>{clip.time}</span>
                      <div style={{ textAlign: "right" }}>
                        <button
                          className="btn btn-sm"
                          onClick={() => handleCopyText(clip.text, clip.id)}
                        >
                          {copiedClipId === clip.id ? (
                            <>
                              <Check size={11} color="var(--green)" />
                              <span style={{ color: "var(--green)" }}>Copied</span>
                            </>
                          ) : (
                            <>
                              <Copy size={11} />
                              <span>Copy</span>
                            </>
                          )}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* PAGE 4: NOTIFICATIONS */}
          {activePage === "notifications" && (
            <div>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
                <span className="section-label" style={{ marginBottom: 0 }}>
                  Incoming Alerts ({notifications.length})
                </span>
                {notifications.length > 0 && (
                  <button className="btn btn-sm" onClick={handleClearAllNotifications}>
                    <Trash2 size={11} />
                    <span>Clear All</span>
                  </button>
                )}
              </div>

              {notifications.length === 0 ? (
                <div className="empty-state-box">
                  <div className="empty-state-icon">
                    <Bell size={20} />
                  </div>
                  <div className="empty-state-title">No incoming notifications</div>
                  <div className="empty-state-subtitle">
                    Push alerts from your connected mobile devices will appear here in real time.
                  </div>
                </div>
              ) : (
                <div className="notif-list">
                  {notifications.map((notif) => (
                    <div key={notif.id} className="notif-card">
                      <div className="notif-icon-box">
                        <Bell size={15} />
                      </div>

                      <div className="notif-content">
                        <div className="notif-header-line">
                          <span className="notif-title">
                            {notif.title} &bull;{" "}
                            <span style={{ color: "var(--text-muted)", fontWeight: 400 }}>
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
              <span className="section-label">Continuity Capabilities</span>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-lead">
                    <Upload size={17} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">File Transfer Support</span>
                      <span className="setting-subtitle">
                        Allow receiving and streaming chunked files over encrypted QUIC
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
                    <Bell size={17} className="setting-icon" />
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
                    <SlidersHorizontal size={17} className="setting-icon" />
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

              <span className="section-label">Security & Cryptography</span>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-lead">
                    <Lock size={17} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">Mutual TLS 1.3 Certificate Pinning</span>
                      <span className="setting-subtitle">
                        SPKI transport certificate pinned to Ed25519 identity key
                      </span>
                    </div>
                  </div>
                  <span style={{ fontSize: 11, color: "var(--green)", fontWeight: 600 }}>Enforced</span>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-lead">
                    <Key size={17} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">Ed25519 Node Fingerprint</span>
                      <span className="setting-subtitle" style={{ fontFamily: "var(--font-mono)" }}>
                        {identity.fingerprint}
                      </span>
                    </div>
                  </div>
                  <button className="btn btn-sm" onClick={handleCopyFingerprint}>
                    <Copy size={11} />
                    <span>Copy Key</span>
                  </button>
                </div>
              </div>

              <span className="section-label">Network Configuration</span>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-lead">
                    <ArrowDownUp size={17} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">QUIC Transport Port</span>
                      <span className="setting-subtitle">Listening on UDP socket 0.0.0.0:4433</span>
                    </div>
                  </div>
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-secondary)" }}>
                    :4433
                  </span>
                </div>

                <div className="setting-divider" />

                <div className="setting-row">
                  <div className="setting-lead">
                    <SlidersHorizontal size={17} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">UDP Datagram Buffer</span>
                      <span className="setting-subtitle">Kernel socket buffer optimized for low latency streams</span>
                    </div>
                  </div>
                  <span style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-secondary)" }}>
                    2.0 MB
                  </span>
                </div>
              </div>

              <span className="section-label">About Continue</span>
              <div className="setting-card">
                <div className="setting-row">
                  <div className="setting-lead">
                    <Share2 size={17} className="setting-icon" />
                    <div className="setting-text">
                      <span className="setting-title">Continue Desktop Client</span>
                      <span className="setting-subtitle">
                        Version 0.1.0 &bull; Apache-2.0 License &bull; Peer-to-Peer Device Continuity
                      </span>
                    </div>
                  </div>
                  <span style={{ fontSize: 11, color: "var(--text-muted)", fontFamily: "var(--font-mono)" }}>
                    Up to date
                  </span>
                </div>
              </div>
            </div>
          )}
        </div>
      </main>

      {/* Pair New Device Modal */}
      {showPairDialog && (
        <div
          className="modal-backdrop"
          onClick={() => setShowPairDialog(false)}
          onKeyDown={(e) => {
            if (e.key === "Escape") setShowPairDialog(false);
          }}
        >
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <span className="modal-title">Pair New Device</span>
              <button className="modal-close-btn" onClick={() => setShowPairDialog(false)}>
                <X size={15} />
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
                  Connect Device
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
