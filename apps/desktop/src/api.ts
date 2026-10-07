// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DeviceIdentity,
  HistoryEntry,
  IncomingTransfer,
  PeerPermission,
  MediaCommand,
  NowPlaying,
  PhoneCall,
  PhoneFile,
  PhoneNotification,
  Photo,
  TextMessage,
  Conversation,
  TrustedPeer,
  WaitingItem,
  PhoneSide,
  Snippet,
  SearchResult,
  Contact,
} from "./types.ts";

export const getDeviceIdentity = () => invoke<DeviceIdentity>("get_device_identity");

export const getTrustedPeers = () => invoke<TrustedPeer[]>("get_trusted_peers");

export const removeTrustedPeer = (fingerprint: string) => invoke<boolean>("remove_trusted_peer", { fingerprint });

/** Opens a one-shot pairing listener and returns the code to show as a QR. */
export const startPairing = () => invoke<string>("start_pairing");

export const cancelPairing = () => invoke<void>("cancel_pairing");

/** Trusts the phone that paired, or forgets it. */
export const confirmPairing = (accept: boolean) => invoke<void>("confirm_pairing", { accept });

/** A phone paired; its six digits should match the ones this shows. */
export const onPairingCheck = (handler: (code: string) => void): Promise<UnlistenFn> =>
  listen<string>("pairing-check", (event) => handler(event.payload));

export const pairFromCode = (qrPayload: string) => invoke<TrustedPeer>("pair_from_qr", { qrPayload });

export const connectToPeer = (peerFingerprint: string, endpoint: string) =>
  invoke<void>("connect_to_peer", { peerFingerprint, endpoint });

export const disconnectPeer = (peerFingerprint: string) => invoke<void>("disconnect_peer", { peerFingerprint });

export const reconnectPeer = (peerFingerprint: string) => invoke<void>("reconnect_peer", { peerFingerprint });

export const getPermissions = (peerFingerprint: string) =>
  invoke<PeerPermission[]>("get_permissions", { peerFingerprint });

export const setAllowed = (peerFingerprint: string, capabilityId: number, allowed: boolean) =>
  invoke<void>("set_allowed", { peerFingerprint, capabilityId, allowed });

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

export const listIncoming = () => invoke<IncomingTransfer[]>("list_incoming");

export const cancelIncoming = (transferId: string) => invoke<void>("cancel_incoming", { transferId });

export const onIncomingProgress = (handler: (file: IncomingTransfer) => void): Promise<UnlistenFn> =>
  listen<IncomingTransfer>("incoming-progress", (event) => handler(event.payload));

export const onIncomingEnded = (handler: (transferId: string) => void): Promise<UnlistenFn> =>
  listen<string>("incoming-ended", (event) => handler(event.payload));

export const getSaveFolder = () => invoke<string>("get_save_folder");

/** Saves received files to `folder` from now on, or to Downloads when it's null. */
export const setSaveFolder = (folder: string | null) => invoke<string>("set_save_folder", { folder });

/** A small preview of a received image, as a data URL. */
export const thumbnail = (path: string) => invoke<string>("thumbnail", { path });

/** null when the phone hasn't granted photo access. */
export const listPhotos = (peerFingerprint: string) => invoke<Photo[] | null>("list_photos", { peerFingerprint });

export const onPhotoTaken = (handler: (peerId: string, photo: Photo) => void): Promise<UnlistenFn> =>
  listen<{ peerId: string; photo: Photo }>("photo-taken", (e) => handler(e.payload.peerId, e.payload.photo));

/** "" is the top folder; null when the phone isn't sharing its files. */
export const listPhoneFolder = (peerFingerprint: string, path: string) =>
  invoke<PhoneFile[] | null>("list_phone_folder", { peerFingerprint, path });

export const getPhoneFile = (peerFingerprint: string, path: string) =>
  invoke<void>("get_phone_file", { peerFingerprint, path });

/** null when the phone hasn't granted access to its contacts. */
export const listContacts = (peerFingerprint: string) => invoke<Contact[] | null>("list_contacts", { peerFingerprint });

