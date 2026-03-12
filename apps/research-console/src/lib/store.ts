import { type DependencyList, useCallback, useEffect, useMemo, useRef, useState } from "react";

import { api } from "./api";
import type {
  ArtifactDescriptor,
  CampaignDetail,
  CampaignOperationalSummary,
  CampaignListItem,
  LiveOverviewResponse,
  LiveEventBucket,
  LiveRunSnapshot,
  ManagedProcessSnapshot,
  ProcessDetail,
  RunDetailResponse,
  RunOperationalSummary,
  UiEvent,
  WorkspaceIndex,
} from "./types";

type StreamConnectionState = {
  status: "connecting" | "connected" | "degraded" | "disconnected";
  openedAt?: string | null;
  lastEventAt?: string | null;
  lastErrorAt?: string | null;
  eventCount: number;
  errorCount: number;
};

type LiveLine = {
  id: string;
  line: string;
  stream: string;
  timestamp: string;
  processId: string;
};

type HookState<T> = {
  data: T | null;
  error: string | null;
  loading: boolean;
};

const globalEventTypes = [
  "process.output",
  "process.updated",
  "workspace.updated",
  "run.updated",
  "run.live.updated",
  "run.summary.updated",
  "run.phase.changed",
  "run.focus.changed",
  "run.warning.appended",
  "campaign.updated",
  "campaign.summary.updated",
  "campaign.artifact.updated",
  "run.timeline.appended",
  "run.message.appended",
  "run.tool.appended",
  "run.patch.appended",
  "run.command.appended",
  "run.personality.appended",
  "run.mechanism.appended",
  "run.skill.appended",
  "run.token.appended",
  "artifact.updated",
  "system",
  "system.warning",
] as const;

const globalEventSubscribers = new Set<(event: UiEvent) => void>();
const globalStreamSubscribers = new Set<(state: StreamConnectionState) => void>();
let globalEventSource: EventSource | null = null;
let globalStreamState: StreamConnectionState = {
  status: "connecting",
  openedAt: null,
  lastEventAt: null,
  lastErrorAt: null,
  eventCount: 0,
  errorCount: 0,
};

function emitStreamState(next: Partial<StreamConnectionState>) {
  globalStreamState = {
    ...globalStreamState,
    ...next,
  };
  for (const subscriber of globalStreamSubscribers) {
    subscriber(globalStreamState);
  }
}

function ensureGlobalEventSource() {
  if (globalEventSource || typeof window === "undefined") return;
  const source = new EventSource("/api/events");
  globalEventSource = source;
  emitStreamState({ status: "connecting" });

  source.onopen = () => {
    emitStreamState({
      status: "connected",
      openedAt: new Date().toISOString(),
    });
  };

  source.onerror = () => {
    emitStreamState({
      status: globalStreamState.eventCount > 0 ? "degraded" : "disconnected",
      lastErrorAt: new Date().toISOString(),
      errorCount: globalStreamState.errorCount + 1,
    });
  };

  const listener = (event: MessageEvent<string>) => {
    try {
      const decoded = JSON.parse(event.data) as UiEvent;
      emitStreamState({
        status: "connected",
        lastEventAt: new Date().toISOString(),
        eventCount: globalStreamState.eventCount + 1,
      });
      for (const subscriber of globalEventSubscribers) {
        subscriber(decoded);
      }
    } catch {
      // ignore malformed events
    }
  };

  for (const eventType of globalEventTypes) {
    source.addEventListener(eventType, listener as EventListener);
  }
}

function teardownGlobalEventSourceIfIdle() {
  if (!globalEventSource) return;
  if (globalEventSubscribers.size > 0 || globalStreamSubscribers.size > 0) return;
  globalEventSource.close();
  globalEventSource = null;
  globalStreamState = {
    status: "disconnected",
    openedAt: null,
    lastEventAt: null,
    lastErrorAt: globalStreamState.lastErrorAt,
    eventCount: globalStreamState.eventCount,
    errorCount: globalStreamState.errorCount,
  };
}

