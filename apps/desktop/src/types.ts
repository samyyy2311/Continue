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

export interface RemoteFileItem {
  id: string;
  name: string;
  folder: "DCIM" | "Download" | "Documents" | "Movies" | "Music";
  category: "images" | "videos" | "documents" | "audio" | "other";
  size: number;
  modifiedAt: number;
}

export type AccentName = "recordRed" | "coral" | "amber" | "cyan" | "emerald" | "magenta" | "silver";

export interface AccentColor {
  id: AccentName;
  label: string;
  base: string;
  hover: string;
}

export const ACCENT_PALETTE: AccentColor[] = [
  { id: "recordRed", label: "Red", base: "#C23B30", hover: "#D64337" },
  { id: "coral", label: "Coral", base: "#FF5C35", hover: "#FF724E" },
  { id: "amber", label: "Amber", base: "#F59E0B", hover: "#FBBF24" },
  { id: "cyan", label: "Cyan", base: "#06B6D4", hover: "#22D3EE" },
  { id: "emerald", label: "Green", base: "#10B981", hover: "#34D399" },
  { id: "magenta", label: "Pink", base: "#EC4899", hover: "#F472B6" },
  { id: "silver", label: "Mono", base: "#C4C4C0", hover: "#E5E5E3" },
];
