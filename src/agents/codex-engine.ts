import { spawn } from "node:child_process";
import { once } from "node:events";
import readline from "node:readline";
import type { AgentTool, AgentToolResult } from "@mariozechner/pi-agent-core";
import type { ImageContent, TextContent } from "@mariozechner/pi-ai";
import { resolveHeartbeatPrompt } from "../auto-reply/heartbeat.js";
import type { ThinkLevel, VerboseLevel } from "../auto-reply/thinking.js";
import type { OpenClawConfig } from "../config/config.js";
import type { SessionSystemPromptReport } from "../config/sessions/types.js";
import {
  DEFAULT_OPERATOR_REQUEST_TIMEOUT_MS,
  type OperatorRequestKind,
} from "../gateway/operator-request-manager.js";
import { requestOperatorResolution } from "../gateway/operator-request-runtime.js";
import { emitAgentEvent } from "../infra/agent-events.js";
import { createSubsystemLogger } from "../logging/subsystem.js";
import { resolveSessionAgentIds } from "./agent-scope.js";
import {
  analyzeBootstrapBudget,
  buildBootstrapInjectionStats,
  buildBootstrapPromptWarning,
  buildBootstrapTruncationReportMeta,
} from "./bootstrap-budget.js";
import { makeBootstrapWarn, resolveBootstrapContextForRun } from "./bootstrap-files.js";
import { writeCliImages } from "./cli-runner/helpers.js";
import { resolveOpenClawDocsPath } from "./docs-path.js";
import { FailoverError } from "./failover-error.js";
import {
  resolveBootstrapMaxChars,
  resolveBootstrapPromptTruncationWarningMode,
  resolveBootstrapTotalMaxChars,
} from "./pi-embedded-helpers.js";
import type { EmbeddedPiRunResult } from "./pi-embedded-runner.js";
import { buildSystemPromptReport } from "./system-prompt-report.js";
import { buildSystemPrompt } from "./cli-runner/helpers.js";
import { createOpenClawTools } from "./openclaw-tools.js";
import { wrapOwnerOnlyToolExecution } from "./tools/common.js";

const log = createSubsystemLogger("agent/codex-engine");

type JsonRpcId = number;

type JsonRpcRequest = {
  id: JsonRpcId;
  method: string;
  params?: unknown;
};

type JsonRpcResponse =
  | {
      id: JsonRpcId;
      result: unknown;
      error?: undefined;
    }
  | {
      id: JsonRpcId;
      result?: undefined;
      error: { code?: number; message?: string; data?: unknown };
    };

type JsonRpcMessage = JsonRpcRequest | JsonRpcResponse | { method: string; params?: unknown };

export type CodexThreadMeta = {
  kind: "codex";
  threadId: string;
  lastTurnId?: string;
  threadStatus?: string;
  runtimeOrigin?: string;
  protocolVersion?: string;
  compatibilityVersion?: string;
};

type CodexDynamicToolSpec = {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
};

type CodexDynamicToolOutputContentItem =
  | { type: "inputText"; text: string }
  | { type: "inputImage"; imageUrl: string };

type CodexToolCallParams = {
  threadId: string;
  turnId: string;
  callId: string;
  tool: string;
  arguments: unknown;
};

type CodexEngineConfig = {
  command: string;
  args: string[];
  env?: Record<string, string>;
  defaultModel: string;
  modelProvider: string;
  approvalPolicy: "untrusted" | "on-failure" | "on-request" | "never";
  sandbox: "read-only" | "workspace-write" | "danger-full-access";
  minimumVersion?: string;
  experimentalApi: boolean;
};

type CodexAgentParams = {
  sessionId: string;
  sessionKey?: string;
  sessionFile: string;
  agentId?: string;
  workspaceDir: string;
  prompt: string;
  config?: OpenClawConfig;
  model?: string;
  provider?: string;
  thinkLevel?: ThinkLevel;
  verboseLevel?: VerboseLevel;
  timeoutMs: number;
  runId: string;
  extraSystemPrompt?: string;
  images?: ImageContent[];
  ownerNumbers?: string[];
  senderIsOwner?: boolean;
  messageChannel?: string;
  agentAccountId?: string;
  messageTo?: string;
  messageThreadId?: string | number;
  groupId?: string;
  groupChannel?: string;
  groupSpace?: string;
  currentChannelId?: string;
  currentThreadTs?: string;
  currentMessageId?: string | number;
  replyToMode?: "off" | "first" | "all";
  hasRepliedRef?: { value: boolean };
  agentDir?: string;
  requesterSenderId?: string | null;
  spawnedBy?: string;
  sessionEngine?: CodexThreadMeta;
  bootstrapPromptWarningSignaturesSeen?: string[];
  bootstrapPromptWarningSignature?: string;
  onAgentEvent?: (evt: { stream: string; data?: Record<string, unknown> }) => void;
  abortSignal?: AbortSignal;
};

