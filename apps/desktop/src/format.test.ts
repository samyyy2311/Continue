// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from "vitest";
import { dayLabel, fileNameFromPath, formatBytes, formatRelativeTime, getFileCategory, linkIn } from "./format.ts";

describe("formatBytes", () => {
  it("picks the largest unit below the value", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(2097152)).toBe("2.0 MB");
    expect(formatBytes(3 * 1024 ** 3)).toBe("3.00 GB");
  });
});

describe("fileNameFromPath", () => {
  it("handles Unix and Windows separators", () => {
    expect(fileNameFromPath("/home/me/Pictures/cat.png")).toBe("cat.png");
    expect(fileNameFromPath("C:\\Users\\me\\report.pdf")).toBe("report.pdf");
  });

  it("returns the input when there is no separator", () => {
    expect(fileNameFromPath("notes.txt")).toBe("notes.txt");
  });
});

describe("formatRelativeTime", () => {
  const now = 1_700_000_000_000;

  it("treats the last few seconds as just now", () => {
    expect(formatRelativeTime(now - 10_000, now)).toBe("just now");
  });

  it("uses minutes and hours for the same day", () => {
    expect(formatRelativeTime(now - 5 * 60_000, now)).toBe("5 min ago");
    expect(formatRelativeTime(now - 3 * 3_600_000, now)).toBe("3 h ago");
  });

  it("names yesterday instead of giving a date", () => {
    const evening = new Date(2026, 9, 3, 22).getTime();
    expect(formatRelativeTime(new Date(2026, 9, 2, 9).getTime(), evening)).toBe("yesterday");
  });
});

describe("getFileCategory", () => {
  it("classifies file extensions accurately", () => {
    expect(getFileCategory("photo.JPEG")).toBe("image");
    expect(getFileCategory("movie.mp4")).toBe("video");
    expect(getFileCategory("podcast.flac")).toBe("audio");
    expect(getFileCategory("backup.tar.gz")).toBe("archive");
    expect(getFileCategory("main.rs")).toBe("code");
    expect(getFileCategory("invoice.pdf")).toBe("document");
    expect(getFileCategory("unknown.xyz123")).toBe("other");
  });
});

describe("linkIn", () => {
  it("finds a link sent on its own", () => {
    expect(linkIn("  https://example.com/a?b=1 ")).toBe("https://example.com/a?b=1");
  });

  it("ignores links inside other text and other kinds of address", () => {
    expect(linkIn("see https://example.com")).toBeNull();
    expect(linkIn("file:///etc/passwd")).toBeNull();
    expect(linkIn("Gate B12")).toBeNull();
  });
});

describe("dayLabel", () => {
  const evening = new Date(2026, 9, 3, 22).getTime();

  it("names today and yesterday", () => {
    expect(dayLabel(new Date(2026, 9, 3, 1).getTime(), evening)).toBe("Today");
    expect(dayLabel(new Date(2026, 9, 2, 23).getTime(), evening)).toBe("Yesterday");
  });
});
