// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from "vitest";
import { fileNameFromPath, formatBytes, formatRelativeTime, shortFingerprint } from "./format.ts";

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
    expect(formatRelativeTime(now - 10_000, now)).toBe("Just now");
  });

  it("uses minutes and hours for the same day", () => {
    expect(formatRelativeTime(now - 5 * 60_000, now)).toBe("5 min ago");
    expect(formatRelativeTime(now - 3 * 3_600_000, now)).toBe("3 h ago");
  });
});

describe("shortFingerprint", () => {
  it("keeps both ends of long fingerprints", () => {
    expect(shortFingerprint("cont1q8f7e2a9d4c6b8a1e3f5a7b9c1d3e5f7a9b1c3d")).toBe("cont1q8f7e…9b1c3d");
  });

  it("leaves short values unchanged", () => {
    expect(shortFingerprint("cont1qabc")).toBe("cont1qabc");
  });
});