function payloadRunId(payload: unknown): string | null {
  if (!payload || typeof payload !== "object") return null;
  const record = payload as Record<string, unknown>;
  const direct = record.run_id ?? record.runId;
  if (typeof direct === "string" && direct.length > 0) return direct;
  const row = record.row;
  if (row && typeof row === "object") {
    const rowRecord = row as Record<string, unknown>;
    const nested = rowRecord.run_id ?? rowRecord.runId;
    if (typeof nested === "string" && nested.length > 0) return nested;
  }
  return null;
}

function toBucket(event: UiEvent): LiveEventBucket {
  const payload = typeof event.payload === "object" && event.payload !== null
    ? event.payload as Record<string, unknown>
    : {};
  const timestamp = typeof payload.timestamp === "string"
    ? payload.timestamp
    : typeof payload.generated_at === "string"
      ? payload.generated_at
      : typeof payload.last_event_at === "string"
        ? payload.last_event_at
        : undefined;
  return {
    type: event.type,
    timestamp,
    payload,
  };
}

function usePollingResource<T>(
  loader: () => Promise<T>,
  deps: DependencyList,
  options?: { intervalMs?: number; enabled?: boolean },
): HookState<T> & { refresh: () => Promise<void> } {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const inflightRef = useRef<Promise<void> | null>(null);
  const mountedRef = useRef(true);

  const refresh = useCallback(async () => {
    if (options?.enabled === false) {
      setLoading(false);
      return;
    }
    if (inflightRef.current) {
      return inflightRef.current;
    }
    const run = (async () => {
      try {
        const next = await loader();
        if (!mountedRef.current) return;
        setData(next);
        setError(null);
      } catch (err) {
        if (!mountedRef.current) return;
        setError(String(err));
      } finally {
        if (mountedRef.current) {
          setLoading(false);
        }
        inflightRef.current = null;
      }
    })();
    inflightRef.current = run;
    return run;
  }, [loader]);

  useEffect(() => {
    mountedRef.current = true;
    if (options?.enabled === false) {
      setLoading(false);
      return;
    }
    setLoading(true);
    void refresh();
    return () => {
      mountedRef.current = false;
    };
  }, [...deps, refresh]);

  useEffect(() => {
    if (options?.enabled === false || !options?.intervalMs) {
      return;
    }
    const timer = window.setInterval(() => {
      if (typeof document !== "undefined" && document.visibilityState === "hidden") {
        return;
      }
      void refresh();
    }, options.intervalMs);
    return () => window.clearInterval(timer);
  }, [options?.enabled, options?.intervalMs, refresh]);

  return { data, error, loading, refresh };
}

export function useLiveEvents(handler: (event: UiEvent) => void) {
  useEffect(() => {
    ensureGlobalEventSource();
    globalEventSubscribers.add(handler);
    return () => {
      globalEventSubscribers.delete(handler);
      teardownGlobalEventSourceIfIdle();
    };
  }, [handler]);
}

export function useEventStreamStatus() {
  const [state, setState] = useState<StreamConnectionState>(globalStreamState);

  useEffect(() => {
    ensureGlobalEventSource();
    globalStreamSubscribers.add(setState);
    setState(globalStreamState);
    return () => {
      globalStreamSubscribers.delete(setState);
      teardownGlobalEventSourceIfIdle();
    };
  }, []);

  return state;
}

export function useWorkspaceIndex() {
  const resource = usePollingResource(() => api.workspaceIndex(), [], { intervalMs: 5000 });
  useLiveEvents((event) => {
    if (event.type === "workspace.updated" && typeof event.payload === "object") {
      resource.refresh().catch(() => undefined);
    }
  });
  return resource;
}

export function useLiveOverview() {
  const resource = usePollingResource(() => api.liveOverview(), [], { intervalMs: 1_500 });
  useLiveEvents((event) => {
    if (
      event.type === "workspace.updated" ||
      event.type === "campaign.updated" ||
      event.type === "campaign.summary.updated" ||
      event.type === "campaign.artifact.updated" ||
      event.type === "run.updated" ||
      event.type === "run.live.updated" ||
      event.type === "run.summary.updated" ||
      event.type === "run.phase.changed" ||
      event.type === "run.focus.changed" ||
      event.type === "run.warning.appended" ||
      event.type === "process.updated" ||
      event.type === "process.output" ||
      event.type === "system.warning"
    ) {
      resource.refresh().catch(() => undefined);
    }
  });
  return resource as HookState<LiveOverviewResponse> & { refresh: () => Promise<void> };
}

