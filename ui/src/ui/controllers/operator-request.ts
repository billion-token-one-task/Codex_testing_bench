export type OperatorRequestKind =
  | "codex_command_approval"
  | "codex_file_change_approval"
  | "codex_permissions_approval"
  | "codex_tool_input"
  | "codex_mcp_elicitation";

export type OperatorRequestPayload = {
  kind: OperatorRequestKind;
  method: string;
  requestId?: string | null;
  sessionKey?: string | null;
  runId?: string | null;
  threadId?: string | null;
  turnId?: string | null;
  itemId?: string | null;
  payload: Record<string, unknown>;
};

export type OperatorRequest = {
  id: string;
  request: OperatorRequestPayload;
  createdAtMs: number;
  expiresAtMs: number;
  resolvedAtMs?: number | null;
  resolvedBy?: string | null;
  resolution?: unknown;
};

export type OperatorToolQuestion = {
  id: string;
  header: string;
  question: string;
  isOther: boolean;
  isSecret: boolean;
  options: Array<{ label: string; description: string }>;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isOperatorRequestKind(value: unknown): value is OperatorRequestKind {
  return (
    value === "codex_command_approval" ||
    value === "codex_file_change_approval" ||
    value === "codex_permissions_approval" ||
    value === "codex_tool_input" ||
    value === "codex_mcp_elicitation"
  );
}

function safeParseDraftObject(entry: OperatorRequest, draft: string): Record<string, unknown> {
  const trimmed = draft.trim();
  if (!trimmed) {
    return parseOperatorResolutionDraft(buildDefaultOperatorResolutionDraft(entry)) as Record<
      string,
      unknown
    >;
  }
  try {
    const parsed = parseOperatorResolutionDraft(trimmed);
    return isRecord(parsed) ? parsed : {};
  } catch {
    return parseOperatorResolutionDraft(buildDefaultOperatorResolutionDraft(entry)) as Record<
      string,
      unknown
    >;
  }
}

function toPrettyJson(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

export function parseOperatorRequest(payload: unknown): OperatorRequest | null {
  if (!isRecord(payload)) {
    return null;
  }
  const id = typeof payload.id === "string" ? payload.id.trim() : "";
  const request = payload.request;
  const createdAtMs = typeof payload.createdAtMs === "number" ? payload.createdAtMs : 0;
  const expiresAtMs = typeof payload.expiresAtMs === "number" ? payload.expiresAtMs : 0;
  if (!id || !isRecord(request) || !createdAtMs || !expiresAtMs) {
    return null;
  }
  const kind = request.kind;
  const method = typeof request.method === "string" ? request.method.trim() : "";
  const requestPayload = request.payload;
  if (!isOperatorRequestKind(kind) || !method || !isRecord(requestPayload)) {
    return null;
  }
  return {
    id,
    request: {
      kind,
      method,
      requestId: typeof request.requestId === "string" ? request.requestId : null,
      sessionKey: typeof request.sessionKey === "string" ? request.sessionKey : null,
      runId: typeof request.runId === "string" ? request.runId : null,
      threadId: typeof request.threadId === "string" ? request.threadId : null,
      turnId: typeof request.turnId === "string" ? request.turnId : null,
      itemId: typeof request.itemId === "string" ? request.itemId : null,
      payload: requestPayload,
    },
    createdAtMs,
    expiresAtMs,
    resolvedAtMs: typeof payload.resolvedAtMs === "number" ? payload.resolvedAtMs : null,
    resolvedBy: typeof payload.resolvedBy === "string" ? payload.resolvedBy : null,
    resolution: "resolution" in payload ? payload.resolution : undefined,
  };
}

export function pruneOperatorRequestQueue(queue: OperatorRequest[]): OperatorRequest[] {
  const now = Date.now();
  return queue.filter((entry) => entry.expiresAtMs > now);
}

export function addOperatorRequest(
  queue: OperatorRequest[],
  entry: OperatorRequest,
): OperatorRequest[] {
  const next = pruneOperatorRequestQueue(queue).filter((item) => item.id !== entry.id);
  next.push(entry);
  next.sort((a, b) => a.createdAtMs - b.createdAtMs);
  return next;
}

export function removeOperatorRequest(queue: OperatorRequest[], id: string): OperatorRequest[] {
  return pruneOperatorRequestQueue(queue).filter((entry) => entry.id !== id);
}

export function isOperatorApprovalRequest(entry: OperatorRequest): boolean {
  return (
    entry.request.kind === "codex_command_approval" ||
    entry.request.kind === "codex_file_change_approval"
  );
}

export function formatOperatorRequestTitle(entry: OperatorRequest): string {
  switch (entry.request.kind) {
    case "codex_command_approval":
      return "Command approval needed";
    case "codex_file_change_approval":
      return "File change approval needed";
    case "codex_permissions_approval":
      return "Permission grant needed";
    case "codex_tool_input":
      return "Tool input needed";
    case "codex_mcp_elicitation":
      return "MCP input needed";
  }
}

export function buildOperatorDecisionResolution(
  entry: OperatorRequest,
  decision: "allow-once" | "allow-always" | "deny",
): unknown {
  switch (entry.request.kind) {
    case "codex_command_approval":
      return {
        decision:
          decision === "allow-once"
            ? "accept"
            : decision === "allow-always"
              ? "acceptForSession"
              : "decline",
      };
    case "codex_file_change_approval":
      return {
        decision:
          decision === "allow-once"
            ? "accept"
            : decision === "allow-always"
              ? "acceptForSession"
              : "decline",
      };
    case "codex_permissions_approval":
    case "codex_tool_input":
    case "codex_mcp_elicitation":
      return decision;
  }
}

export function buildDefaultOperatorResolutionDraft(entry: OperatorRequest): string {
  const payload = entry.request.payload;
  switch (entry.request.kind) {
    case "codex_command_approval":
      return JSON.stringify({ decision: "accept" }, null, 2);
    case "codex_file_change_approval":
      return JSON.stringify({ decision: "accept" }, null, 2);
    case "codex_permissions_approval":
      return JSON.stringify(
        {
          permissions: isRecord(payload.permissions) ? payload.permissions : {},
          scope: "turn",
        },
        null,
        2,
      );
    case "codex_tool_input": {
      const questions = Array.isArray(payload.questions) ? payload.questions : [];
      const answers = Object.fromEntries(
        questions
          .map((question) => (isRecord(question) ? question : null))
          .filter((question): question is Record<string, unknown> => Boolean(question))
          .map((question) => {
            const id = typeof question.id === "string" ? question.id : "";
            return [id, { answers: [""] }];
          })
          .filter(([id]) => id),
      );
      return JSON.stringify({ answers }, null, 2);
    }
    case "codex_mcp_elicitation":
      return JSON.stringify({ action: "accept", content: {}, _meta: null }, null, 2);
  }
}

export function parseOperatorResolutionDraft(draft: string): unknown {
  const trimmed = draft.trim();
  if (!trimmed) {
    throw new Error("Resolution JSON is required.");
  }
  return JSON.parse(trimmed);
}

export function getOperatorToolQuestions(entry: OperatorRequest): OperatorToolQuestion[] {
  if (entry.request.kind !== "codex_tool_input") {
    return [];
  }
  const rawQuestions = Array.isArray(entry.request.payload.questions)
    ? entry.request.payload.questions
    : [];
  return rawQuestions
    .map((question) => (isRecord(question) ? question : null))
    .filter((question): question is Record<string, unknown> => Boolean(question))
    .map((question) => ({
      id: typeof question.id === "string" ? question.id : "",
      header: typeof question.header === "string" ? question.header : "",
      question: typeof question.question === "string" ? question.question : "",
      isOther: question.isOther === true,
      isSecret: question.isSecret === true,
      options: Array.isArray(question.options)
        ? question.options
            .map((option) => (isRecord(option) ? option : null))
            .filter((option): option is Record<string, unknown> => Boolean(option))
            .map((option) => ({
              label: typeof option.label === "string" ? option.label : "",
              description: typeof option.description === "string" ? option.description : "",
            }))
            .filter((option) => option.label)
        : [],
    }))
    .filter((question) => question.id);
}

export function getOperatorToolAnswerValue(
  entry: OperatorRequest,
  draft: string,
  questionId: string,
): string {
  if (entry.request.kind !== "codex_tool_input") {
    return "";
  }
  const parsed = safeParseDraftObject(entry, draft);
  const answers = isRecord(parsed.answers) ? parsed.answers : {};
  const answer = isRecord(answers[questionId]) ? answers[questionId] : {};
  const list = Array.isArray(answer.answers) ? answer.answers : [];
  return typeof list[0] === "string" ? list[0] : "";
}

export function updateOperatorToolAnswerDraft(
  entry: OperatorRequest,
  draft: string,
  questionId: string,
  value: string,
): string {
  if (entry.request.kind !== "codex_tool_input") {
    return draft;
  }
  const parsed = safeParseDraftObject(entry, draft);
  const answers = isRecord(parsed.answers) ? { ...parsed.answers } : {};
  answers[questionId] = { answers: [value] };
  return toPrettyJson({
    ...parsed,
    answers,
  });
}

export function getOperatorPermissionScope(entry: OperatorRequest, draft: string): "turn" | "session" {
  if (entry.request.kind !== "codex_permissions_approval") {
    return "turn";
  }
  const parsed = safeParseDraftObject(entry, draft);
  return parsed.scope === "session" ? "session" : "turn";
}

export function getOperatorPermissionNetworkEnabled(entry: OperatorRequest, draft: string): boolean {
  if (entry.request.kind !== "codex_permissions_approval") {
    return false;
  }
  const parsed = safeParseDraftObject(entry, draft);
  const permissions = isRecord(parsed.permissions) ? parsed.permissions : {};
  const network = isRecord(permissions.network) ? permissions.network : {};
  return network.enabled === true;
}

function getOperatorPermissionPathList(
  entry: OperatorRequest,
  draft: string,
  key: "read" | "write",
): string[] {
  if (entry.request.kind !== "codex_permissions_approval") {
    return [];
  }
  const parsed = safeParseDraftObject(entry, draft);
  const permissions = isRecord(parsed.permissions) ? parsed.permissions : {};
  const fileSystem = isRecord(permissions.fileSystem) ? permissions.fileSystem : {};
  const values = Array.isArray(fileSystem[key]) ? fileSystem[key] : [];
  return values.filter((value): value is string => typeof value === "string" && value.trim().length > 0);
}

export function getOperatorPermissionPaths(
  entry: OperatorRequest,
  draft: string,
  key: "read" | "write",
): string {
  return getOperatorPermissionPathList(entry, draft, key).join("\n");
}

export function updateOperatorPermissionDraft(
  entry: OperatorRequest,
  draft: string,
  updates: {
    scope?: "turn" | "session";
    networkEnabled?: boolean;
    readPaths?: string;
    writePaths?: string;
  },
): string {
  if (entry.request.kind !== "codex_permissions_approval") {
    return draft;
  }
  const parsed = safeParseDraftObject(entry, draft);
  const permissions = isRecord(parsed.permissions) ? { ...parsed.permissions } : {};
  const fileSystem = isRecord(permissions.fileSystem) ? { ...permissions.fileSystem } : {};
  const network = isRecord(permissions.network) ? { ...permissions.network } : {};
  if (updates.networkEnabled !== undefined) {
    network.enabled = updates.networkEnabled;
  }
  if (updates.readPaths !== undefined) {
    fileSystem.read = updates.readPaths
      .split(/\r?\n/)
      .map((value) => value.trim())
      .filter(Boolean);
  }
  if (updates.writePaths !== undefined) {
    fileSystem.write = updates.writePaths
      .split(/\r?\n/)
      .map((value) => value.trim())
      .filter(Boolean);
  }
  return toPrettyJson({
    ...parsed,
    permissions: {
      ...permissions,
      network,
      fileSystem,
    },
    scope: updates.scope ?? getOperatorPermissionScope(entry, draft),
  });
}

export function getOperatorMcpAction(
  entry: OperatorRequest,
  draft: string,
): "accept" | "decline" | "cancel" {
  if (entry.request.kind !== "codex_mcp_elicitation") {
    return "accept";
  }
  const parsed = safeParseDraftObject(entry, draft);
  return parsed.action === "decline" || parsed.action === "cancel" ? parsed.action : "accept";
}

export function getOperatorMcpContent(entry: OperatorRequest, draft: string): string {
  if (entry.request.kind !== "codex_mcp_elicitation") {
    return "";
  }
  const parsed = safeParseDraftObject(entry, draft);
  return toPrettyJson("content" in parsed ? parsed.content : {});
}

export function updateOperatorMcpDraft(
  entry: OperatorRequest,
  draft: string,
  updates: {
    action?: "accept" | "decline" | "cancel";
    content?: string;
  },
): string {
  if (entry.request.kind !== "codex_mcp_elicitation") {
    return draft;
  }
  const parsed = safeParseDraftObject(entry, draft);
  let nextContent = "content" in parsed ? parsed.content : {};
  if (updates.content !== undefined) {
    try {
      nextContent = updates.content.trim() ? JSON.parse(updates.content) : null;
    } catch {
      nextContent = updates.content;
    }
  }
  return toPrettyJson({
    ...parsed,
    action: updates.action ?? getOperatorMcpAction(entry, draft),
    content: nextContent,
    _meta: "_meta" in parsed ? parsed._meta : null,
  });
}
