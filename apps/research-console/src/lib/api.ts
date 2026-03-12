import type {
  ArtifactFileResponse,
  ArtifactTailResponse,
  CampaignDetail,
  CampaignListItem,
  ManagedProcessSnapshot,
  RunDetailResponse,
  RunIndexEntry,
  WorkspaceIndex,
} from "./types";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
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
}

export const api = {
  workspaceIndex: () => request<WorkspaceIndex>("/api/workspace/index"),
  campaigns: () => request<CampaignListItem[]>("/api/campaigns"),
  campaign: (campaignId: string) =>
    request<CampaignDetail>(`/api/campaigns/${campaignId}`),
  campaignReports: (campaignId: string) =>
    request(`/api/campaigns/${campaignId}/reports`),
  campaignDatasets: (campaignId: string) =>
    request(`/api/campaigns/${campaignId}/datasets`),
  run: (runId: string) => request<RunIndexEntry>(`/api/runs/${runId}`),
  runDetail: (runId: string) => request<RunDetailResponse>(`/api/runs/${runId}/detail`),
  processes: () => request<ManagedProcessSnapshot[]>("/api/processes"),
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