type PendingRequest = {
  resolve: (value: unknown) => void;
  reject: (error: unknown) => void;
};

type CodexServerRequestMethod =
  | "item/tool/call"
  | "item/commandExecution/requestApproval"
  | "item/fileChange/requestApproval"
  | "item/permissions/requestApproval"
  | "item/tool/requestUserInput"
  | "mcpServer/elicitation/request";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function isTextToolContent(value: unknown): value is TextContent {
  return isRecord(value) && value.type === "text" && typeof value.text === "string";
}

function isImageToolContent(value: unknown): value is ImageContent {
  return (
    isRecord(value) &&
    value.type === "image" &&
    typeof value.data === "string" &&
    typeof value.mimeType === "string"
  );
}

function isJsonRpcSuccessResponse(message: unknown): message is Extract<JsonRpcResponse, { result: unknown }> {
  return isRecord(message) && typeof message.id === "number" && "result" in message;
}

function isJsonRpcErrorResponse(message: unknown): message is Extract<JsonRpcResponse, { error: unknown }> {
  return isRecord(message) && typeof message.id === "number" && "error" in message;
}

function isJsonRpcRequest(message: unknown): message is JsonRpcRequest {
  return isRecord(message) && typeof message.method === "string" && typeof message.id === "number";
}

function isJsonRpcNotification(message: unknown): message is Extract<JsonRpcMessage, { method: string }> {
  return (
    isRecord(message) &&
    typeof message.method === "string" &&
    (!("id" in message) || typeof message.id !== "number")
  );
}

function toJsonError(method: string, error: unknown): Error {
  if (error instanceof Error) {
    return error;
  }
  if (isRecord(error)) {
    const message = asString(error.message) ?? `Codex request failed: ${method}`;
    const err = new Error(message);
    if (typeof error.code === "number") {
      (err as { code?: number }).code = error.code;
    }
    return err;
  }
  return new Error(`Codex request failed: ${method}`);
}

function normalizeCodexConfig(cfg?: OpenClawConfig): CodexEngineConfig {
  const configured = cfg?.agents?.defaults?.codex;
  const backendEnv = cfg?.agents?.defaults?.cliBackends?.codex?.env;
  const rawArgs = Array.isArray(configured?.args) ? configured.args.filter(Boolean) : [];
  const args =
    rawArgs.length > 0 ? rawArgs : ["app-server", "--listen", configured?.listen ?? "stdio://"];
  return {
    command: configured?.command?.trim() || "codex",
    args,
    env:
      backendEnv && typeof backendEnv === "object"
        ? Object.fromEntries(
            Object.entries(backendEnv)
              .filter(([, value]) => typeof value === "string" && value.length > 0)
              .map(([key, value]) => [key, value]),
          )
        : undefined,
    defaultModel: configured?.defaultModel?.trim() || "gpt-5.4",
    modelProvider: configured?.provider?.trim() || "openai",
    approvalPolicy: configured?.approvalPolicy ?? "never",
    sandbox: configured?.sandbox ?? "workspace-write",
    minimumVersion: configured?.minimumVersion?.trim() || undefined,
    experimentalApi: configured?.experimentalApi !== false,
  };
}

function maybeToolSchema(tool: AgentTool): Record<string, unknown> {
  if (tool.parameters && typeof tool.parameters === "object" && !Array.isArray(tool.parameters)) {
    return tool.parameters as Record<string, unknown>;
  }
  return {
    type: "object",
    additionalProperties: true,
  };
}

function buildCodexToolName(name: string): string {
  return `openclaw_${name.replace(/[^a-zA-Z0-9_]+/g, "_")}`;
}

function serializeToolResultText(result: AgentToolResult<unknown>): string {
  if (Array.isArray(result.content)) {
    const text = result.content.filter(isTextToolContent).map((entry) => entry.text).join("\n").trim();
    if (text) {
      return text;
    }
  }
  if (result.details !== undefined) {
    try {
      return JSON.stringify(result.details, null, 2);
    } catch {
      /* ignore */
    }
  }
  return "";
}

function toCodexToolResultItems(result: AgentToolResult<unknown>): CodexDynamicToolOutputContentItem[] {
  const items: CodexDynamicToolOutputContentItem[] = [];
  if (Array.isArray(result.content)) {
    for (const entry of result.content) {
      if (isTextToolContent(entry) && entry.text.trim()) {
        items.push({ type: "inputText", text: entry.text });
        continue;
      }
      if (isImageToolContent(entry) && entry.data.trim()) {
        items.push({
          type: "inputImage",
          imageUrl: `data:${entry.mimeType};base64,${entry.data}`,
        });
      }
    }
  }
  if (items.length === 0) {
    const fallbackText = serializeToolResultText(result);
    items.push({
      type: "inputText",
      text: fallbackText || "Tool completed successfully.",
    });
  }
  return items;
}

