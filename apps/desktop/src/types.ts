// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

export interface DeviceIdentity {
  fingerprint: string;
  spkiHash: string;
  deviceName: string;
}

export interface TrustedPeer {
  fingerprint: string;
  displayName: string;
  pairedAt: number;
  isConnected: boolean;
  endpoint?: string;
}

export interface TransferHistoryItem {
  id: string;
  fileName: string;
  fileSize: number;
  direction: "incoming" | "outgoing";
  peerFingerprint: string;
  status: "completed" | "in_progress" | "failed";
  timestamp: number;
}

export interface NotificationItem {
  id: string;
  appName: string;
  title: string;
  body: string;
  timestamp: number;
  peerFingerprint: string;
}
