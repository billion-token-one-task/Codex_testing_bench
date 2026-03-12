import type {
  ArtifactFileResponse,
  ArtifactTailResponse,
  CampaignDetail,
  CampaignOperationalSummary,
  CampaignListItem,
  LiveOverviewResponse,
  ManagedProcessSnapshot,
  ProcessDetail,
  LiveRunSnapshot,
  RunDetailResponse,
  RunOperationalSummary,
  RunIndexEntry,
  WorkspaceIndex,
} from "./types";

const inflightGetRequests = new Map<string, Promise<unknown>>();

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const method = init?.method ?? "GET";
  if (method === "GET" && !init?.body) {
    const existing = inflightGetRequests.get(path);
    if (existing) {
      return existing as Promise<T>;
    }
  }

  const run = (async () => {
    const response = await fetch(path, {
      headers: {
        "Content-Type": "application/json",
        ...(init?.headers ?? {}),
      },
      ...init,
    });
    if (!response.ok) {
      const text = await response.text();
      throw new Error(text || `HTTP ${response.status}`);
    }
    return response.json() as Promise<T>;
  })();

  if (method === "GET" && !init?.body) {
    inflightGetRequests.set(path, run as Promise<unknown>);
    try {
      return await run;
    } finally {
      inflightGetRequests.delete(path);
    }
  }

  return run;
}

export const api = {
  workspaceIndex: () => request<WorkspaceIndex>("/api/workspace/index"),
  liveOverview: () => request<LiveOverviewResponse>("/api/live/overview"),
  campaigns: () => request<CampaignListItem[]>("/api/campaigns"),
  campaign: (campaignId: string) =>
    request<CampaignDetail>(`/api/campaigns/${campaignId}`),
  campaignOperationalSummary: (campaignId: string) =>
    request<CampaignOperationalSummary>(`/api/campaigns/${campaignId}/operational-summary`),
  campaignReports: (campaignId: string) =>
    request(`/api/campaigns/${campaignId}/reports`),
  campaignDatasets: (campaignId: string) =>
    request(`/api/campaigns/${campaignId}/datasets`),
  run: (runId: string) => request<RunIndexEntry>(`/api/runs/${runId}`),
  runStreamUrl: (runId: string, eventTypes?: string[]) => {
    const params = new URLSearchParams();
    if (eventTypes?.length) {
      params.set("event_types", eventTypes.join(","));
    }
    return `/api/runs/${runId}/stream${params.toString() ? `?${params.toString()}` : ""}`;
  },
  runOperationalSummary: (runId: string) => request<RunOperationalSummary>(`/api/runs/${runId}/operational-summary`),
  runDetail: (runId: string) => request<RunDetailResponse>(`/api/runs/${runId}/detail`),
  liveRun: (runId: string) => request<LiveRunSnapshot>(`/api/live/runs/${runId}`),
  processes: () => request<ManagedProcessSnapshot[]>("/api/processes"),
  processDetail: (processId: string) => request<ProcessDetail>(`/api/processes/${processId}`),
  liveRuns: () => request<LiveRunSnapshot[]>("/api/live/runs"),
  artifactFile: (path: string, format?: string) => {
    const params = new URLSearchParams({ path });
    if (format) params.set("format", format);
    return request<ArtifactFileResponse>(`/api/artifacts/file?${params.toString()}`);
  },
  artifactTail: (path: string, lines = 120) => {
    const params = new URLSearchParams({ path, lines: String(lines) });
    return request<ArtifactTailResponse>(`/api/artifacts/tail?${params.toString()}`);
  },
  action: (kind: string, body: Record<string, unknown>) =>
    request<{ process_id: string; kind: string; status: string }>(
      `/api/actions/${kind}`,
      {
        method: "POST",
        body: JSON.stringify(body),
      },
    ),
};