function buildDynamicTools(
  tools: AgentTool[],
): {
  specs: CodexDynamicToolSpec[];
  byName: Map<string, AgentTool>;
} {
  const byName = new Map<string, AgentTool>();
  const specs = tools.map((tool) => {
    const name = buildCodexToolName(tool.name);
    byName.set(name, tool);
    return {
      name,
      description: tool.description?.trim() || tool.label?.trim() || tool.name,
      inputSchema: maybeToolSchema(tool),
    };
  });
  return { specs, byName };
}

class CodexEngine {
  private readonly child;
  private readonly lineReader;
  private readonly pending = new Map<JsonRpcId, PendingRequest>();
  private readonly requestHandlers = new Map<string, (message: JsonRpcRequest) => Promise<void>>();
  private readonly notificationHandlers = new Map<string, Array<(params: unknown) => void>>();
  private nextId = 1;
  private closed = false;

  constructor(
    private readonly runtime: CodexEngineConfig,
    private readonly workspaceDir: string,
  ) {
    this.child = spawn(runtime.command, runtime.args, {
      cwd: workspaceDir,
      stdio: ["pipe", "pipe", "pipe"],
      env: {
        ...process.env,
        ...(runtime.env ?? {}),
      },
    });
    this.lineReader = readline.createInterface({ input: this.child.stdout });
    this.lineReader.on("line", (line) => {
      void this.handleLine(line);
    });
    this.child.stderr.on("data", (chunk: Buffer | string) => {
      const text = String(chunk ?? "").trim();
      if (text) {
        log.debug(`[codex-stderr] ${text}`);
      }
    });
    this.child.once("exit", (code, signal) => {
      this.closed = true;
      const err = new Error(`Codex app-server exited (code=${code ?? "null"} signal=${signal ?? "null"})`);
      for (const pending of this.pending.values()) {
        pending.reject(err);
      }
      this.pending.clear();
    });
  }

  async initialize() {
    const result = await this.request("initialize", {
      clientInfo: {
        name: "openclaw",
        title: "OpenClaw",
        version: "2026.3.9",
      },
      capabilities: {
        experimentalApi: this.runtime.experimentalApi,
        optOutNotificationMethods: [],
      },
    });
    await this.notify("initialized", {});
    return result;
  }

  onNotification(method: string, handler: (params: unknown) => void) {
    const existing = this.notificationHandlers.get(method) ?? [];
    existing.push(handler);
    this.notificationHandlers.set(method, existing);
  }

  onRequest(method: string, handler: (message: JsonRpcRequest) => Promise<void>) {
    this.requestHandlers.set(method, handler);
  }

  async request(method: string, params?: unknown): Promise<unknown> {
    const id = this.nextId++;
    const payload: JsonRpcRequest = { id, method, ...(params !== undefined ? { params } : {}) };
    const promise = new Promise<unknown>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
    this.writeMessage(payload);
    return await promise;
  }

  async notify(method: string, params?: unknown) {
    this.writeMessage({ method, ...(params !== undefined ? { params } : {}) });
  }

  respondResult(id: JsonRpcId, result: unknown) {
    this.writeMessage({ id, result });
  }

  respondError(id: JsonRpcId, error: { code: number; message: string; data?: unknown }) {
    this.writeMessage({ id, error });
  }

  async close() {
    if (this.closed) {
      return;
    }
    this.closed = true;
    this.lineReader.close();
    this.child.kill();
    await once(this.child, "exit").catch(() => undefined);
  }

  private writeMessage(payload: Record<string, unknown>) {
    if (this.closed || !this.child.stdin.writable) {
      throw new Error("Codex app-server is not writable.");
    }
    this.child.stdin.write(`${JSON.stringify(payload)}\n`);
  }

  private async handleLine(rawLine: string) {
    const line = rawLine.trim();
    if (!line) {
      return;
    }
    let message: unknown;
    try {
      message = JSON.parse(line);
    } catch (error) {
      log.warn(`Failed to parse Codex JSON-RPC message: ${String(error)}`);
      return;
    }
    if (isJsonRpcSuccessResponse(message)) {
      const pending = this.pending.get(message.id);
      if (!pending) {
        return;
      }
      this.pending.delete(message.id);
      pending.resolve(message.result);
      return;
    }
    if (isJsonRpcErrorResponse(message)) {
      const pending = this.pending.get(message.id);
      if (!pending) {
        return;
      }
      this.pending.delete(message.id);
      pending.reject(toJsonError("unknown", message.error));
      return;
    }
    if (isJsonRpcRequest(message)) {
      const handler = this.requestHandlers.get(message.method);
      if (!handler) {
        this.respondError(message.id, {
          code: -32601,
          message: `Unhandled Codex server request: ${message.method}`,
        });
        return;
      }
      try {
        await handler(message);
      } catch (error) {
        this.respondError(message.id, {
          code: -32000,
          message: error instanceof Error ? error.message : String(error),
        });
      }
      return;
    }
    if (isJsonRpcNotification(message)) {
      for (const handler of this.notificationHandlers.get(message.method) ?? []) {
        try {
          handler(message.params);
        } catch (error) {
          log.warn(`Codex notification handler failed (${message.method}): ${String(error)}`);
        }
      }
    }
  }
}

