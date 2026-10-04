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

/**
 * Formats a timestamp as relative time or a localized calendar date.
 *
 * @param timestamp - The time to format, in milliseconds since the Unix epoch
 * @param now - The reference time, in milliseconds since the Unix epoch
 * @returns “just now” for times under 45 seconds ago, rounded minutes or hours for times under 24 hours ago, “Yesterday” for the previous calendar day, or a localized date
 */
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
  if (date.toDateString() === yesterday.toDateString()) return "Yesterday";
  const sameYear = date.getFullYear() === new Date(now).getFullYear();
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric", year: sameYear ? undefined : "numeric" });
}

/**
 * Labels a timestamp as today, yesterday, or its localized weekday and date.
 *
 * @returns “Today,” “Yesterday,” or the localized weekday and date for the timestamp
 */
export function dayLabel(timestamp: number, now = Date.now()): string {
  const day = new Date(timestamp).toDateString();
  if (day === new Date(now).toDateString()) return "Today";
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  if (day === yesterday.toDateString()) return "Yesterday";
  return new Date(timestamp).toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" });
}

/**
 * Parses text containing only an HTTP or HTTPS URL.
 *
 * @param text - Text to parse; surrounding whitespace is ignored
 * @returns The normalized URL, or `null` if the text contains whitespace or is not a valid HTTP or HTTPS URL
 */
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
  if (["rs", "ts", "tsx", "js", "jsx", "html", "css", "py", "json", "c", "cpp", "go", "toml", "yaml", "yml"].includes(ext)) {
    return "code";
  }
  if (["pdf", "doc", "docx", "txt", "md", "rtf", "xls", "xlsx", "csv"].includes(ext)) {
    return "document";
  }
  return "other";
}