export const searchPhone = (peerFingerprint: string, query: string) =>
  invoke<SearchResult[]>("search_phone", { peerFingerprint, query });

/** null when the phone hasn't granted SMS access. */
export const listConversations = (peerFingerprint: string) =>
  invoke<Conversation[] | null>("list_conversations", { peerFingerprint });

export const readConversation = (peerFingerprint: string, conversationId: string) =>
  invoke<TextMessage[]>("read_conversation", { peerFingerprint, conversationId });

export const sendSms = (peerFingerprint: string, address: string, body: string) =>
  invoke<void>("send_sms", { peerFingerprint, address, body });

export const onMessagesChanged = (handler: (peerId: string) => void): Promise<UnlistenFn> =>
  listen<string>("messages-changed", (e) => handler(e.payload));

/** Files passed in through Explorer's Send to menu. */
export const takeFilesToSend = () => invoke<string[]>("take_files_to_send");

export const onFilesToSend = (handler: () => void): Promise<UnlistenFn> => listen("files-to-send", handler);

export const getPhoto = (peerFingerprint: string, id: string) => invoke<void>("get_photo", { peerFingerprint, id });

/** Opens a web link in the default browser. */
export const openLink = (url: string) => invoke<void>("open_link", { url });

/** Opens a received file, or with `reveal` shows it in its folder. */
export const openReceived = (path: string, reveal: boolean) => invoke<void>("open_received", { path, reveal });

export const onPhoneCall = (handler: (call: PhoneCall) => void): Promise<UnlistenFn> =>
  listen<PhoneCall>("phone-call", (event) => handler(event.payload));

/** Something to do on the phone's screen. Positions are fractions of the screen. */
export type ScreenInput =
  | { kind: "down" | "move" | "up"; x: number; y: number }
  | { kind: "swipe"; fromX: number; fromY: number; toX: number; toY: number; durationMs: number }
  | { kind: "text"; text: string }
  | { kind: "back" | "home" | "recents" | "backspace" | "enter" };

/** Each frame arrives as a key frame flag, quarter turns to show it upright, then the H.264 data. */
export const startMirror = (peerFingerprint: string, onFrame: Channel<ArrayBuffer>) =>
  invoke<{ width: number; height: number }>("start_mirror", { peerFingerprint, onFrame });

export const screenInput = (input: ScreenInput) => invoke<void>("screen_input", { input });

export const stopMirror = () => invoke<void>("stop_mirror");

export const onMirrorEnded = (handler: () => void): Promise<UnlistenFn> => listen("mirror-ended", handler);

/** Frames arrive in the same format as startMirror's. */
export const startCamera = (peerFingerprint: string, front: boolean, onFrame: Channel<ArrayBuffer>) =>
  invoke<{ width: number; height: number }>("start_camera", { peerFingerprint, front, onFrame });

export const cameraControl = (control: { kind: "front"; front: boolean } | { kind: "framing"; on: boolean }) =>
  invoke<void>("camera_control", { control });

export const stopCamera = () => invoke<void>("stop_camera");

export const callCameraAvailable = () => invoke<boolean>("call_camera_available");

/** Turning it on can wait for Windows to ask permission the first time. */
export const setCallCamera = (on: boolean) => invoke<void>("set_call_camera", { on });

export const onCameraEnded = (handler: () => void): Promise<UnlistenFn> => listen("camera-ended", handler);

export const onNowPlaying = (handler: (playing: NowPlaying) => void): Promise<UnlistenFn> =>
  listen<NowPlaying>("now-playing", (event) => handler(event.payload));

export const mediaCommand = (peerFingerprint: string, command: MediaCommand) =>
  invoke<void>("media_command", { peerFingerprint, command });

export const ringPhone = (peerFingerprint: string, on: boolean) => invoke<void>("ring_phone", { peerFingerprint, on });

export const answerCall = (peerFingerprint: string) => invoke<void>("answer_call", { peerFingerprint });

