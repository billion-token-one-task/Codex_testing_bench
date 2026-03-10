import path from "node:path";
import { CodexRpcClient } from "../agents/codex-rpc-client.js";
import type { OpenClawConfig } from "../config/config.js";
import type { SessionEntry } from "../config/sessions.js";
import type { SessionPreviewItem } from "./session-utils.types.js";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function normalizeCodexRuntime(cfg?: OpenClawConfig): { command: string; args: string[] } {
  const configured = cfg?.agents?.defaults?.codex;
  const rawArgs = Array.isArray(configured?.args) ? configured.args.filter(Boolean) : [];
  return {
    command: configured?.command?.trim() || "codex",
    args:
      rawArgs.length > 0 ? rawArgs : ["app-server", "--listen", configured?.listen ?? "stdio://"],
  };
}

function resolveCodexWorkspaceDir(params: {
  cfg: OpenClawConfig;
  entry: SessionEntry;
  workspaceDir?: string;
}): string {
  return path.resolve(
    params.workspaceDir?.trim() ||
      params.entry.systemPromptReport?.workspaceDir ||
      params.cfg.agents?.defaults?.workspace ||
      process.cwd(),
  );
}

async function withCodexThreadClient<T>(params: {
  cfg: OpenClawConfig;
  entry: SessionEntry;
  workspaceDir?: string;
  exitLabel: string;
  run: (client: CodexRpcClient, threadId: string) => Promise<T>;
}): Promise<T> {
  if (params.entry.engine?.kind !== "codex" || !params.entry.engine.threadId) {
    throw new Error("Codex thread metadata is missing.");
  }
  const runtime = normalizeCodexRuntime(params.cfg);
  const client = new CodexRpcClient(
    runtime.command,
    runtime.args,
    resolveCodexWorkspaceDir(params),
    {
      exitLabel: params.exitLabel,
    },
  );
  try {
    await client.initialize();
    return await params.run(client, params.entry.engine.threadId);
  } finally {
    await client.close().catch(() => undefined);
  }
}

function resolveMessageText(item: unknown): { role: string; text: string } | null {
  if (!isRecord(item) || typeof item.type !== "string") {
    return null;
  }
  switch (item.type) {
    case "userMessage": {
      const content = Array.isArray(item.content) ? item.content : [];
      const text = content
        .filter(isRecord)
        .filter((entry) => entry.type === "text" && typeof entry.text === "string")
        .map((entry) => String(entry.text))
        .join("\n")
        .trim();
      return text ? { role: "user", text } : null;
    }
    case "agentMessage": {
      const text = asString(item.text);
      return text ? { role: "assistant", text } : null;
    }
    case "plan":
    case "reasoning":
    case "enteredReviewMode":
    case "exitedReviewMode": {
      const text =
        asString(item.text) ??
        (Array.isArray(item.summary) ? item.summary.map(String).join("\n").trim() : undefined) ??
        asString(item.review);
      return text ? { role: "assistant", text } : null;
    }
    case "commandExecution": {
      const output = asString(item.aggregatedOutput);
      return output ? { role: "tool", text: output } : null;
    }
    default:
      return null;
  }
}

function toPreviewItems(messages: Array<{ role: string; text: string }>, limit: number, maxChars: number) {
  return messages
    .slice(-limit)
    .map((message) => ({
      role:
        message.role === "user" ||
        message.role === "assistant" ||
        message.role === "tool" ||
        message.role === "system"
          ? message.role
          : "other",
      text:
        message.text.length > maxChars ? `${message.text.slice(0, Math.max(1, maxChars - 1))}…` : message.text,
    })) satisfies SessionPreviewItem[];
}

export async function readCodexThreadMessages(params: {
  cfg: OpenClawConfig;
  entry: SessionEntry;
  workspaceDir?: string;
}): Promise<unknown[]> {
  if (params.entry.engine?.kind !== "codex" || !params.entry.engine.threadId) {
    return [];
  }
  return await withCodexThreadClient({
    ...params,
    exitLabel: "Codex app-server exited while reading history",
    run: async (client, threadId) => {
      const response = await client.request("thread/read", {
        threadId,
        includeTurns: true,
      });
    const thread = isRecord(response) && isRecord(response.thread) ? response.thread : undefined;
    const turns = Array.isArray(thread?.turns) ? thread.turns : [];
    const messages: unknown[] = [];
    for (const turn of turns) {
      const items = isRecord(turn) && Array.isArray(turn.items) ? turn.items : [];
      for (const item of items) {
        const message = resolveMessageText(item);
        if (!message) {
          continue;
        }
        messages.push({
          role: message.role,
          content: [{ type: "text", text: message.text }],
          __openclaw: {
            source: "codex-thread-read",
            threadId,
          },
        });
      }
    }
    return messages;
    },
  });
}

export async function readCodexThreadPreviewItems(params: {
  cfg: OpenClawConfig;
  entry: SessionEntry;
  workspaceDir?: string;
  limit: number;
  maxChars: number;
}): Promise<SessionPreviewItem[]> {
  const messages = await readCodexThreadMessages(params);
  const flattened = messages
    .map((message) => {
      if (!isRecord(message) || typeof message.role !== "string") {
        return null;
      }
      const content = Array.isArray(message.content) ? message.content : [];
      const text = content
        .filter(isRecord)
        .filter((entry) => entry.type === "text" && typeof entry.text === "string")
        .map((entry) => String(entry.text))
        .join("\n")
        .trim();
      return text ? { role: message.role, text } : null;
    })
    .filter((entry): entry is { role: string; text: string } => Boolean(entry));
  return toPreviewItems(flattened, params.limit, params.maxChars);
}

export async function compactCodexThread(params: {
  cfg: OpenClawConfig;
  entry: SessionEntry;
  workspaceDir?: string;
}): Promise<{ compacted: boolean; threadId?: string; reason?: string }> {
  if (params.entry.engine?.kind !== "codex" || !params.entry.engine.threadId) {
    return {
      compacted: false,
      reason: "no-thread-id",
    };
  }
  return await withCodexThreadClient({
    ...params,
    exitLabel: "Codex app-server exited while compacting history",
    run: async (client, threadId) => {
      await client.request("thread/compact/start", {
        threadId,
      });
      return {
        compacted: true,
        threadId,
      };
    },
  });
}
