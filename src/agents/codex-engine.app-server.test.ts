import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { runCodexAgent } from "./codex-engine.js";

const fakeCodexServerPath = fileURLToPath(new URL("../test-fixtures/fake-codex-app-server.mjs", import.meta.url));

describe("runCodexAgent", () => {
  it("uses Codex app-server as the harness and preserves Codex-native compaction events", async () => {
    const workspaceDir = await fs.mkdtemp(path.join(os.tmpdir(), "openclaw-codex-engine-"));
    const sessionFile = path.join(workspaceDir, "session.jsonl");
    const events: Array<{ stream: string; data?: Record<string, unknown> }> = [];

    try {
      const result = await runCodexAgent({
        sessionId: "sess-codex",
        sessionKey: "agent:dev:main",
        sessionFile,
        workspaceDir,
        prompt: "Say READY.",
        timeoutMs: 5_000,
        runId: "run-codex",
        config: {
          agents: {
            defaults: {
              workspace: workspaceDir,
              codex: {
                command: process.execPath,
                args: [fakeCodexServerPath],
                defaultModel: "gpt-5.4",
                provider: "openai",
                approvalPolicy: "on-request",
                sandbox: "workspace-write",
                experimentalApi: true,
              },
            },
          },
        },
        onAgentEvent: (event) => {
          events.push(event);
        },
      });

      expect(result.payloads).toEqual([{ text: "READY" }]);
      expect(result.meta.agentMeta).toMatchObject({
        sessionId: "sess-codex",
        provider: "codex-cli",
        model: "gpt-5.4",
        engine: {
          kind: "codex",
          threadId: "thread-1",
          lastTurnId: "turn-1",
        },
      });
      expect(events).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            stream: "compaction",
            data: expect.objectContaining({ phase: "start", itemType: "contextCompaction" }),
          }),
          expect.objectContaining({
            stream: "compaction",
            data: expect.objectContaining({ phase: "end", itemType: "contextCompaction" }),
          }),
          expect.objectContaining({
            stream: "assistant",
            data: expect.objectContaining({ delta: "READY", text: "READY" }),
          }),
        ]),
      );
    } finally {
      await fs.rm(workspaceDir, { recursive: true, force: true });
    }
  });
});