function normalizeToolList(tools: AgentTool[], senderIsOwner: boolean): AgentTool[] {
  return tools.map((tool) => wrapOwnerOnlyToolExecution(tool, senderIsOwner));
}

function extractThreadStatus(payload: unknown): string | undefined {
  if (!isRecord(payload)) {
    return undefined;
  }
  const thread = isRecord(payload.thread) ? payload.thread : payload;
  return asString(thread.status);
}

function extractThreadId(payload: unknown): string | undefined {
  if (!isRecord(payload)) {
    return undefined;
  }
  const thread = isRecord(payload.thread) ? payload.thread : payload;
  return asString(thread.id);
}

function extractTurnId(payload: unknown): string | undefined {
  if (!isRecord(payload)) {
    return undefined;
  }
  const turn = isRecord(payload.turn) ? payload.turn : payload;
  return asString(turn.id);
}

function extractTurnStatus(payload: unknown): string | undefined {
  if (!isRecord(payload)) {
    return undefined;
  }
  const turn = isRecord(payload.turn) ? payload.turn : payload;
  return asString(turn.status);
}

function extractItemRecord(payload: unknown): Record<string, unknown> | undefined {
  if (!isRecord(payload)) {
    return undefined;
  }
  return isRecord(payload.item) ? payload.item : payload;
}

export function buildCodexCompactionEvent(
  method: string,
  payload: unknown,
): Record<string, unknown> | null {
  const payloadRecord = isRecord(payload) ? payload : {};
  const topLevelThreadId = asString(payloadRecord.threadId);
  const topLevelTurnId = asString(payloadRecord.turnId);
  if (method === "context/compacted") {
    const data = payloadRecord;
    return {
      phase: "end",
      threadId: topLevelThreadId ?? extractThreadId(payload),
      previousTokens: typeof data.previousTokens === "number" ? data.previousTokens : undefined,
      newTokens: typeof data.newTokens === "number" ? data.newTokens : undefined,
    };
  }
  if (method !== "item/started" && method !== "item/completed") {
    return null;
  }
  const item = extractItemRecord(payload);
  if (asString(item?.type) !== "contextCompaction") {
    return null;
  }
  return {
    phase: method === "item/started" ? "start" : "end",
    threadId: topLevelThreadId ?? extractThreadId(payload),
    turnId: topLevelTurnId ?? extractTurnId(payload),
    itemId: asString(item?.id),
    itemType: "contextCompaction",
  };
}

function isNotFoundError(error: unknown): boolean {
  const message = error instanceof Error ? error.message.toLowerCase() : String(error).toLowerCase();
  return message.includes("not found") || message.includes("unknown thread");
}

function emitCodexAgentEvent(
  params: CodexAgentParams,
  event: { stream: string; data?: Record<string, unknown> },
) {
  emitAgentEvent({
    runId: params.runId,
    stream: event.stream,
    sessionKey: params.sessionKey,
    data: event.data ?? {},
  });
  params.onAgentEvent?.(event);
}

function unsupportedServerRequestMessage(method: CodexServerRequestMethod): string {
  switch (method) {
    case "item/commandExecution/requestApproval":
      return "OpenClaw does not yet provide an interactive Codex command-approval bridge for this session.";
    case "item/fileChange/requestApproval":
      return "OpenClaw does not yet provide an interactive Codex file-change approval bridge for this session.";
    case "item/permissions/requestApproval":
      return "OpenClaw does not yet provide an interactive Codex permission-grant bridge for this session.";
    case "item/tool/requestUserInput":
      return "OpenClaw does not yet provide an interactive Codex request_user_input bridge for this session.";
    case "mcpServer/elicitation/request":
      return "OpenClaw does not yet provide an interactive Codex MCP elicitation bridge for this session.";
    case "item/tool/call":
      return "OpenClaw tool execution failed.";
  }
}

