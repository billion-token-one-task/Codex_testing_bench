import { beforeEach, describe, expect, it, vi } from "vitest";

const requests: Array<{ method: string; params: unknown }> = [];
const initializeMock = vi.fn(async () => undefined);
const closeMock = vi.fn(async () => undefined);

vi.mock("../agents/codex-rpc-client.js", () => ({
  CodexRpcClient: class MockCodexRpcClient {
    async initialize() {
      return await initializeMock();
    }

    async request(method: string, params?: unknown) {
      requests.push({ method, params });
      if (method === "thread/read") {
        return {
          thread: {
            turns: [
              {
                id: "turn-1",
                items: [
                  {
                    id: "item-user-1",
                    type: "userMessage",
                    content: [{ type: "text", text: "hello from fake codex" }],
                  },
                  {
                    id: "item-agent-1",
                    type: "agentMessage",
                    text: "hello from assistant",
                  },
                  {
                    id: "item-command-1",
                    type: "commandExecution",
                    aggregatedOutput: "stdout: ok",
                  },
                ],
              },
            ],
          },
        };
      }
      return {};
    }

    async close() {
      return await closeMock();
    }
  },
}));

describe("codex thread history helpers", () => {
  beforeEach(() => {
    requests.length = 0;
    vi.clearAllMocks();
  });

  it("reads Codex thread history and preview items through thread/read", async () => {
    const { readCodexThreadMessages, readCodexThreadPreviewItems } = await import(
      "./codex-thread-history.js"
    );

    const cfg = {
      agents: {
        defaults: {
          workspace: "/tmp/workspace",
          codex: {
            command: "/tmp/codex",
            args: ["app-server", "--listen", "stdio://"],
          },
        },
      },
    };
    const entry = {
      sessionId: "sess-codex",
      updatedAt: Date.now(),
      engine: {
        kind: "codex" as const,
        threadId: "thread-1",
      },
    };

    const messages = await readCodexThreadMessages({
      cfg,
      entry: entry as never,
      workspaceDir: "/tmp/workspace",
    });
    expect(messages).toEqual([
      {
        role: "user",
        content: [{ type: "text", text: "hello from fake codex" }],
        __openclaw: {
          source: "codex-thread-read",
          threadId: "thread-1",
        },
      },
      {
        role: "assistant",
        content: [{ type: "text", text: "hello from assistant" }],
        __openclaw: {
          source: "codex-thread-read",
          threadId: "thread-1",
        },
      },
      {
        role: "tool",
        content: [{ type: "text", text: "stdout: ok" }],
        __openclaw: {
          source: "codex-thread-read",
          threadId: "thread-1",
        },
      },
    ]);

    const preview = await readCodexThreadPreviewItems({
      cfg,
      entry: entry as never,
      workspaceDir: "/tmp/workspace",
      limit: 2,
      maxChars: 40,
    });
    expect(preview).toEqual([
      { role: "assistant", text: "hello from assistant" },
      { role: "tool", text: "stdout: ok" },
    ]);

    expect(initializeMock).toHaveBeenCalledTimes(2);
    expect(closeMock).toHaveBeenCalledTimes(2);
    expect(requests).toEqual([
      {
        method: "thread/read",
        params: { threadId: "thread-1", includeTurns: true },
      },
      {
        method: "thread/read",
        params: { threadId: "thread-1", includeTurns: true },
      },
    ]);
  });

  it("starts Codex-native compaction through thread/compact/start", async () => {
    const { compactCodexThread } = await import("./codex-thread-history.js");

    const result = await compactCodexThread({
      cfg: {
        agents: {
          defaults: {
            workspace: "/tmp/workspace",
            codex: {
              command: "/tmp/codex",
              args: ["app-server", "--listen", "stdio://"],
            },
          },
        },
      },
      entry: {
        sessionId: "sess-codex",
        updatedAt: Date.now(),
        engine: {
          kind: "codex",
          threadId: "thread-1",
        },
      } as never,
      workspaceDir: "/tmp/workspace",
    });

    expect(result).toEqual({
      compacted: true,
      threadId: "thread-1",
    });
    expect(requests).toEqual([
      {
        method: "thread/compact/start",
        params: { threadId: "thread-1" },
      },
    ]);
  });
});