export function useProcesses() {
  const resource = usePollingResource(() => api.processes(), [], { intervalMs: 2500 });
  useLiveEvents((event) => {
    if (event.type === "process.updated" || event.type === "process.output") {
      resource.refresh().catch(() => undefined);
    }
  });
  return resource;
}

export function useProcessDetail(processId: string) {
  const resource = usePollingResource(
    () => api.processDetail(processId),
    [processId],
    { intervalMs: 1_500, enabled: Boolean(processId) },
  );
  useLiveEvents((event) => {
    if (!processId) return;
    if (event.type === "process.updated" || event.type === "process.output") {
      const payload = (event.payload ?? {}) as { processId?: string; id?: string };
      const eventProcessId = payload.processId ?? payload.id;
      if (!eventProcessId || eventProcessId === processId) {
        resource.refresh().catch(() => undefined);
      }
    }
  });
  return resource as HookState<ProcessDetail> & { refresh: () => Promise<void> };
}

export function useLiveRuns() {
  const resource = usePollingResource(() => api.liveRuns(), [], { intervalMs: 2_000 });
  useLiveEvents((event) => {
    if (
      event.type === "run.live.updated" ||
      event.type === "run.summary.updated" ||
      event.type === "run.phase.changed" ||
      event.type === "run.focus.changed" ||
      event.type === "run.warning.appended" ||
      event.type === "run.updated" ||
      event.type === "workspace.updated" ||
      event.type === "campaign.updated" ||
      event.type === "campaign.summary.updated"
    ) {
      resource.refresh().catch(() => undefined);
    }
  });
  return resource as HookState<LiveRunSnapshot[]> & { refresh: () => Promise<void> };
}

export function useLiveRun(runId: string) {
  const resource = usePollingResource(
    () => api.liveRun(runId),
    [runId],
    { intervalMs: 1_500, enabled: Boolean(runId) },
  );
  useLiveEvents((event) => {
    if (!runId) return;
    if (
      event.type === "run.live.updated" ||
      event.type === "run.summary.updated" ||
      event.type === "run.phase.changed" ||
      event.type === "run.focus.changed" ||
      event.type === "run.warning.appended" ||
      event.type === "run.updated" ||
      event.type === "workspace.updated"
    ) {
      const payload = (event.payload ?? {}) as { run_id?: string; runId?: string };
      const eventRunId = payload.run_id ?? payload.runId;
      if (!eventRunId || eventRunId === runId || event.type === "workspace.updated") {
        resource.refresh().catch(() => undefined);
      }
    }
  });
  return resource as HookState<LiveRunSnapshot> & { refresh: () => Promise<void> };
}

export function useRunDetail(runId: string) {
  const resource = usePollingResource(
    () => api.runDetail(runId),
    [runId],
    { intervalMs: 3000, enabled: Boolean(runId) },
  );
  useLiveEvents((event) => {
    if (!runId) return;
    if (
      event.type === "run.updated" ||
      event.type === "run.live.updated" ||
      event.type === "run.summary.updated" ||
      event.type === "run.phase.changed" ||
      event.type === "run.focus.changed" ||
      event.type === "run.warning.appended" ||
      event.type === "run.message.appended" ||
      event.type === "run.tool.appended" ||
      event.type === "run.patch.appended" ||
      event.type === "run.command.appended" ||
      event.type === "run.personality.appended" ||
      event.type === "run.mechanism.appended" ||
      event.type === "run.skill.appended" ||
      event.type === "run.token.appended" ||
      event.type === "run.timeline.appended"
    ) {
      const payload = event.payload as { run_id?: string };
      if (payload?.run_id === runId) {
        resource.refresh().catch(() => undefined);
      }
    }
  });
  return resource;
}

