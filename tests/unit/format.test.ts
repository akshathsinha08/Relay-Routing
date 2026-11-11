import { describe, expect, it } from "vitest";

import {
  conversationTitle,
  formatBytes,
  formatDuration,
  initials,
  isTaskActive,
  relativeTime,
  taskStatusLabel,
  truncate,
} from "@/lib/format";
import type { Conversation } from "@/types";

function conversation(overrides: Partial<Conversation>): Conversation {
  return {
    id: "c1",
    projectId: null,
    projectName: null,
    title: "",
    pinned: false,
    archived: false,
    providerId: null,
    accountId: null,
    model: null,
    summary: null,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    lastMessageAt: null,
    lastMessage: null,
    ...overrides,
  };
}

describe("format utilities", () => {
  it("formats byte sizes with sensible precision", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(999)).toBe("999 B");
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(20 * 1024 * 1024)).toBe("20 MB");
  });

  it("formats durations from timestamps", () => {
    const start = new Date(Date.now() - 65_000).toISOString();
    expect(formatDuration(start, new Date().toISOString())).toBe("1m 5s");
    expect(formatDuration(null)).toBe("—");
  });

  it("falls back to a placeholder conversation title", () => {
    expect(conversationTitle(conversation({ title: "  " }))).toBe("New conversation");
    expect(conversationTitle(conversation({ title: "Plan" }))).toBe("Plan");
  });

  it("truncates long strings with an ellipsis", () => {
    expect(truncate("short", 10)).toBe("short");
    expect(truncate("abcdefghij", 5)).toBe("abcd…");
  });

  it("derives initials", () => {
    expect(initials("Relay Routing")).toBe("RR");
    expect(initials("")).toBe("");
  });

  it("labels task statuses and detects active tasks", () => {
    expect(taskStatusLabel("waiting_for_input")).toBe("Waiting");
    expect(taskStatusLabel("mystery")).toBe("mystery");
    expect(isTaskActive({ status: "running" })).toBe(true);
    expect(isTaskActive({ status: "paused" })).toBe(false);
    expect(isTaskActive(null)).toBe(false);
  });

  it("renders recent timestamps relatively", () => {
    expect(relativeTime(new Date(Date.now() - 120_000).toISOString())).toBe("2m ago");
    expect(relativeTime(null)).toBe("");
  });
});