function buildUnsupportedServerRequestResponse(method: CodexServerRequestMethod): unknown {
  switch (method) {
    case "item/commandExecution/requestApproval":
      return { decision: "decline" };
    case "item/fileChange/requestApproval":
      return { decision: "decline" };
    case "item/permissions/requestApproval":
      return { permissions: {}, scope: "turn" };
    case "item/tool/requestUserInput":
      return { answers: {} };
    case "mcpServer/elicitation/request":
      return { action: "decline", content: null };
    case "item/tool/call":
      return {
        contentItems: [{ type: "inputText", text: unsupportedServerRequestMessage(method) }],
        success: false,
      };
  }
}

function resolveOperatorRequestKind(method: CodexServerRequestMethod): OperatorRequestKind | null {
  switch (method) {
    case "item/commandExecution/requestApproval":
      return "codex_command_approval";
    case "item/fileChange/requestApproval":
      return "codex_file_change_approval";
    case "item/permissions/requestApproval":
      return "codex_permissions_approval";
    case "item/tool/requestUserInput":
      return "codex_tool_input";
    case "mcpServer/elicitation/request":
      return "codex_mcp_elicitation";
    case "item/tool/call":
      return null;
  }
}

function normalizeInteractiveServerRequestResponse(
  method: CodexServerRequestMethod,
  resolution: unknown,
): unknown {
  switch (method) {
    case "item/commandExecution/requestApproval":
      if (isRecord(resolution) && "decision" in resolution) {
        return resolution;
      }
      if (typeof resolution === "string" || isRecord(resolution)) {
        return { decision: resolution };
      }
      return buildUnsupportedServerRequestResponse(method);
    case "item/fileChange/requestApproval":
      if (isRecord(resolution) && "decision" in resolution) {
        return resolution;
      }
      if (typeof resolution === "string") {
        return { decision: resolution };
      }
      return buildUnsupportedServerRequestResponse(method);
    case "item/permissions/requestApproval":
      if (isRecord(resolution) && "permissions" in resolution && "scope" in resolution) {
        return resolution;
      }
      return buildUnsupportedServerRequestResponse(method);
    case "item/tool/requestUserInput":
      if (isRecord(resolution) && "answers" in resolution) {
        return resolution;
      }
      return buildUnsupportedServerRequestResponse(method);
    case "mcpServer/elicitation/request":
      if (isRecord(resolution) && "action" in resolution) {
        return {
          ...resolution,
          content: "content" in resolution ? resolution.content : null,
          _meta: "_meta" in resolution ? resolution._meta : null,
        };
      }
      return buildUnsupportedServerRequestResponse(method);
    case "item/tool/call":
      return resolution;
  }
}

