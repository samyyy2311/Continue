// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

export interface DeviceIdentity {
  deviceName: string;
}

export interface TrustedPeer {
  fingerprint: string;
  displayName: string;
  /** The same four words show on the device, to check it's the one that was paired. */
  words: string;
  pairedAt: number;
  isConnected: boolean;
  endpoint?: string;
  /** Only while connected, once the device has reported it. Signal bars run 0–4; null without that network. */
  status: {
    percent: number;
    charging: boolean;
    cellBars: number | null;
    wifiBars: number | null;
    carrier: string;
    /** "5G", "LTE", "3G" or "2G"; empty when unknown. */
    network: string;
  } | null;
  /** The main colour of the phone's wallpaper, as #rrggbb, once it has said. */
  wallpaperColor: string | null;
  /** JPEG data URL, when the phone could read its wallpaper. */
  wallpaper: string | null;
}

/** True unless turned off for this device. */
export interface PeerPermission {
  capabilityId: number;
  allowed: boolean;
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

export type View = "transfer" | "messages" | "files" | "notifications" | "history" | "devices" | "search" | "settings";

export interface Contact {
  name: string;
  number: string;
  favorite: boolean;
  /** JPEG data URL. */
  photo: string | null;
}

/** Something on the phone that matched a search. */
export interface SearchResult {
  kind: "file" | "text" | "contact";
  title: string;
  /** The folder, the text itself, or the phone number. */
  detail: string;
  /** A file's path in the phone's shared folder, or a text's conversation id. */
  reference: string;
  /** Unix milliseconds; 0 for contacts. */
  at: number;
}
export type PhoneSide = "off" | "left" | "right";
export type Theme = "system" | "light" | "dark";
export type HistoryFilter = "all" | "file" | "text" | "failed" | "pinned";

/** A pinned clip, kept on every paired device. */
export interface Snippet {
  id: string;
  text: string;
}

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
  status: "waiting" | "sending" | "sent" | "failed" | "receiving" | "received";
  timestamp: number;
  path?: string;
  bytesSent?: number;
  totalBytes?: number;
  error?: string;
  /** For a file coming in, the id that cancels it. */
  transferId?: string;
}

/** Text or a file kept for a device that isn't connected, until it is. */
export interface WaitingItem {
  id: number;
  peerId: string;
  peerName: string;
  kind: "file" | "text";
  /** The text, or the file's path. */
  content: string;
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

/** By capability id. */
export const PERMISSIONS: Record<number, { label: string; description: string }> = {
  1: { label: "Files", description: "Send files to this computer" },
  2: { label: "Text", description: "Send copied text and links" },
  3: { label: "Notifications", description: "Show its notifications here" },
  4: { label: "New photos", description: "Show photos as you take them" },
  7: { label: "Calls", description: "Show calls as they ring" },
  10: { label: "Media", description: "Show what's playing" },
  12: { label: "Touchpad", description: "Move this computer's pointer and type here" },
  13: { label: "Control this computer", description: "Lock it, type into it and open links" },
};

export interface NowPlaying {
  peerId: string;
  /** Empty when nothing is playing. */
  title: string;
  artist: string;
  app: string;
  playing: boolean;
  durationMs: number;
  positionMs: number;
  /** JPEG data URL. */
  art: string | null;
}

export type MediaCommand = "PLAY_PAUSE" | "NEXT" | "PREVIOUS" | "VOLUME_UP" | "VOLUME_DOWN";

export interface PhoneCall {
  peerId: string;
  state: "ringing" | "talking" | "ended";
  number: string;
  /** Empty when not in contacts. */
  name: string;
}

export interface PhoneFile {
  name: string;
  folder: boolean;
  size: number;
  modified: number;
}

export interface Conversation {
  id: string;
  address: string;
  /** Empty when not in contacts. */
  name: string;
  snippet: string;
  at: number;
  unread: boolean;
}

export interface TextMessage {
  id: string;
  body: string;
  at: number;
  outgoing: boolean;
}

export interface Photo {
  id: string;
  name: string;
  takenAt: number;
  /** JPEG data URL. */
  thumbnail: string;
}

/** A notification from the phone, to read and act on here. */
export interface PhoneNotification {
  peerId: string;
  id: string;
  packageName: string;
  appName: string;
  title: string;
  text: string;
  /** Unix time in milliseconds. */
  postedAt: number;
  buttons: { id: string; label: string; isReply: boolean }[];
}

export const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPod|iPad/.test(navigator.platform);

export const MOD_KEY = isMac ? "⌘" : "Ctrl+";
export const MOD_SHIFT_KEY = isMac ? "⌘⇧" : "Ctrl+Shift+";
