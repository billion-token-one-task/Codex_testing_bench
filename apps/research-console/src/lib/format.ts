export function formatNumber(value: number | null | undefined) {
  if (value == null || Number.isNaN(value)) return "—";
  return new Intl.NumberFormat("en-US").format(value);
}

export function formatCompact(value: number | null | undefined) {
  if (value == null || Number.isNaN(value)) return "—";
  return new Intl.NumberFormat("en-US", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}

export function formatDate(value: string | null | undefined) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function formatDateFull(value: string | null | undefined) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(date);
}

export function truncateMiddle(value: string, max = 72) {
  if (value.length <= max) return value;
  const half = Math.floor((max - 3) / 2);
  return `${value.slice(0, half)}...${value.slice(-half)}`;
}

export function truncateEnd(value: string, max = 96) {
  if (value.length <= max) return value;
  return `${value.slice(0, max - 3)}...`;
}

export function percentFromBps(value: number | null | undefined) {
  if (value == null || Number.isNaN(value)) return "—";
  return `${(value / 100).toFixed(1)}%`;
}

export function summarizeMap(
  map: Record<string, number> | null | undefined,
  limit = 3,
) {
  if (!map) return [];
  return Object.entries(map)
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .slice(0, limit);
}

export function formatDurationMs(value: number | null | undefined) {
  if (value == null || Number.isNaN(value)) return "—";
  if (value < 1_000) return `${Math.round(value)} ms`;
  if (value < 60_000) return `${(value / 1_000).toFixed(1)} s`;
  const minutes = Math.floor(value / 60_000);
  const seconds = Math.round((value % 60_000) / 1_000);
  return `${minutes}m ${seconds}s`;
}

export function formatRelativeTime(value: string | null | undefined) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const diff = Date.now() - date.getTime();
  if (diff < 0) return formatDate(value);
  const seconds = Math.floor(diff / 1_000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function uniqueValues(values: Array<string | null | undefined>) {
  return Array.from(
    new Set(
      values
        .map((value) => value?.trim())
        .filter((value): value is string => Boolean(value)),
    ),
  ).sort((a, b) => a.localeCompare(b));
}

export function groupBy<T>(
  rows: T[],
  keyFn: (row: T) => string,
) {
  return rows.reduce<Record<string, T[]>>((acc, row) => {
    const key = keyFn(row);
    acc[key] ??= [];
    acc[key].push(row);
    return acc;
  }, {});
}

export function humanizeKey(value: string) {
  return value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[_-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function detectToneFromStatus(status: string | null | undefined) {
  if (!status) return "neutral" as const;
  if (status.includes("running")) return "running" as const;
  if (status.includes("graded") || status.includes("resolved") || status.includes("completed")) {
    return "completed" as const;
  }
  if (status.includes("failed") || status.includes("error") || status.includes("unresolved")) {
    return "failed" as const;
  }
  if (status.includes("warning")) return "warning" as const;
  return "neutral" as const;
}
