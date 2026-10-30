/**
 * Helpers for the OAuth usage API response (api.anthropic.com/api/oauth/usage).
 *
 * The usage endpoint is a read-only account query: calling it does not run a
 * model and does not count against the 5h / 7d limits.
 */

/* eslint-disable @typescript-eslint/no-explicit-any */

export interface UsageBar {
  key: string;
  /** Short label, e.g. "5h", "7d", "Fable 7d". */
  label: string;
  /** Percent of the window already used, 0–100. */
  pct: number;
  resetsAt?: string;
}

export interface UsageState {
  loading: boolean;
  error?: string;
  data?: any;
  fetchedAt?: number;
}

/** Label for one entry of the `limits[]` array. */
function limitLabel(limit: any): string {
  const model = limit?.scope?.model?.display_name;
  const surface = limit?.scope?.surface;
  switch (limit?.kind) {
    case "session":
      return "5h";
    case "weekly_all":
      return "7d";
    default:
      if (model) return `${model} 7d`;
      if (surface) return `${surface} 7d`;
      return String(limit?.kind ?? "limit");
  }
}

/**
 * Flatten a usage response into display bars, in the order the API lists them.
 * Prefers the newer `limits[]` array (which carries model-scoped limits such as
 * Fable) and falls back to the legacy five_hour / seven_day fields.
 */
export function extractUsageBars(data: any): UsageBar[] {
  if (!data) return [];

  const limits = Array.isArray(data.limits) ? data.limits : [];
  if (limits.length > 0) {
    return limits.map((l: any, i: number) => ({
      key: `${l.kind ?? "limit"}-${i}`,
      label: limitLabel(l),
      pct: Math.round(Number(l.percent ?? 0)),
      resetsAt: l.resets_at ?? undefined,
    }));
  }

  const legacy: [string, string, any][] = [
    ["five_hour", "5h", data.five_hour],
    ["seven_day", "7d", data.seven_day],
    ["seven_day_opus", "Opus 7d", data.seven_day_opus],
    ["seven_day_sonnet", "Sonnet 7d", data.seven_day_sonnet],
  ];
  return legacy
    .filter(([, , v]) => v && typeof v === "object")
    .map(([key, label, v]) => ({
      key,
      label,
      pct: Math.round(Number(v.utilization ?? 0)),
      resetsAt: v.resets_at ?? undefined,
    }));
}

/** "1h 12m", "2d 5h", "now" — time until an ISO timestamp. */
export function formatResetTime(isoStr: string): string {
  try {
    const diffMs = new Date(isoStr).getTime() - Date.now();
    if (Number.isNaN(diffMs)) return isoStr;
    if (diffMs <= 0) return "now";
    const diffH = Math.floor(diffMs / 3600000);
    const diffM = Math.floor((diffMs % 3600000) / 60000);
    if (diffH > 24) return `${Math.floor(diffH / 24)}d ${diffH % 24}h`;
    if (diffH > 0) return `${diffH}h ${diffM}m`;
    return `${diffM}m`;
  } catch {
    return isoStr;
  }
}

/** Tailwind colour classes for a usage percentage (same thresholds app-wide). */
export function usageTone(pct: number): { bar: string; text: string } {
  if (pct >= 80) return { bar: "bg-danger", text: "text-danger" };
  if (pct >= 50) return { bar: "bg-warning", text: "text-warning" };
  return { bar: "bg-success", text: "text-success" };
}
