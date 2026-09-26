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

export type Grant = "Allow" | "Ask" | "Deny" | "AllowOnce";

export interface PeerPermission {
  capabilityId: number;
  capabilityName: string;
  grant: Grant;
}

export interface TransferHistoryItem {
  id: string;
  fileName: string;
  peerFingerprint: string;
  status: "in_progress" | "completed" | "failed";
  timestamp: number;
  bytesSent?: number;
  error?: string;
}

export type AccentName = "blue" | "recordRed" | "coral" | "amber" | "cyan" | "emerald" | "magenta" | "silver";

export interface AccentColor {
  id: AccentName;
  label: string;
  base: string;
  /** Text color drawn on top of `base`, chosen for contrast. */
  onBase: string;
}

export const ACCENT_PALETTE: AccentColor[] = [
  { id: "blue", label: "Blue", base: "#2563EB", onBase: "#FFFFFF" },
  { id: "recordRed", label: "Red", base: "#C23B30", onBase: "#FFFFFF" },
  { id: "coral", label: "Coral", base: "#FF5C35", onBase: "#1A0A05" },
  { id: "amber", label: "Amber", base: "#F59E0B", onBase: "#1A1204" },
  { id: "cyan", label: "Cyan", base: "#06B6D4", onBase: "#03181C" },
  { id: "emerald", label: "Green", base: "#10B981", onBase: "#03170F" },
  { id: "magenta", label: "Pink", base: "#EC4899", onBase: "#FFFFFF" },
  { id: "silver", label: "Mono", base: "#52525B", onBase: "#FFFFFF" },
];