export function useRunOperationalSummary(runId: string) {
  const resource = usePollingResource(
    () => api.runOperationalSummary(runId),
    [runId],
    { intervalMs: 2_000, enabled: Boolean(runId) },
  );
  useLiveEvents((event) => {
    if (!runId) return;
    if (
      event.type === "run.updated" ||
      event.type === "run.live.updated" ||
      event.type === "run.summary.updated" ||
      event.type === "run.phase.changed" ||
      event.type === "run.focus.changed" ||
      event.type === "run.warning.appended" ||
      event.type === "run.message.appended" ||
      event.type === "run.tool.appended" ||
      event.type === "run.patch.appended" ||
      event.type === "run.command.appended" ||
      event.type === "run.personality.appended" ||
      event.type === "run.mechanism.appended" ||
      event.type === "run.skill.appended" ||
      event.type === "run.token.appended" ||
      event.type === "campaign.artifact.updated"
    ) {
      const payload = (event.payload ?? {}) as { run_id?: string; runId?: string };
      const eventRunId = payload.run_id ?? payload.runId;
      if (!eventRunId || eventRunId === runId || event.type === "campaign.artifact.updated") {
        resource.refresh().catch(() => undefined);
      }
    }
  });
  return resource as HookState<RunOperationalSummary> & { refresh: () => Promise<void> };
}

export function useCampaignDetail(campaignId: string) {
  const resource = usePollingResource(
    () => api.campaign(campaignId),
    [campaignId],
    { intervalMs: 4_000, enabled: Boolean(campaignId) },
  );
  useLiveEvents((event) => {
    if (!campaignId) return;
    if (
      event.type === "workspace.updated" ||
      event.type === "campaign.updated" ||
      event.type === "campaign.summary.updated" ||
      event.type === "campaign.artifact.updated"
    ) {
      resource.refresh().catch(() => undefined);
    }
  });
  return resource;
}

export function useCampaignOperationalSummary(campaignId: string) {
  const resource = usePollingResource(
    () => api.campaignOperationalSummary(campaignId),
    [campaignId],
    { intervalMs: 2_500, enabled: Boolean(campaignId) },
  );
  useLiveEvents((event) => {
    if (!campaignId) return;
    if (
      event.type === "campaign.updated" ||
      event.type === "campaign.summary.updated" ||
      event.type === "campaign.artifact.updated" ||
      event.type === "run.updated" ||
      event.type === "run.live.updated" ||
      event.type === "run.summary.updated" ||
      event.type === "run.warning.appended" ||
      event.type === "workspace.updated"
    ) {
      const payload = (event.payload ?? {}) as { campaign_id?: string; campaignId?: string };
      const eventCampaignId = payload.campaign_id ?? payload.campaignId;
      if (!eventCampaignId || eventCampaignId === campaignId || event.type === "workspace.updated") {
        resource.refresh().catch(() => undefined);
      }
    }
  });
  return resource as HookState<CampaignOperationalSummary> & { refresh: () => Promise<void> };
}

export function useArtifactPreview(artifact: ArtifactDescriptor | null, format?: string) {
  return usePollingResource(
    () => artifact ? api.artifactFile(artifact.path, format) : Promise.resolve(null),
    [artifact?.path, format],
  );
}

export function useArtifactTail(path: string | null, lines = 120, enabled = true) {
  return usePollingResource(
    () => path ? api.artifactTail(path, lines) : Promise.resolve(null),
    [path, lines],
    { intervalMs: 2000, enabled: Boolean(path) && enabled },
  );
}

export function useRecentEventLines(limit = 120) {
  const [lines, setLines] = useState<LiveLine[]>([]);
  useLiveEvents((event) => {
    if (event.type !== "process.output") return;
    const payload = event.payload as {
      processId: string;
      line: string;
      stream: string;
      timestamp: string;
    };
    setLines((current) => {
      const next = [
        {
          id: `${payload.processId}-${payload.timestamp}-${current.length}`,
          line: payload.line,
          stream: payload.stream,
          timestamp: payload.timestamp,
          processId: payload.processId,
        },
        ...current,
      ];
      return next.slice(0, limit);
    });
  });
  return lines;
}

export function useEventBuckets(limit = 80) {
  const [events, setEvents] = useState<LiveEventBucket[]>([]);
  useLiveEvents((event) => {
    setEvents((current) => {
      return [toBucket(event), ...current].slice(0, limit);
    });
  });
  return events;
}