export async function runCodexAgent(params: CodexAgentParams): Promise<EmbeddedPiRunResult> {
  const started = Date.now();
  const runtime = normalizeCodexConfig(params.config);
  const codex = new CodexEngine(runtime, params.workspaceDir);
  const toolCleanup: Array<() => Promise<void>> = [];
  let finalizedText = "";
  let threadMeta: CodexThreadMeta | undefined;
  let systemPromptReport: SessionSystemPromptReport | undefined;

  const tools = normalizeToolList(
    createOpenClawTools({
      config: params.config,
      agentSessionKey: params.sessionKey,
      agentChannel: params.messageChannel as never,
      agentAccountId: params.agentAccountId,
      agentTo: params.messageTo,
      agentThreadId: params.messageThreadId,
      agentDir: params.agentDir,
      workspaceDir: params.workspaceDir,
      currentChannelId: params.currentChannelId,
      currentThreadTs: params.currentThreadTs,
      currentMessageId: params.currentMessageId,
      replyToMode: params.replyToMode,
      hasRepliedRef: params.hasRepliedRef,
      requesterAgentIdOverride: params.agentId,
      requesterSenderId: params.requesterSenderId ?? undefined,
      senderIsOwner: params.senderIsOwner,
      sessionId: params.sessionId,
      agentGroupId: params.groupId,
      agentGroupChannel: params.groupChannel,
      agentGroupSpace: params.groupSpace,
    }),
    params.senderIsOwner ?? false,
  );
  const { specs: dynamicTools, byName: toolsByName } = buildDynamicTools(tools);

  const resolveInteractiveServerRequest = async (
    method: CodexServerRequestMethod,
    requestId: JsonRpcId,
    requestPayload: unknown,
  ) => {
    const payload = isRecord(requestPayload)
      ? { ...requestPayload, requestId: String(requestId) }
      : { requestId: String(requestId) };
    const kind = resolveOperatorRequestKind(method);
    emitCodexAgentEvent(params, {
      stream: method.includes("Approval") ? "approval" : "server_request",
      data: {
        phase: "requested",
        type: method,
        requestId,
        payload,
      },
    });
    const interactive =
      kind === null
        ? null
        : await requestOperatorResolution({
            kind,
            method,
            payload,
            sessionKey: params.sessionKey ?? null,
            runId: params.runId,
            timeoutMs: DEFAULT_OPERATOR_REQUEST_TIMEOUT_MS,
          });
    const normalizedResponse = interactive
      ? normalizeInteractiveServerRequestResponse(method, interactive.resolution)
      : buildUnsupportedServerRequestResponse(method);
    emitCodexAgentEvent(params, {
      stream: method.includes("Approval") ? "approval" : "server_request",
      data: {
        phase: "resolved",
        type: method,
        requestId,
        operatorRequestId: interactive?.record.id,
        autoResolved: !interactive || interactive.resolution == null,
        resolution: interactive?.resolution ?? normalizedResponse,
        message:
          interactive?.resolution == null ? unsupportedServerRequestMessage(method) : undefined,
        payload,
      },
    });
    codex.respondResult(requestId, normalizedResponse);
  };

  codex.onRequest("item/tool/call", async (request) => {
    const toolRequest = request.params as CodexToolCallParams;
    const tool = toolsByName.get(toolRequest.tool);
    if (!tool?.execute) {
      codex.respondResult(request.id, {
        contentItems: [{ type: "inputText", text: `Unknown OpenClaw tool: ${toolRequest.tool}` }],
        success: false,
      });
      return;
    }
    emitCodexAgentEvent(params, {
      stream: "tool",
      data: {
        phase: "start",
        name: toolRequest.tool,
        toolCallId: toolRequest.callId,
        args: isRecord(toolRequest.arguments) ? toolRequest.arguments : {},
        meta: "codex-dynamic-tool",
        threadId: toolRequest.threadId,
        turnId: toolRequest.turnId,
      },
    });
    try {
      const result = await tool.execute(
        toolRequest.callId,
        isRecord(toolRequest.arguments) ? toolRequest.arguments : {},
        params.abortSignal,
      );
      codex.respondResult(request.id, {
        contentItems: toCodexToolResultItems(result),
        success: true,
      });
      emitCodexAgentEvent(params, {
        stream: "tool",
        data: {
          phase: "result",
          name: toolRequest.tool,
          toolCallId: toolRequest.callId,
          result,
          isError: false,
          meta: "codex-dynamic-tool",
          threadId: toolRequest.threadId,
          turnId: toolRequest.turnId,
        },
      });
    } catch (error) {
      codex.respondResult(request.id, {
        contentItems: [
          {
            type: "inputText",
            text: error instanceof Error ? error.message : String(error),
          },
        ],
        success: false,
      });
      emitCodexAgentEvent(params, {
        stream: "tool",
        data: {
          phase: "result",
          name: toolRequest.tool,
          toolCallId: toolRequest.callId,
          isError: true,
          error: error instanceof Error ? error.message : String(error),
          meta: "codex-dynamic-tool",
          threadId: toolRequest.threadId,
          turnId: toolRequest.turnId,
        },
      });
    }
  });
  codex.onRequest("item/commandExecution/requestApproval", async (request) => {
    await resolveInteractiveServerRequest(
      "item/commandExecution/requestApproval",
      request.id,
      request.params,
    );
  });
  codex.onRequest("item/fileChange/requestApproval", async (request) => {
    await resolveInteractiveServerRequest("item/fileChange/requestApproval", request.id, request.params);
  });
  codex.onRequest("item/permissions/requestApproval", async (request) => {
    await resolveInteractiveServerRequest(
      "item/permissions/requestApproval",
      request.id,
      request.params,
    );
  });
  codex.onRequest("item/tool/requestUserInput", async (request) => {
    await resolveInteractiveServerRequest("item/tool/requestUserInput", request.id, request.params);
  });
  codex.onRequest("mcpServer/elicitation/request", async (request) => {
    await resolveInteractiveServerRequest("mcpServer/elicitation/request", request.id, request.params);
  });

  const handleNotification = (method: string, stream: string, extra?: (params: unknown) => Record<string, unknown>) => {
    codex.onNotification(method, (payload) => {
      emitCodexAgentEvent(params, {
        stream,
        data: {
          type: method,
          ...(extra ? extra(payload) : {}),
        },
      });
    });
  };

  handleNotification("thread/started", "lifecycle", (payload) => ({
    threadId: extractThreadId(payload),
    threadStatus: extractThreadStatus(payload),
  }));
  handleNotification("turn/started", "lifecycle", (payload) => ({
    turnId: extractTurnId(payload),
    turnStatus: extractTurnStatus(payload),
  }));
  handleNotification("item/started", "item", (payload) => ({
    item: isRecord(payload) ? payload.item ?? payload : payload,
  }));
  handleNotification("item/completed", "item", (payload) => ({
    item: isRecord(payload) ? payload.item ?? payload : payload,
  }));
  handleNotification("turn/plan/updated", "plan", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("model/rerouted", "lifecycle", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("configWarning", "lifecycle", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("deprecationNotice", "lifecycle", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("item/plan/delta", "plan", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("item/reasoning/summaryTextDelta", "reasoning", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("item/reasoning/summaryPartAdded", "reasoning", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("item/reasoning/textDelta", "reasoning", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("turn/diff/updated", "diff", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("command/exec/outputDelta", "command", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("item/commandExecution/outputDelta", "command", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("item/commandExecution/terminalInteraction", "command", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("item/fileChange/outputDelta", "diff", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("skills/changed", "lifecycle", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("serverRequest/resolved", "approval", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  handleNotification("error", "error", (payload) => ({
    payload: isRecord(payload) ? payload : {},
  }));
  codex.onNotification("item/started", (payload) => {
    const compaction = buildCodexCompactionEvent("item/started", payload);
    if (!compaction) {
      return;
    }
    emitCodexAgentEvent(params, {
      stream: "compaction",
      data: compaction,
    });
  });
  codex.onNotification("item/completed", (payload) => {
    const compaction = buildCodexCompactionEvent("item/completed", payload);
    if (!compaction) {
      return;
    }
    emitCodexAgentEvent(params, {
      stream: "compaction",
      data: compaction,
    });
  });
  codex.onNotification("context/compacted", (payload) => {
    const compaction = buildCodexCompactionEvent("context/compacted", payload);
    if (!compaction) {
      return;
    }
    emitCodexAgentEvent(params, {
      stream: "compaction",
      data: compaction,
    });
  });
  codex.onNotification("item/agentMessage/delta", (payload) => {
    const delta = isRecord(payload) ? asString(payload.delta) ?? "" : "";
    if (!delta) {
      return;
    }
    finalizedText += delta;
    emitCodexAgentEvent(params, {
      stream: "assistant",
      data: {
        type: "item/agentMessage/delta",
        delta,
        text: finalizedText,
      },
    });
  });

  try {
    await codex.initialize();
    await codex.request("skills/list", {
      cwd: params.workspaceDir,
      forceReload: false,
    });

    const bootstrapMaxChars = resolveBootstrapMaxChars(params.config);
    const bootstrapTotalMaxChars = resolveBootstrapTotalMaxChars(params.config);
    const { bootstrapFiles, contextFiles } = await resolveBootstrapContextForRun({
      workspaceDir: params.workspaceDir,
      config: params.config,
      sessionKey: params.sessionKey,
      sessionId: params.sessionId,
      warn: makeBootstrapWarn({
        sessionLabel: params.sessionKey ?? params.sessionId,
        warn: (message) => log.warn(message),
      }),
    });
    const bootstrapAnalysis = analyzeBootstrapBudget({
      files: buildBootstrapInjectionStats({
        bootstrapFiles,
        injectedFiles: contextFiles,
      }),
      bootstrapMaxChars,
      bootstrapTotalMaxChars,
    });
    const bootstrapPromptWarningMode = resolveBootstrapPromptTruncationWarningMode(params.config);
    const bootstrapPromptWarning = buildBootstrapPromptWarning({
      analysis: bootstrapAnalysis,
      mode: bootstrapPromptWarningMode,
      seenSignatures: params.bootstrapPromptWarningSignaturesSeen,
      previousSignature: params.bootstrapPromptWarningSignature,
    });
    const { defaultAgentId, sessionAgentId } = resolveSessionAgentIds({
      sessionKey: params.sessionKey,
      config: params.config,
      agentId: params.agentId,
    });
    const heartbeatPrompt =
      sessionAgentId === defaultAgentId
        ? resolveHeartbeatPrompt(params.config?.agents?.defaults?.heartbeat?.prompt)
        : undefined;
    const docsPath = await resolveOpenClawDocsPath({
      workspaceDir: params.workspaceDir,
      argv1: process.argv[1],
      cwd: process.cwd(),
      moduleUrl: import.meta.url,
    });
    const modelProvider = params.provider?.trim() || runtime.modelProvider;
    const model = params.model?.trim() || runtime.defaultModel;
    const modelDisplay = `${modelProvider}/${model}`;
    const systemPrompt = buildSystemPrompt({
      workspaceDir: params.workspaceDir,
      config: params.config,
      defaultThinkLevel: params.thinkLevel,
      extraSystemPrompt: params.extraSystemPrompt,
      ownerNumbers: params.ownerNumbers,
      heartbeatPrompt,
      docsPath: docsPath ?? undefined,
      tools,
      contextFiles,
      bootstrapTruncationWarningLines: bootstrapPromptWarning.lines,
      modelDisplay,
      agentId: sessionAgentId,
    });
    systemPromptReport = buildSystemPromptReport({
      source: "run",
      generatedAt: Date.now(),
      sessionId: params.sessionId,
      sessionKey: params.sessionKey,
      provider: modelProvider,
      model,
      workspaceDir: params.workspaceDir,
      bootstrapMaxChars,
      bootstrapTotalMaxChars,
      bootstrapTruncation: buildBootstrapTruncationReportMeta({
        analysis: bootstrapAnalysis,
        warningMode: bootstrapPromptWarningMode,
        warning: bootstrapPromptWarning,
      }),
      sandbox: { mode: runtime.sandbox, sandboxed: runtime.sandbox !== "danger-full-access" },
      systemPrompt,
      bootstrapFiles,
      injectedFiles: contextFiles,
      skillsPrompt: "",
      tools,
    });

    let imageInputs: Array<{ type: "localImage"; path: string }> = [];
    if (params.images?.length) {
      const imagePayload = await writeCliImages(params.images);
      imageInputs = imagePayload.paths.map((filePath) => ({ type: "localImage" as const, path: filePath }));
      toolCleanup.push(imagePayload.cleanup);
    }

    let threadResponse: unknown;
    if (params.sessionEngine?.threadId) {
      try {
        threadResponse = await codex.request("thread/resume", {
          threadId: params.sessionEngine.threadId,
          model,
          modelProvider,
          cwd: params.workspaceDir,
          approvalPolicy: runtime.approvalPolicy,
          sandbox: runtime.sandbox,
          baseInstructions: systemPrompt,
          persistExtendedHistory: true,
        });
      } catch (error) {
        if (isNotFoundError(error)) {
          throw new FailoverError(`Stored Codex thread no longer exists: ${params.sessionEngine.threadId}`, {
            reason: "session_expired",
            provider: "codex-cli",
            model,
            cause: error,
          });
        }
        throw error;
      }
    } else {
      threadResponse = await codex.request("thread/start", {
        model,
        modelProvider,
        cwd: params.workspaceDir,
        approvalPolicy: runtime.approvalPolicy,
        sandbox: runtime.sandbox,
        baseInstructions: systemPrompt,
        dynamicTools,
        persistExtendedHistory: true,
      });
    }

    const threadId = extractThreadId(threadResponse);
    if (!threadId) {
      throw new Error("Codex app-server did not return a thread id.");
    }
    threadMeta = {
      kind: "codex",
      threadId,
      threadStatus: extractThreadStatus(threadResponse),
      runtimeOrigin: "codex app-server",
      protocolVersion: "v2",
      compatibilityVersion: runtime.minimumVersion,
    };

    const turnStart = await codex.request("turn/start", {
      threadId,
      input: [{ type: "text", text: params.prompt, text_elements: [] }, ...imageInputs],
      model,
    });
    threadMeta.lastTurnId = extractTurnId(turnStart);

    const completed = await new Promise<unknown>((resolve, reject) => {
      let finished = false;
      codex.onNotification("turn/completed", (payload) => {
        const payloadTurnId =
          isRecord(payload) && isRecord(payload.turn) ? asString(payload.turn.id) : undefined;
        if (threadMeta?.lastTurnId && payloadTurnId && payloadTurnId !== threadMeta.lastTurnId) {
          return;
        }
        if (!finished) {
          finished = true;
          resolve(payload);
        }
      });
      if (params.abortSignal) {
        params.abortSignal.addEventListener(
          "abort",
          () => {
            if (finished || !threadMeta?.lastTurnId) {
              return;
            }
            finished = true;
            void codex
              .request("turn/interrupt", {
                threadId,
                turnId: threadMeta.lastTurnId,
              })
              .then(() =>
                reject(
                  new FailoverError("Codex turn interrupted.", {
                    reason: "timeout",
                    provider: "codex-cli",
                    model,
                  }),
                ),
              )
              .catch(reject);
          },
          { once: true },
        );
      }
      setTimeout(() => {
        if (finished) {
          return;
        }
        finished = true;
        reject(
          new FailoverError("Codex turn timed out.", {
            reason: "timeout",
            provider: "codex-cli",
            model,
          }),
        );
      }, params.timeoutMs);
    });

    threadMeta.threadStatus =
      extractTurnStatus(completed) ?? extractThreadStatus(completed) ?? threadMeta.threadStatus;
    const text = finalizedText.trim();
    return {
      payloads: text ? [{ text }] : [],
      meta: {
        durationMs: Date.now() - started,
        agentMeta: {
          sessionId: params.sessionId,
          provider: "codex-cli",
          model,
          engine: threadMeta,
        },
        aborted: false,
        systemPromptReport,
      },
    };
  } finally {
    for (const cleanup of toolCleanup) {
      await cleanup().catch(() => undefined);
    }
    await codex.close().catch(() => undefined);
  }
}
