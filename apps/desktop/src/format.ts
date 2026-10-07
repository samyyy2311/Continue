// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

/** Paths come from the OS file dialog or drag-and-drop, so both separators occur. */
export function fileNameFromPath(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export function formatRelativeTime(timestamp: number, now = Date.now()): string {
  const seconds = Math.round((now - timestamp) / 1000);
  if (seconds < 45) return "just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const date = new Date(timestamp);
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (date.toDateString() === yesterday.toDateString()) return "yesterday";
  const sameYear = date.getFullYear() === new Date(now).getFullYear();
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric", year: sameYear ? undefined : "numeric" });
}

/** A heading for the day something happened: Today, Yesterday, or the date. */
export function dayLabel(timestamp: number, now = Date.now()): string {
  const day = new Date(timestamp).toDateString();
  if (day === new Date(now).toDateString()) return "Today";
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (day === yesterday.toDateString()) return "Yesterday";
  return new Date(timestamp).toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" });
}

/** Splits newest-first items into days, keeping the order. */
export function byDay<T>(items: T[], timestamp: (item: T) => number): [string, T[]][] {
  const days: [string, T[]][] = [];
  for (const item of items) {
    const day = dayLabel(timestamp(item));
    const last = days[days.length - 1];
    if (last?.[0] === day) last[1].push(item);
    else days.push([day, [item]]);
  }
  return days;
}

/** The web link, if the text is nothing but one. */
export function linkIn(text: string): string | null {
  const trimmed = text.trim();
  if (/\s/.test(trimmed)) return null;
  try {
    const url = new URL(trimmed);
    return url.protocol === "https:" || url.protocol === "http:" ? url.href : null;
  } catch {
    return null;
  }
}

/** The code in a text that says it's one, such as "Your verification code is 482913". */
export function oneTimeCode(text: string): string | null {
  if (!/\b(code|otp|pin|passcode|verification|verify)\b/i.test(text)) return null;
  return text.match(/(?<![\d-])\d{4,8}(?![\d-])/)?.[0] ?? null;
}

/** Every web link in a text, without the punctuation that ends a sentence. */
export function linksIn(text: string): string[] {
  return (text.match(/https?:\/\/\S+/g) ?? [])
    .map((link) => link.replace(/[.,!?)\]]+$/, ""))
    .filter((link) => linkIn(link) !== null);
}

export type FileCategory = "image" | "video" | "audio" | "archive" | "code" | "document" | "other";

export function getFileCategory(name: string): FileCategory {
  const ext = name.includes(".") ? (name.split(".").pop() ?? "").toLowerCase() : "";
  if (["png", "jpg", "jpeg", "gif", "webp", "heic", "svg", "dng", "bmp"].includes(ext)) {
    return "image";
  }
  if (["mp4", "mov", "mkv", "avi", "webm", "flv"].includes(ext)) {
    return "video";
  }
  if (["mp3", "m4a", "wav", "flac", "ogg", "aac"].includes(ext)) {
    return "audio";
  }
  if (["zip", "rar", "7z", "tar", "gz", "apk", "dmg"].includes(ext)) {
    return "archive";
  }
  if (
    ["rs", "ts", "tsx", "js", "jsx", "html", "css", "py", "json", "c", "cpp", "go", "toml", "yaml", "yml"].includes(ext)
  ) {
    return "code";
  }
  if (["pdf", "doc", "docx", "txt", "md", "rtf", "xls", "xlsx", "csv"].includes(ext)) {
    return "document";
  }
  return "other";
}