export function useRunStream(runId: string, types: string[], limit = 120) {
  const [events, setEvents] = useState<LiveEventBucket[]>([]);
  const typeKey = types.join("|");

  useEffect(() => {
    if (!runId) {
      setEvents([]);
      return;
    }
    const source = new EventSource(api.runStreamUrl(runId, types));
    const handler = (message: MessageEvent<string>) => {
      try {
        const decoded = JSON.parse(message.data) as UiEvent;
        setEvents((current) => [toBucket(decoded), ...current].slice(0, limit));
      } catch {
        // ignore malformed stream rows
      }
    };
    const uniqueTypes = Array.from(new Set([...types, "system.warning"]));
    for (const eventType of uniqueTypes) {
      source.addEventListener(eventType, handler as EventListener);
    }
    source.onerror = () => {
      // Let browser reconnect automatically.
    };
    return () => {
      source.close();
    };
  }, [limit, runId, typeKey]);

  return events;
}

export function useRecentEventTypes(types: string[], limit = 80) {
  const events = useEventBuckets(limit * 2);
  return useMemo(
    () => events.filter((event) => types.includes(event.type)).slice(0, limit),
    [events, limit, types],
  );
}

export function useRunEventBuckets(runId: string, types: string[], limit = 80) {
  const globalEvents = useEventBuckets(limit * 4);
  const scopedEvents = useRunStream(runId, types, limit * 2);
  return useMemo(
    () => {
      const merged = [...scopedEvents, ...globalEvents]
        .filter((event) => {
          if (!types.includes(event.type)) return false;
          const eventRunId = payloadRunId(event.payload);
          return eventRunId === runId;
        })
        .filter((event, index, rows) => {
          const signature = JSON.stringify([event.type, event.timestamp, event.payload]);
          return rows.findIndex((candidate) => JSON.stringify([candidate.type, candidate.timestamp, candidate.payload]) === signature) === index;
        });
      return merged.slice(0, limit);
    },
    [globalEvents, limit, runId, scopedEvents, types],
  );
}

export function useLatestCampaign(workspace: WorkspaceIndex | null) {
  return useMemo(() => workspace?.campaigns?.[0] ?? null, [workspace]);
}

export function useActiveRuns(workspace: WorkspaceIndex | null) {
  return useMemo(
    () => (workspace?.runs ?? []).filter((run) => run.status === "running"),
    [workspace],
  );
}

export function useRunsForCampaign(
  workspace: WorkspaceIndex | null,
  campaignId: string | null | undefined,
) {
  return useMemo(() => {
    if (!workspace || !campaignId) return [];
    return workspace.runs.filter((run) => run.campaign_id === campaignId);
  }, [campaignId, workspace]);
}

export function useCampaignSelection(
  campaigns: CampaignListItem[],
  selectedCampaignId: string,
) {
  return useMemo(
    () => campaigns.find((campaign) => campaign.campaign_id === selectedCampaignId) ?? campaigns[0] ?? null,
    [campaigns, selectedCampaignId],
  );
}

export function useArtifactSelection(
  rows: ArtifactDescriptor[],
  preferredPath?: string | null,
) {
  return useMemo(() => {
    if (!rows.length) return null;
    return rows.find((artifact) => artifact.path === preferredPath) ?? rows[0];
  }, [preferredPath, rows]);
}

export function useLatestRunningProcesses(processes: ManagedProcessSnapshot[]) {
  return useMemo(
    () => processes.filter((item) => item.status === "running"),
    [processes],
  );
}

export function useDatasetArtifacts(workspace: WorkspaceIndex | null, campaignId?: string) {
  return useMemo(() => {
    if (!workspace) return [] as string[];
    const campaign = workspace.campaigns.find((item) => item.campaign_id === campaignId) ?? workspace.campaigns[0];
    return campaign?.dataset_paths ?? [];
  }, [campaignId, workspace]);
}

export function useCampaignArtifacts(detail: CampaignDetail | null, mode: "reports" | "datasets") {
  return useMemo(() => {
    if (!detail) return [];
    return mode === "datasets" ? detail.datasets : detail.reports;
  }, [detail, mode]);
}

export function readTableRows(
  detail: RunDetailResponse | null,
  key: string,
): Array<Record<string, unknown>> {
  return (detail?.tables?.[key] as Array<Record<string, unknown>> | undefined) ?? [];
}