export const declineCall = (peerFingerprint: string) => invoke<void>("decline_call", { peerFingerprint });

/** Stops the ringtone and leaves the call ringing. */
export const silenceCall = (peerFingerprint: string) => invoke<void>("silence_call", { peerFingerprint });

/** The call itself happens on the phone. */
export const callNumber = (peerFingerprint: string, number: string) =>
  invoke<void>("call_number", { peerFingerprint, number });

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

export const getWaiting = () => invoke<WaitingItem[]>("get_waiting");

export const openDropFolder = () => invoke<void>("open_drop_folder");

export const stopRinging = () => invoke<void>("stop_ringing");

/** The phone started or stopped looking for this computer. */
export const onRing = (handler: (on: boolean) => void): Promise<UnlistenFn> =>
  listen<boolean>("ring", (event) => handler(event.payload));

/** Null where this isn't available. */
export const getLockWhenAway = () => invoke<boolean | null>("get_lock_when_away");

export const setLockWhenAway = (on: boolean) => invoke<void>("set_lock_when_away", { on });

export const getSnippets = () => invoke<Snippet[]>("get_snippets");

export const pinSnippet = (text: string) => invoke<Snippet>("pin_snippet", { text });

export const unpinSnippet = (id: string) => invoke<void>("unpin_snippet", { id });

/** Another device pinned or removed something. */
export const onSnippetsChanged = (handler: () => void): Promise<UnlistenFn> => listen("snippets-changed", handler);

/** Null where the mouse and keyboard can't move to the phone. */
export const getPhoneSide = () => invoke<PhoneSide | null>("get_phone_side");

export const setPhoneSide = (choice: PhoneSide) => invoke<void>("set_phone_side", { choice });

/** The pointer reached the phone's side, but the phone can't take input. */
export const onPointerUnavailable = (handler: () => void): Promise<UnlistenFn> =>
  listen("pointer-unavailable", () => handler());

/** Resolves with the id the item waits under. */
export const sendLater = (peerFingerprint: string, isText: boolean, content: string) =>
  invoke<number>("send_later", { peerFingerprint, isText, content });

export const cancelWaiting = (id: number) => invoke<void>("cancel_waiting", { id });

/** A waiting item went, or failed, once its device connected. */
export const onWaitingSent = (handler: (id: number, sent: boolean) => void): Promise<UnlistenFn> =>
  listen<{ id: number; sent: boolean }>("waiting-sent", (event) => handler(event.payload.id, event.payload.sent));

export const onPairingCompleted = (handler: (peer: TrustedPeer) => void): Promise<UnlistenFn> =>
  listen<TrustedPeer>("pairing-completed", (event) => handler(event.payload));

export const onPairingFailed = (handler: (message: string) => void): Promise<UnlistenFn> =>
  listen<string>("pairing-failed", (event) => handler(event.payload));

export const onNotificationPosted = (handler: (notification: PhoneNotification) => void): Promise<UnlistenFn> =>
  listen<PhoneNotification>("notification-posted", (event) => handler(event.payload));

export const onNotificationRemoved = (handler: (id: string) => void): Promise<UnlistenFn> =>
  listen<string>("notification-removed", (event) => handler(event.payload));

/** Presses a button on a phone notification; `reply` is the text for a button that takes some. */
export const pressNotificationButton = (
  peerFingerprint: string,
  notificationId: string,
  buttonId: string,
  reply: string,
) => invoke<void>("press_notification_button", { peerFingerprint, notificationId, buttonId, reply });

/** Clears a notification on the phone too. */
export const dismissNotification = (peerFingerprint: string, notificationId: string) =>
  invoke<void>("dismiss_notification", { peerFingerprint, notificationId });

/** The phone stops forwarding the app's notifications until it's unmuted there. */
export const muteApp = (peerFingerprint: string, packageName: string) =>
  invoke<void>("mute_app", { peerFingerprint, packageName });

/** Commands reject with the backend's error string; anything else is unexpected. */
export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Something went wrong";
}
