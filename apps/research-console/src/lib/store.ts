import { type DependencyList, useCallback, useEffect, useMemo, useState } from "react";

import { api } from "./api";
import type {
  ArtifactDescriptor,
  ManagedProcessSnapshot,
  RunDetailResponse,
  UiEvent,
  WorkspaceIndex,
} from "./types";

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

function usePollingResource<T>(
  loader: () => Promise<T>,
  deps: DependencyList,
  options?: { intervalMs?: number; enabled?: boolean },
): HookState<T> & { refresh: () => Promise<void> } {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const next = await loader();
      setData(next);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [loader]);

  useEffect(() => {
    let mounted = true;
    setLoading(true);
    loader()
      .then((next) => {
        if (!mounted) return;
        setData(next);
        setError(null);
        setLoading(false);
      })
      .catch((err) => {
        if (!mounted) return;
        setError(String(err));
        setLoading(false);
      });
    return () => {
      mounted = false;
    };
  }, deps);

  useEffect(() => {
    if (options?.enabled === false || !options?.intervalMs) {
      return;
    }
    const timer = window.setInterval(() => {
      void refresh();
    }, options.intervalMs);
    return () => window.clearInterval(timer);
  }, [options?.enabled, options?.intervalMs, refresh]);

  return { data, error, loading, refresh };
}

export function useLiveEvents(handler: (event: UiEvent) => void) {
  useEffect(() => {
    const source = new EventSource("/api/events");
    const listener = (event: MessageEvent<string>) => {
      try {
        const decoded = JSON.parse(event.data) as UiEvent;
        handler(decoded);
      } catch {
        // ignore malformed events
      }
    };
    const events = [
      "process.output",
      "process.updated",
      "workspace.updated",
      "run.updated",
      "artifact.updated",
      "system",
    ];
    for (const eventType of events) {
      source.addEventListener(eventType, listener as EventListener);
    }
    return () => {
      source.close();
    };
  }, [handler]);
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

export function useProcesses() {
  const resource = usePollingResource(() => api.processes(), [], { intervalMs: 2500 });
  useLiveEvents((event) => {
    if (event.type === "process.updated") {
      resource.refresh().catch(() => undefined);
    }
  });
  return resource;
}

export function useRunDetail(runId: string) {
  const resource = usePollingResource(
    () => api.runDetail(runId),
    [runId],
    { intervalMs: 3000, enabled: Boolean(runId) },
  );
  useLiveEvents((event) => {
    if (!runId) return;
    if (event.type === "run.updated") {
      const payload = event.payload as { run_id?: string };
      if (payload?.run_id === runId) {
        resource.refresh().catch(() => undefined);
      }
    }
  });
  return resource;
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

export function useLatestCampaign(workspace: WorkspaceIndex | null) {
  return useMemo(() => workspace?.campaigns?.[0] ?? null, [workspace]);
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

export function readTableRows(
  detail: RunDetailResponse | null,
  key: string,
): Array<Record<string, unknown>> {
  return (detail?.tables?.[key] as Array<Record<string, unknown>> | undefined) ?? [];
}
