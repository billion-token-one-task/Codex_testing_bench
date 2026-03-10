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
      if (method === "thread/start") {
        return { thread: { id: "thread-compat-1" } };
      }
      return {};
    }

    async close() {
      return await closeMock();
    }
  },
}));

describe("probeCodexCompatibility", () => {
  beforeEach(() => {
    requests.length = 0;
    vi.clearAllMocks();
  });

  it("probes the Codex app-server surface needed by OpenClaw", async () => {
    const { probeCodexCompatibility } = await import("./codex-managed.js");

    await probeCodexCompatibility({
      command: "/tmp/codex",
      workspaceDir: "/tmp/workspace",
      smokeTurn: true,
      model: "gpt-5.4",
    });

    expect(initializeMock).toHaveBeenCalledTimes(1);
    expect(closeMock).toHaveBeenCalledTimes(1);
    expect(requests.map((entry) => entry.method)).toEqual([
      "skills/list",
      "thread/start",
      "thread/read",
      "thread/compact/start",
      "thread/fork",
      "turn/start",
      "review/start",
    ]);

    expect(requests[1]).toEqual(
      expect.objectContaining({
        method: "thread/start",
        params: expect.objectContaining({
          model: "gpt-5.4",
          dynamicTools: [
            expect.objectContaining({
              name: "openclaw_smoke_ok",
            }),
          ],
          persistExtendedHistory: true,
          experimentalRawEvents: false,
          ephemeral: true,
        }),
      }),
    );
    expect(requests[2]).toEqual(
      expect.objectContaining({
        method: "thread/read",
        params: { threadId: "thread-compat-1", includeTurns: true },
      }),
    );
    expect(requests[3]).toEqual(
      expect.objectContaining({
        method: "thread/compact/start",
        params: { threadId: "thread-compat-1" },
      }),
    );
    expect(requests[4]).toEqual(
      expect.objectContaining({
        method: "thread/fork",
        params: expect.objectContaining({
          threadId: "thread-compat-1",
          persistExtendedHistory: true,
        }),
      }),
    );
  });
});
