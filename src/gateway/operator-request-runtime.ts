import { getPluginRuntimeGatewayRequestScope } from "../plugins/runtime/gateway-request-scope.js";
import { getFallbackGatewayContext } from "./server-plugins.js";
import type {
  OperatorRequestKind,
  OperatorRequestPayload,
  OperatorRequestRecord,
} from "./operator-request-manager.js";
import { DEFAULT_OPERATOR_REQUEST_TIMEOUT_MS } from "./operator-request-manager.js";

function resolveGatewayContext() {
  return getPluginRuntimeGatewayRequestScope()?.context ?? getFallbackGatewayContext();
}

function serializeOperatorRequestRecord(record: OperatorRequestRecord) {
  return {
    id: record.id,
    request: record.request,
    createdAtMs: record.createdAtMs,
    expiresAtMs: record.expiresAtMs,
    resolvedAtMs: record.resolvedAtMs,
    resolvedBy: record.resolvedBy ?? null,
  };
}

export async function requestOperatorResolution(params: {
  kind: OperatorRequestKind;
  method: string;
  payload: Record<string, unknown>;
  sessionKey?: string | null;
  runId?: string | null;
  timeoutMs?: number;
}): Promise<{ record: OperatorRequestRecord; resolution: unknown | null } | null> {
  const context = resolveGatewayContext();
  const manager = context?.operatorRequestManager;
  if (!context || !manager) {
    return null;
  }

  const request: OperatorRequestPayload = {
    kind: params.kind,
    method: params.method,
    requestId:
      typeof params.payload.requestId === "string" ? params.payload.requestId : undefined,
    sessionKey: params.sessionKey ?? null,
    runId: params.runId ?? null,
    threadId: typeof params.payload.threadId === "string" ? params.payload.threadId : null,
    turnId: typeof params.payload.turnId === "string" ? params.payload.turnId : null,
    itemId: typeof params.payload.itemId === "string" ? params.payload.itemId : null,
    payload: params.payload,
  };

  const timeoutMs =
    typeof params.timeoutMs === "number" && params.timeoutMs > 0
      ? params.timeoutMs
      : DEFAULT_OPERATOR_REQUEST_TIMEOUT_MS;
  const record = manager.create(request, timeoutMs);
  const decisionPromise = manager.register(record, timeoutMs);

  context.broadcast("operator.requested", serializeOperatorRequestRecord(record), {
    dropIfSlow: true,
  });

  if (!context.hasExecApprovalClients?.()) {
    manager.expire(record.id, "auto-expire:no-approver-clients");
  }

  const resolution = await decisionPromise;
  const snapshot = manager.getSnapshot(record.id) ?? record;
  context.broadcast(
    "operator.resolved",
    {
      ...serializeOperatorRequestRecord(snapshot),
      resolution,
    },
    { dropIfSlow: true },
  );

  return { record: snapshot, resolution };
}
