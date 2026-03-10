import { describe, expect, it } from "vitest";
import { buildCodexCompactionEvent } from "./codex-engine.js";

describe("buildCodexCompactionEvent", () => {
  it("maps context compaction item lifecycle into OpenClaw compaction events", () => {
    expect(
      buildCodexCompactionEvent("item/started", {
        threadId: "thread-1",
        turn: { id: "turn-1" },
        item: { id: "item-1", type: "contextCompaction" },
      }),
    ).toEqual({
      phase: "start",
      threadId: "thread-1",
      turnId: "turn-1",
      itemId: "item-1",
      itemType: "contextCompaction",
    });

    expect(
      buildCodexCompactionEvent("item/completed", {
        threadId: "thread-1",
        turn: { id: "turn-1" },
        item: { id: "item-1", type: "contextCompaction" },
      }),
    ).toEqual({
      phase: "end",
      threadId: "thread-1",
      turnId: "turn-1",
      itemId: "item-1",
      itemType: "contextCompaction",
    });
  });

  it("maps direct context/compacted notifications into completion events", () => {
    expect(
      buildCodexCompactionEvent("context/compacted", {
        threadId: "thread-1",
        previousTokens: 1200,
        newTokens: 320,
      }),
    ).toEqual({
      phase: "end",
      threadId: "thread-1",
      previousTokens: 1200,
      newTokens: 320,
    });
  });

  it("ignores unrelated Codex notifications", () => {
    expect(
      buildCodexCompactionEvent("item/started", {
        threadId: "thread-1",
        turn: { id: "turn-1" },
        item: { id: "item-1", type: "agentMessage" },
      }),
    ).toBeNull();
  });
});
