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

export type AccentName = "blue" | "recordRed" | "coral" | "amber" | "cyan" | "emerald" | "magenta" | "silver";

export interface AccentColor {
  id: AccentName;
  label: string;
  base: string;
  /** Text color drawn on top of `base`, chosen for contrast. */
  onBase: string;
}

// Mid-tone colours that hold up on both light and dark surfaces.
export const ACCENT_PALETTE: AccentColor[] = [
  { id: "blue", label: "Blue", base: "#3d63dd", onBase: "#ffffff" },
  { id: "recordRed", label: "Red", base: "#d93d42", onBase: "#ffffff" },
  { id: "coral", label: "Coral", base: "#d4532b", onBase: "#ffffff" },
  { id: "amber", label: "Amber", base: "#f0b000", onBase: "#221a00" },
  { id: "cyan", label: "Teal", base: "#0b7f95", onBase: "#ffffff" },
  { id: "emerald", label: "Green", base: "#1e8a4c", onBase: "#ffffff" },
  { id: "magenta", label: "Pink", base: "#c0317a", onBase: "#ffffff" },
  // Follows the theme: dark on light, light on dark.
  { id: "silver", label: "Mono", base: "var(--text)", onBase: "var(--bg)" },
];
