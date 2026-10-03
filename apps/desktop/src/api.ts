// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DeviceIdentity,
  Grant,
  HistoryEntry,
  PeerPermission,
  PermissionAnswer,
  PermissionQuestion,
  TrustedPeer,
} from "./types.ts";

export const getDeviceIdentity = () => invoke<DeviceIdentity>("get_device_identity");

export const getTrustedPeers = () => invoke<TrustedPeer[]>("get_trusted_peers");

export const removeTrustedPeer = (fingerprint: string) =>
  invoke<boolean>("remove_trusted_peer", { fingerprint });

/** Opens a one-shot pairing listener and returns the code to show as a QR. */
export const startPairing = () => invoke<string>("start_pairing");

export const cancelPairing = () => invoke<void>("cancel_pairing");

export const pairFromCode = (qrPayload: string) =>
  invoke<TrustedPeer>("pair_from_qr", { qrPayload });

export const connectToPeer = (peerFingerprint: string, endpoint: string) =>
  invoke<void>("connect_to_peer", { peerFingerprint, endpoint });

export const disconnectPeer = (peerFingerprint: string) =>
  invoke<void>("disconnect_peer", { peerFingerprint });

export const reconnectPeer = (peerFingerprint: string) =>
  invoke<void>("reconnect_peer", { peerFingerprint });

export const getPermissions = (peerFingerprint: string) =>
  invoke<PeerPermission[]>("get_permissions", { peerFingerprint });

export const setPermission = (peerFingerprint: string, capabilityId: number, grant: Grant) =>
  invoke<void>("set_permission", { peerFingerprint, capabilityId, grant });

export const getHistory = () => invoke<HistoryEntry[]>("get_history");

export const clearHistory = () => invoke<void>("clear_history");

/** Saves a pasted file to disk so it can be sent, and returns its path. */
export const savePastedFile = async (file: File, name: string) =>
  invoke<string>("save_pasted_file", new Uint8Array(await file.arrayBuffer()), {
    headers: { "x-file-name": encodeURIComponent(name) },
  });

/** Turns sending what's copied on this computer on or off. */
export const setClipboardSyncEnabled = (enabled: boolean) => invoke<void>("set_clipboard_sync", { enabled });

export const getAutostart = () => invoke<boolean>("get_autostart");

export const setAutostart = (enabled: boolean) => invoke<void>("set_autostart", { enabled });

/** Opens a web link in the default browser. */
export const openLink = (url: string) => invoke<void>("open_link", { url });

/** Opens a received file, or with `reveal` shows it in its folder. */
export const openReceived = (path: string, reveal: boolean) => invoke<void>("open_received", { path, reveal });

export const answerPermission = (id: number, answer: PermissionAnswer) =>
  invoke<void>("answer_permission", { id, answer });

export const onPermissionRequest = (handler: (question: PermissionQuestion) => void): Promise<UnlistenFn> =>
  listen<PermissionQuestion>("permission-request", (event) => handler(event.payload));

/** Questions asked before the window started listening. */
export const pendingPermissionQuestions = () => invoke<PermissionQuestion[]>("pending_permission_questions");

/** The question went unanswered for too long and was declined. */
export const onPermissionRequestClosed = (handler: (id: number) => void): Promise<UnlistenFn> =>
  listen<number>("permission-request-closed", (event) => handler(event.payload));

export interface TransferProgress {
  bytesSent: number;
  totalBytes: number;
}

/** Resolves with the number of bytes sent once the peer has the whole file. */
export function sendFileToPeer(
  peerFingerprint: string,
  filePath: string,
  onProgress: (progress: TransferProgress) => void,
) {
  const channel = new Channel<TransferProgress>();
  channel.onmessage = onProgress;
  return invoke<number>("send_file_to_peer", { peerFingerprint, filePath, onProgress: channel });
}

export const sendClipboardText = (peerFingerprint: string, text: string) =>
  invoke<void>("send_clipboard_text", { peerFingerprint, text });

export const onPairingCompleted = (handler: (peer: TrustedPeer) => void): Promise<UnlistenFn> =>
  listen<TrustedPeer>("pairing-completed", (event) => handler(event.payload));

export const onPairingFailed = (handler: (message: string) => void): Promise<UnlistenFn> =>
  listen<string>("pairing-failed", (event) => handler(event.payload));

/** Commands reject with the backend's error string; anything else is unexpected. */
export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong";
}
