// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { useState } from "react";
import "./App.css";
import type { DeviceIdentity, TrustedPeer, TransferHistoryItem, NotificationItem } from "./types.ts";

export function App() {
  const [activeTab, setActiveTab] = useState<"devices" | "transfers" | "clipboard" | "notifications" | "permissions">("devices");

  const [identity] = useState<DeviceIdentity>({
    deviceName: "Desktop PC",
    fingerprint: "cont1q8f7e2a9d4c6b8a1e3f5a7b9c1d3e5f7a9b1c3d",
    spkiHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  });

  const [peers, setPeers] = useState<TrustedPeer[]>([
    {
      fingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      displayName: "Pixel 8 Pro",
      pairedAt: 1726920000,
      isConnected: true,
      endpoint: "192.168.1.105:4433",
    },
  ]);

  const [transfers] = useState<TransferHistoryItem[]>([
    {
      id: "tx-1",
      fileName: "annual_report.pdf",
      fileSize: 2048576,
      direction: "outgoing",
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
      status: "completed",
      timestamp: Date.now() - 360000,
    },
  ]);

  const [notifications] = useState<NotificationItem[]>([
    {
      id: "notif-1",
      appName: "Messages",
      title: "Alice",
      body: "Sent you the project update document.",
      timestamp: Date.now() - 120000,
      peerFingerprint: "cont1q9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a",
    },
  ]);

  const [clipboardSyncEnabled, setClipboardSyncEnabled] = useState(true);
  const [notificationSyncEnabled, setNotificationSyncEnabled] = useState(true);
  const [qrInput, setQrInput] = useState("");
  const [showPairModal, setShowPairModal] = useState(false);

  const handleRemovePeer = (fingerprint: string) => {
    setPeers(peers.filter((p) => p.fingerprint !== fingerprint));
  };

  const handlePairFromQr = (e: React.FormEvent) => {
    e.preventDefault();
    if (!qrInput.trim()) return;

    const newPeer: TrustedPeer = {
      fingerprint: "cont1q" + Math.random().toString(36).substring(2, 15),
      displayName: "New Device",
      pairedAt: Math.floor(Date.now() / 1000),
      isConnected: true,
      endpoint: "192.168.1.120:4433",
    };

    setPeers([...peers, newPeer]);
    setQrInput("");
    setShowPairModal(false);
  };

  return (
    <div className="app-container">
      <aside className="sidebar">
        <div className="sidebar-header">
          <div className="sidebar-title">Continue</div>
          <div className="sidebar-subtitle">Device Continuity</div>
        </div>
        <ul className="nav-list">
          <li>
            <button
              className={`nav-item ${activeTab === "devices" ? "active" : ""}`}
              onClick={() => setActiveTab("devices")}
            >
              Devices
            </button>
          </li>
          <li>
            <button
              className={`nav-item ${activeTab === "transfers" ? "active" : ""}`}
              onClick={() => setActiveTab("transfers")}
            >
              File Transfer
            </button>
          </li>
          <li>
            <button
              className={`nav-item ${activeTab === "clipboard" ? "active" : ""}`}
              onClick={() => setActiveTab("clipboard")}
            >
              Clipboard
            </button>
          </li>
          <li>
            <button
              className={`nav-item ${activeTab === "notifications" ? "active" : ""}`}
              onClick={() => setActiveTab("notifications")}
            >
              Notifications
            </button>
          </li>
          <li>
            <button
              className={`nav-item ${activeTab === "permissions" ? "active" : ""}`}
              onClick={() => setActiveTab("permissions")}
            >
              Permissions
            </button>
          </li>
        </ul>
      </aside>

      <main className="main-content">
        <header className="header">
          <h1 className="header-title">
            {activeTab === "devices" && "Connected Devices"}
            {activeTab === "transfers" && "File Transfers"}
            {activeTab === "clipboard" && "Clipboard Synchronization"}
            {activeTab === "notifications" && "Notification Mirroring"}
            {activeTab === "permissions" && "Security & Permissions"}
          </h1>
          {activeTab === "devices" && (
            <button className="btn btn-primary" onClick={() => setShowPairModal(true)}>
              Pair New Device
            </button>
          )}
        </header>

        <div className="content-pane">
          {activeTab === "devices" && (
            <div>
              <div className="card">
                <div className="card-title">This Device</div>
                <p style={{ color: "var(--text-secondary)", marginBottom: 8 }}>
                  Name: <strong>{identity.deviceName}</strong>
                </p>
                <p style={{ color: "var(--text-secondary)", marginBottom: 8 }}>
                  Fingerprint: <span className="mono-tag">{identity.fingerprint}</span>
                </p>
                <p style={{ color: "var(--text-secondary)" }}>
                  Transport SPKI: <span className="mono-tag">{identity.spkiHash.substring(0, 16)}...</span>
                </p>
              </div>

              {showPairModal && (
                <div className="card" style={{ border: "2px solid var(--accent)" }}>
                  <div className="card-title">Pair a Remote Device</div>
                  <p style={{ color: "var(--text-secondary)", marginBottom: 12 }}>
                    Paste the QR code payload string generated by the remote device to establish an authenticated QUIC session.
                  </p>
                  <form onSubmit={handlePairFromQr}>
                    <input
                      type="text"
                      className="input-field"
                      placeholder="continue://pair/v1?token=..."
                      value={qrInput}
                      onChange={(e) => setQrInput(e.target.value)}
                    />
                    <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
                      <button type="button" className="btn" onClick={() => setShowPairModal(false)}>
                        Cancel
                      </button>
                      <button type="submit" className="btn btn-primary">
                        Confirm Pairing
                      </button>
                    </div>
                  </form>
                </div>
              )}

              <div className="card">
                <div className="card-title">Trusted Devices ({peers.length})</div>
                {peers.length === 0 ? (
                  <p style={{ color: "var(--text-secondary)" }}>No devices paired yet.</p>
                ) : (
                  peers.map((peer) => (
                    <div key={peer.fingerprint} className="peer-row">
                      <div>
                        <div style={{ fontWeight: 500 }}>{peer.displayName}</div>
                        <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                          <span className="mono-tag">{peer.fingerprint.substring(0, 16)}...</span>
                        </div>
                      </div>
                      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                        <span className="status-badge">
                          <span className={`status-dot ${peer.isConnected ? "online" : "offline"}`} />
                          {peer.isConnected ? "Online" : "Offline"}
                        </span>
                        <button
                          className="btn btn-danger"
                          onClick={() => handleRemovePeer(peer.fingerprint)}
                        >
                          Remove
                        </button>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}

          {activeTab === "transfers" && (
            <div>
              <div className="card">
                <div className="card-title">Send File</div>
                <p style={{ color: "var(--text-secondary)", marginBottom: 12 }}>
                  Files are streamed directly over authenticated QUIC streams with SHA-256 verification and path traversal protection.
                </p>
                <button className="btn btn-primary">Select File to Send</button>
              </div>

              <div className="card">
                <div className="card-title">Recent Transfers</div>
                {transfers.map((tx) => (
                  <div key={tx.id} className="peer-row">
                    <div>
                      <div style={{ fontWeight: 500 }}>{tx.fileName}</div>
                      <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                        {(tx.fileSize / 1024 / 1024).toFixed(2)} MB &bull; {tx.direction}
                      </div>
                    </div>
                    <span className="status-badge">
                      <span className="status-dot online" />
                      {tx.status}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === "clipboard" && (
            <div className="card">
              <div className="card-title">Clipboard Synchronization</div>
              <p style={{ color: "var(--text-secondary)", marginBottom: 16 }}>
                Automatically synchronizes text and images across connected peers with local echo loop suppression.
              </p>
              <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={clipboardSyncEnabled}
                  onChange={(e) => setClipboardSyncEnabled(e.target.checked)}
                />
                <span>Enable bidirectional clipboard sync</span>
              </label>
            </div>
          )}

          {activeTab === "notifications" && (
            <div>
              <div className="card">
                <div className="card-title">Notification Mirroring</div>
                <p style={{ color: "var(--text-secondary)", marginBottom: 16 }}>
                  Receive and dismiss alerts forwarded from connected phones and tablets.
                </p>
                <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer" }}>
                  <input
                    type="checkbox"
                    checked={notificationSyncEnabled}
                    onChange={(e) => setNotificationSyncEnabled(e.target.checked)}
                  />
                  <span>Enable notification forwarding</span>
                </label>
              </div>

              <div className="card">
                <div className="card-title">Forwarded Notifications</div>
                {notifications.map((n) => (
                  <div key={n.id} className="peer-row">
                    <div>
                      <div style={{ fontWeight: 500 }}>
                        {n.title} <span style={{ color: "var(--text-secondary)", fontSize: 12 }}>({n.appName})</span>
                      </div>
                      <div style={{ color: "var(--text-secondary)", marginTop: 2 }}>{n.body}</div>
                    </div>
                    <button className="btn">Dismiss</button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === "permissions" && (
            <div className="card">
              <div className="card-title">Four-Layer Capability Model</div>
              <p style={{ color: "var(--text-secondary)", marginBottom: 12 }}>
                Every capability request is strictly checked against platform availability, application permissions, peer trust level, and active session negotiation.
              </p>
              <ul style={{ paddingLeft: 20, color: "var(--text-secondary)" }}>
                <li><strong>File Transfer:</strong> Allowed for trusted peers</li>
                <li><strong>Clipboard Sync:</strong> Allowed for trusted peers</li>
                <li><strong>Notification Mirroring:</strong> Allowed for trusted peers</li>
              </ul>
            </div>
          )}
        </div>
      </main>
    </div>
  );
}

export default App;
