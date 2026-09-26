// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DeviceIdentity, Grant, PeerPermission, TrustedPeer } from "./types.ts";

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

export const getPermissions = (peerFingerprint: string) =>
  invoke<PeerPermission[]>("get_permissions", { peerFingerprint });

export const setPermission = (peerFingerprint: string, capabilityId: number, grant: Grant) =>
  invoke<void>("set_permission", { peerFingerprint, capabilityId, grant });

/** Resolves with the number of bytes sent once the peer has the whole file. */
export const sendFileToPeer = (peerFingerprint: string, filePath: string) =>
  invoke<number>("send_file_to_peer", { peerFingerprint, filePath });

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
