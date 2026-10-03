// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

export interface DeviceIdentity {
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

export type AccentName = "cobalt" | "teal" | "recordRed" | "coral" | "amber" | "emerald" | "magenta" | "silver";

export interface AccentColor {
  id: AccentName;
  label: string;
  base: string;
  /** Text color drawn on top of `base`, chosen for contrast. */
  onBase: string;
}

// Mid-tone colours that hold up on both light and dark surfaces.
export const ACCENT_PALETTE: AccentColor[] = [
  // The Android app's colour, with a light and a dark value.
  { id: "cobalt", label: "Cobalt", base: "var(--brand)", onBase: "var(--on-brand)" },
  { id: "teal", label: "Teal", base: "#1f7f7c", onBase: "#ffffff" },
  { id: "recordRed", label: "Red", base: "#d93d42", onBase: "#ffffff" },
  { id: "coral", label: "Coral", base: "#d4532b", onBase: "#ffffff" },
  { id: "amber", label: "Amber", base: "#f0b000", onBase: "#221a00" },
  { id: "emerald", label: "Green", base: "#1e8a4c", onBase: "#ffffff" },
  { id: "magenta", label: "Pink", base: "#c0317a", onBase: "#ffffff" },
  // Follows the theme: dark on light, light on dark.
  { id: "silver", label: "Mono", base: "var(--text)", onBase: "var(--bg)" },
];

export type View = "transfer" | "devices" | "history" | "settings";
export type Theme = "system" | "light" | "dark";
export type HistoryFilter = "all" | "file" | "text" | "failed";

export interface Toast {
  message: string;
  tone: "info" | "error";
}

export interface Activity {
  id: string;
  kind: "file" | "text";
  label: string;
  peerId: string;
  peerName: string;
  status: "sending" | "sent" | "failed" | "receiving" | "received";
  timestamp: number;
  path?: string;
  bytesSent?: number;
  totalBytes?: number;
  error?: string;
  /** For a file coming in, the id that cancels it. */
  transferId?: string;
}

/** A file on its way in. */
export interface IncomingTransfer {
  transferId: string;
  peerId: string;
  peerName: string;
  fileName: string;
  received: number;
  total: number;
}

/** A saved send or receive, newest first from `getHistory`. */
export interface HistoryEntry {
  id: number;
  /** Unix time in milliseconds. */
  at: number;
  received: boolean;
  kind: "file" | "text";
  label: string;
  peerId: string;
  peerName: string;
  size: number;
  failed: boolean;
  location: string | null;
}

/** Something a device set to Ask wants to send. */
export interface PermissionQuestion {
  id: number;
  peerName: string;
  kind: "file" | "text" | "notification";
  /** The file name, for files. */
  detail: string | null;
}

export type PermissionAnswer = "allow" | "alwaysAllow" | "decline";

export const PERMISSIONS: Record<string, { label: string; description: string }> = {
  "File Transfer": { label: "Files", description: "Send files to this computer" },
  "Clipboard Sync": { label: "Text", description: "Send copied text and links" },
  "Notification Relay": { label: "Notifications", description: "Show its notifications here" },
};

export const GRANT_OPTIONS: { value: Grant; label: string }[] = [
  { value: "Allow", label: "Allow" },
  { value: "Ask", label: "Ask" },
  { value: "Deny", label: "Block" },
];

export const isMac =
  typeof navigator !== "undefined" && /Mac|iPhone|iPod|iPad/.test(navigator.platform);

export const MOD_KEY = isMac ? "⌘" : "Ctrl+";
export const MOD_SHIFT_KEY = isMac ? "⌘⇧" : "Ctrl+Shift+";

