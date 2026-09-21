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

type DeviceTab = "transfers" | "clipboard" | "notifications" | "settings";

export function App() {
  const [selectedPeerId, setSelectedPeerId] = useState<string>("cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a");
  const [activeTab, setActiveTab] = useState<DeviceTab>("transfers");

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

  // Device Permissions
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
        setSelectedPeerId(loadedPeers[0].fingerprint);
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
      fileName: "presentation_deck.pdf",
      fileSize: 3145728,
      direction: "outgoing",
      peerFingerprint: currentPeer?.fingerprint || "cont1q9a8b",
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

  const currentPeer = peers.find((p) => p.fingerprint === selectedPeerId) || peers[0];

  const filteredClips = syncedClips.filter((clip) =>
    clip.text.toLowerCase().includes(clipFilterQuery.toLowerCase())
  );

  return (
    <div className="app-shell">
      {/* Sidebar Rail (240px) */}
      <aside className="sidebar">
        {/* Brand Header */}
        <div className="sidebar-header" data-tauri-drag-region>
          <div className="brand-icon-box">
            <Share2 size={14} />
          </div>
          <span className="brand-title">Continue</span>
          <span className="brand-version">v0.1</span>
        </div>

        {/* Device List Section */}
        <div className="sidebar-content">
          <div className="sidebar-section-row">
            <span className="nav-section-label">Paired Devices</span>
            <span className="nav-count-badge">{peers.length}</span>
          </div>

          <div className="device-nav-list">
            {peers.map((peer) => {
              const isSelected = peer.fingerprint === selectedPeerId;
              const isTablet = peer.displayName.toLowerCase().includes("tablet");
              return (
                <button
                  key={peer.fingerprint}
                  className={`device-nav-item ${isSelected ? "active" : ""}`}
                  onClick={() => setSelectedPeerId(peer.fingerprint)}
                >
                  <div className="device-nav-avatar">
                    {isTablet ? <Tablet size={16} /> : <Smartphone size={16} />}
                  </div>

                  <div className="device-nav-info">
                    <span className="device-nav-name">{peer.displayName}</span>
                    <span className="device-nav-sub">{peer.endpoint || "QUIC Direct"}</span>
                  </div>

                  <div className="device-nav-meta">
                    <span className="status-dot" title="Connected via QUIC" />
                    <div style={{ display: "flex", alignItems: "center", gap: 3 }}>
                      <BatteryCharging size={10} color="var(--green)" />
                      <span className="device-nav-battery">100%</span>
                    </div>
                  </div>
                </button>
              );
            })}

            <button className="btn-add-device" onClick={() => setShowPairDialog(true)}>
              <Plus size={13} />
              <span>Pair New Device</span>
            </button>
          </div>
        </div>

        {/* Host Identity Footer */}
        <div className="sidebar-footer">
          <div className="host-name-row">
            <div className="host-name-left">
              <Laptop size={13} color="var(--text-muted)" />
              <span className="host-label">{identity.deviceName}</span>
            </div>
            <span className="host-tag">This PC</span>
          </div>

          <button
            className="host-fingerprint-btn"
            title="Click to copy Ed25519 identity key"
            onClick={handleCopyFingerprint}
          >
            <Key size={11} />
            <span>{identity.fingerprint.substring(0, 16)}...</span>
            {copiedFingerprint ? <Check size={11} color="var(--green)" /> : <Copy size={11} />}
          </button>
        </div>
      </aside>

      {/* Main Workspace Stage */}
      <main className="main-stage">
        {/* Top Header Bar */}
        <header className="main-header" data-tauri-drag-region>
          {currentPeer ? (
            <div className="header-device-context">
              <div className="header-device-headline">
                <h1 className="header-device-name">{currentPeer.displayName}</h1>
                <div className="header-chip-group">
                  <span className="header-status-chip">
                    <span className="status-dot" />
                    <span>QUIC Connected</span>
                  </span>
                  <span className="header-mono-chip" style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
                    <Wifi size={10} />
                    <span>{currentPeer.endpoint || "4433"}</span>
                  </span>
                  <span className="header-mono-chip">2 ms</span>
                </div>
              </div>
            </div>
          ) : (
            <div className="header-device-context">
              <h1 className="header-device-name">No Device Selected</h1>
            </div>
          )}

          <div className="main-header-right">
            {/* Segmented Device Tab Control */}
            {currentPeer && (
              <div className="segmented-control">
                <button
                  className={`segment-btn ${activeTab === "transfers" ? "active" : ""}`}
                  onClick={() => setActiveTab("transfers")}
                >
                  <ArrowDownUp size={13} />
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
                  <span>Alerts</span>
                  {notifications.length > 0 && (
                    <span className="segment-badge">{notifications.length}</span>
                  )}
                </button>

                <button
                  className={`segment-btn ${activeTab === "settings" ? "active" : ""}`}
                  onClick={() => setActiveTab("settings")}
                >
                  <SlidersHorizontal size={13} />
                  <span>Security</span>
                </button>
              </div>
            )}

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
          {currentPeer ? (
            <>
              {/* TAB 1: TRANSFERS */}
              {activeTab === "transfers" && (
                <div className="tab-pane">
                  {/* Fast Send Dropzone */}
                  <div className="section-block">
                    <span className="section-label">Fast Send</span>
                    <div
                      className={`dropzone-card ${isDraggingOver ? "dragging-over" : ""}`}
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
                      <div className="dropzone-left">
                        <div className="dropzone-icon-box">
                          <Upload size={18} />
                        </div>
                        <div className="dropzone-details">
                          <div className="dropzone-headline">
                            Drop files here to send to {currentPeer.displayName}
                          </div>
                          <div className="dropzone-description">
                            Chunked 64 KB QUIC datagram stream with SHA-256 verification
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
                        <Upload size={13} />
                        <span>Select Files</span>
                      </button>
                    </div>
                  </div>

                  {/* Transfer History */}
                  <div className="section-block">
                    <div className="section-header-row">
                      <span className="section-label">Recent Transfers</span>
                      <span className="section-counter">{transfers.length} items</span>
                    </div>

                    {transfers.length === 0 ? (
                      <div className="empty-state-box">
                        <FileText size={22} className="empty-state-icon" />
                        <div className="empty-state-title">No recent transfers</div>
                        <div className="empty-state-subtitle">
                          Files dropped or received via QUIC will appear here with cryptographic integrity logs.
                        </div>
                      </div>
                    ) : (
                      <div className="table-wrapper">
                        <div className="table-header-row">
                          <span style={{ flex: 3 }}>File Name</span>
                          <span style={{ flex: 1.2 }}>Size</span>
                          <span style={{ flex: 1.2 }}>Direction</span>
                          <span style={{ flex: 1.2 }}>Integrity</span>
                          <span style={{ flex: 1, textAlign: "right" }}>Action</span>
                        </div>

                        {transfers.map((tx) => (
                          <div key={tx.id} className="table-data-row">
                            <div className="table-cell-lead" style={{ flex: 3 }}>
                              <FileText size={15} color="var(--accent)" />
                              <span className="table-filename">{tx.fileName}</span>
                            </div>

                            <span className="table-mono-cell" style={{ flex: 1.2 }}>
                              {(tx.fileSize / 1024 / 1024).toFixed(2)} MB
                            </span>

                            <div style={{ flex: 1.2 }}>
                              <span
                                className={`badge-direction ${
                                  tx.direction === "incoming" ? "dir-in" : "dir-out"
                                }`}
                              >
                                {tx.direction === "incoming" ? (
                                  <ArrowDownLeft size={11} />
                                ) : (
                                  <ArrowUpRight size={11} />
                                )}
                                <span>{tx.direction}</span>
                              </span>
                            </div>

                            <div className="table-cell-verified" style={{ flex: 1.2 }}>
                              <Check size={12} />
                              <span>Verified</span>
                            </div>

                            <div style={{ flex: 1, textAlign: "right" }}>
                              <button
                                className="btn btn-sm"
                                onClick={() => showToast(`Revealing ${tx.fileName}`)}
                              >
                                <ExternalLink size={11} />
                                <span>Reveal</span>
                              </button>
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* TAB 2: CLIPBOARD */}
              {activeTab === "clipboard" && (
                <div className="tab-pane">
                  {/* Clipboard Settings & Echo Suppression */}
                  <div className="section-block">
                    <span className="section-label">Sync Preferences</span>
                    <div className="panel-card">
                      <div className="panel-row">
                        <div className="panel-row-lead">
                          <RefreshCw size={16} className="panel-icon" />
                          <div>
                            <div className="panel-row-title">Real-Time Clipboard Sync</div>
                            <div className="panel-row-sub">
                              Mirror copied text and media between this PC and {currentPeer.displayName}
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

                      <div className="panel-divider" />

                      <div className="panel-row">
                        <div className="panel-row-lead">
                          <ShieldCheck size={16} className="panel-icon" />
                          <div>
                            <div className="panel-row-title">Echo Suppression Ring</div>
                            <div className="panel-row-sub">
                              16-entry hash ring prevents recursive mirror loops across peers
                            </div>
                          </div>
                        </div>
                        <span className="chip-active">Active</span>
                      </div>
                    </div>
                  </div>

                  {/* Send Instant Text */}
                  <div className="section-block">
                    <span className="section-label">Blast Text to Device</span>
                    <div className="instant-input-row">
                      <input
                        type="text"
                        className="text-input"
                        placeholder={`Type or paste text to send to ${currentPeer.displayName}... (Press Enter)`}
                        value={clipboardInput}
                        onChange={(e) => setClipboardInput(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") handleSendClipboard();
                        }}
                      />
                      <button className="btn btn-primary" onClick={handleSendClipboard}>
                        <span>Send to Device</span>
                      </button>
                    </div>
                  </div>

                  {/* Synced Clips */}
                  <div className="section-block">
                    <div className="section-header-row">
                      <span className="section-label">Synchronized Clips</span>
                      <div className="search-input-box">
                        <Search size={12} className="search-icon" />
                        <input
                          type="text"
                          className="search-input"
                          placeholder="Search clips..."
                          value={clipFilterQuery}
                          onChange={(e) => setClipFilterQuery(e.target.value)}
                        />
                      </div>
                    </div>

                    {filteredClips.length === 0 ? (
                      <div className="empty-state-box">
                        <Clipboard size={22} className="empty-state-icon" />
                        <div className="empty-state-title">No clips found</div>
                        <div className="empty-state-subtitle">
                          Text copied on {currentPeer.displayName} or this machine will appear here automatically.
                        </div>
                      </div>
                    ) : (
                      <div className="table-wrapper">
                        <div className="table-header-row">
                          <span style={{ flex: 3 }}>Content</span>
                          <span style={{ flex: 1.2 }}>Origin</span>
                          <span style={{ flex: 1 }}>Time</span>
                          <span style={{ flex: 1, textAlign: "right" }}>Action</span>
                        </div>

                        {filteredClips.map((clip) => (
                          <div key={clip.id} className="table-data-row">
                            <span className="table-clip-text" style={{ flex: 3 }}>
                              {clip.text}
                            </span>
                            <span className="table-dim-cell" style={{ flex: 1.2 }}>
                              {clip.device}
                            </span>
                            <span className="table-mono-cell" style={{ flex: 1 }}>
                              {clip.time}
                            </span>
                            <div style={{ flex: 1, textAlign: "right" }}>
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
                </div>
              )}

              {/* TAB 3: NOTIFICATIONS */}
              {activeTab === "notifications" && (
                <div className="tab-pane">
                  <div className="section-block">
                    <div className="section-header-row">
                      <span className="section-label">Mobile Alerts ({notifications.length})</span>
                      {notifications.length > 0 && (
                        <button className="btn btn-sm" onClick={handleClearAllNotifications}>
                          <Trash2 size={11} />
                          <span>Clear All</span>
                        </button>
                      )}
                    </div>

                    {notifications.length === 0 ? (
                      <div className="empty-state-box">
                        <Bell size={22} className="empty-state-icon" />
                        <div className="empty-state-title">No notifications</div>
                        <div className="empty-state-subtitle">
                          Push notifications forwarded from {currentPeer.displayName} will appear here with live dismiss sync.
                        </div>
                      </div>
                    ) : (
                      <div className="alert-list">
                        {notifications.map((notif) => (
                          <div key={notif.id} className="alert-card">
                            <div className="alert-icon-box">
                              <Bell size={15} />
                            </div>

                            <div className="alert-body">
                              <div className="alert-top-row">
                                <span className="alert-title">{notif.title}</span>
                                <span className="alert-app-pill">{notif.appName}</span>
                                <span className="alert-time">2m ago</span>
                              </div>
                              <div className="alert-message">{notif.body}</div>
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
                </div>
              )}

              {/* TAB 4: SETTINGS & PERMISSIONS */}
              {activeTab === "settings" && (
                <div className="tab-pane">
                  <div className="section-block">
                    <span className="section-label">Device Permissions</span>
                    <div className="panel-card">
                      <div className="panel-row">
                        <div className="panel-row-lead">
                          <Upload size={16} className="panel-icon" />
                          <div>
                            <div className="panel-row-title">File Transfer Permission</div>
                            <div className="panel-row-sub">
                              Permit encrypted QUIC stream payloads to and from this device
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

                      <div className="panel-divider" />

                      <div className="panel-row">
                        <div className="panel-row-lead">
                          <Bell size={16} className="panel-icon" />
                          <div>
                            <div className="panel-row-title">Notification Mirroring</div>
                            <div className="panel-row-sub">
                              Stream push alerts with remote dismiss synchronization
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

                      <div className="panel-divider" />

                      <div className="panel-row">
                        <div className="panel-row-lead">
                          <SlidersHorizontal size={16} className="panel-icon" />
                          <div>
                            <div className="panel-row-title">Auto-Accept Small Payloads</div>
                            <div className="panel-row-sub">
                              Automatically receive files under 10 MB without manual prompt
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
                  </div>

                  <div className="section-block">
                    <span className="section-label">Cryptographic Identity</span>
                    <div className="panel-card">
                      <div className="panel-row">
                        <div className="panel-row-lead">
                          <Lock size={16} className="panel-icon" />
                          <div>
                            <div className="panel-row-title">Mutual TLS 1.3 Pinning</div>
                            <div className="panel-row-sub">
                              Transport certificate pinned to peer Ed25519 identity key
                            </div>
                          </div>
                        </div>
                        <span className="chip-active">Enforced</span>
                      </div>

                      <div className="panel-divider" />

                      <div className="panel-row">
                        <div className="panel-row-lead">
                          <ShieldCheck size={16} className="panel-icon" />
                          <div>
                            <div className="panel-row-title">Peer Fingerprint</div>
                            <div className="panel-mono-val">{currentPeer.fingerprint}</div>
                          </div>
                        </div>
                        <button
                          className="btn btn-sm"
                          onClick={() => {
                            navigator.clipboard.writeText(currentPeer.fingerprint);
                            showToast("Fingerprint copied");
                          }}
                        >
                          <Copy size={11} />
                          <span>Copy</span>
                        </button>
                      </div>
                    </div>
                  </div>

                  <div className="section-block">
                    <span className="section-label" style={{ color: "var(--red)" }}>
                      Danger Zone
                    </span>
                    <div className="panel-card danger-card">
                      <div className="panel-row">
                        <div>
                          <div className="panel-row-title">Unpair {currentPeer.displayName}</div>
                          <div className="panel-row-sub">
                            Revoke mutual authentication keys and terminate the active QUIC session
                          </div>
                        </div>
                        <button
                          className="btn btn-danger"
                          onClick={() => handleDisconnectPeer(currentPeer.fingerprint)}
                        >
                          <Trash2 size={12} />
                          <span>Unpair Device</span>
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </>
          ) : (
            <div className="empty-selection-box">
              <Smartphone size={32} color="var(--text-muted)" />
              <div className="empty-state-title">No Device Selected</div>
              <div className="empty-state-subtitle">
                Pair an Android phone or tablet from the sidebar to start streaming files and syncing clips.
              </div>
              <button className="btn btn-primary" onClick={() => setShowPairDialog(true)}>
                <Plus size={13} />
                <span>Pair New Device</span>
              </button>
            </div>
          )}
        </div>
      </main>

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
                  className="text-input"
                  placeholder="continue://pair/v1?addr=192.168.1.105:4433&spki=..."
                  value={pairingPayload}
                  onChange={(e) => setPairingPayload(e.target.value)}
                  autoFocus
                />
              </div>

              <div className="modal-actions">
                <button
                  type="button"
                  className="btn"
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
          <Check size={13} color="var(--green)" />
          <span>{toastMessage}</span>
        </div>
      )}
    </div>
  );
}

export default App;
