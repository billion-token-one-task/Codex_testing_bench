import { randomUUID } from "node:crypto";

const RESOLVED_ENTRY_GRACE_MS = 15_000;

export const DEFAULT_OPERATOR_REQUEST_TIMEOUT_MS = 120_000;

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

export type OperatorRequestRecord = {
  id: string;
  request: OperatorRequestPayload;
  createdAtMs: number;
  expiresAtMs: number;
  requestedByConnId?: string | null;
  requestedByDeviceId?: string | null;
  requestedByClientId?: string | null;
  resolvedAtMs?: number;
  resolution?: unknown;
  resolvedBy?: string | null;
};

type PendingEntry = {
  record: OperatorRequestRecord;
  resolve: (resolution: unknown | null) => void;
  timer: ReturnType<typeof setTimeout>;
  promise: Promise<unknown | null>;
};

export class OperatorRequestManager {
  private pending = new Map<string, PendingEntry>();

  create(
    request: OperatorRequestPayload,
    timeoutMs: number,
    id?: string | null,
  ): OperatorRequestRecord {
    const now = Date.now();
    const resolvedId = id && id.trim().length > 0 ? id.trim() : randomUUID();
    return {
      id: resolvedId,
      request,
      createdAtMs: now,
      expiresAtMs: now + timeoutMs,
    };
  }

  register(record: OperatorRequestRecord, timeoutMs: number): Promise<unknown | null> {
    const existing = this.pending.get(record.id);
    if (existing) {
      if (existing.record.resolvedAtMs === undefined) {
        return existing.promise;
      }
      throw new Error(`operator request id '${record.id}' already resolved`);
    }

    let resolvePromise!: (resolution: unknown | null) => void;
    const promise = new Promise<unknown | null>((resolve) => {
      resolvePromise = resolve;
    });

    const entry: PendingEntry = {
      record,
      resolve: resolvePromise,
      timer: null as unknown as ReturnType<typeof setTimeout>,
      promise,
    };

    entry.timer = setTimeout(() => {
      this.expire(record.id);
    }, timeoutMs);

    this.pending.set(record.id, entry);
    return promise;
  }

  resolve(recordId: string, resolution: unknown, resolvedBy?: string | null): boolean {
    const pending = this.pending.get(recordId);
    if (!pending || pending.record.resolvedAtMs !== undefined) {
      return false;
    }
    clearTimeout(pending.timer);
    pending.record.resolvedAtMs = Date.now();
    pending.record.resolution = resolution;
    pending.record.resolvedBy = resolvedBy ?? null;
    pending.resolve(resolution);
    setTimeout(() => {
      if (this.pending.get(recordId) === pending) {
        this.pending.delete(recordId);
      }
    }, RESOLVED_ENTRY_GRACE_MS);
    return true;
  }

  expire(recordId: string, resolvedBy?: string | null): boolean {
    const pending = this.pending.get(recordId);
    if (!pending || pending.record.resolvedAtMs !== undefined) {
      return false;
    }
    clearTimeout(pending.timer);
    pending.record.resolvedAtMs = Date.now();
    pending.record.resolution = undefined;
    pending.record.resolvedBy = resolvedBy ?? null;
    pending.resolve(null);
    setTimeout(() => {
      if (this.pending.get(recordId) === pending) {
        this.pending.delete(recordId);
      }
    }, RESOLVED_ENTRY_GRACE_MS);
    return true;
  }

  getSnapshot(recordId: string): OperatorRequestRecord | null {
    const entry = this.pending.get(recordId);
    return entry?.record ?? null;
  }

  awaitDecision(recordId: string): Promise<unknown | null> | null {
    const entry = this.pending.get(recordId);
    return entry?.promise ?? null;
  }
}
