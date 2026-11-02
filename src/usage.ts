/**
 * Helpers for the OAuth usage API response (api.anthropic.com/api/oauth/usage).
 *
 * The usage endpoint is a read-only account query: calling it does not run a
 * model and does not count against the 5h / 7d limits. It is rate limited per
 * account, though, and Claude Code itself (/usage, status line) draws on the
 * same allowance; a 429 locks the account out for minutes up to an hour.
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
  /** After a 429: no requests for this account until this time (ms). */
  blockedUntil?: number;
  /**
   * After an automatic token refresh failed: don't retry it automatically
   * until this time (ms). Not persisted, so a restart tries once more.
   */
  pausedUntil?: number;
}

/** Automatic re-queries (launch, focus, after actions) wait at least this long. */
export const USAGE_AUTO_MIN_AGE_MS = 5 * 60_000;
/** The Usage button may re-query sooner than the automatic refresh. */
export const USAGE_MANUAL_MIN_AGE_MS = 60_000;
/** Back-off after a 429 that carried no Retry-After header. */
export const USAGE_DEFAULT_BACKOFF_MS = 5 * 60_000;
/**
 * Back-off after an automatic token refresh fails. The token endpoint sends
 * no Retry-After, and retrying on every focus only prolongs a 429.
 */
export const REFRESH_RATE_LIMIT_BACKOFF_MS = 10 * 60_000;
export const REFRESH_FAILURE_BACKOFF_MS = 60 * 60_000;

const CACHE_KEY = "c-switcher.usage.v1";

/** Last known usage per profile, so a restart neither blanks nor re-queries it. */
export function loadUsageCache(): Record<string, UsageState> {
  try {
    const raw = JSON.parse(localStorage.getItem(CACHE_KEY) ?? "{}");
    const out: Record<string, UsageState> = {};
    for (const [name, v] of Object.entries<any>(raw)) {
      out[name] = {
        loading: false,
        data: v?.data,
        fetchedAt: v?.fetchedAt,
        blockedUntil: v?.blockedUntil,
      };
    }
    return out;
  } catch {
    return {};
  }
}

export function saveUsageCache(usage: Record<string, UsageState>) {
  try {
    const out: Record<string, Omit<UsageState, "loading" | "error">> = {};
    for (const [name, v] of Object.entries(usage)) {
      out[name] = {
        data: v.data,
        fetchedAt: v.fetchedAt,
        blockedUntil: v.blockedUntil,
      };
    }
    localStorage.setItem(CACHE_KEY, JSON.stringify(out));
  } catch {
    // Storage unavailable: the cache is only a convenience.
  }
}

/** "45s", "21m", "1h 3m" — a duration in milliseconds. */
export function formatDuration(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  if (s < 60) return `${s}s`;
  const m = Math.round(s / 60);
  if (m < 60) return `${m}m`;
  return `${Math.floor(m / 60)}h ${m % 60}m`;
}

/**
 * One-line status for a profile's usage when something is off: rate limited
 * (with time left) or the last request failed. Undefined when all is well.
 */
export function usageNotice(state?: UsageState): string | undefined {
  if (!state) return undefined;
  const now = Date.now();
  if (state.blockedUntil && state.blockedUntil > now) {
    const age = state.fetchedAt
      ? ` · showing data from ${formatDuration(now - state.fetchedAt)} ago`
      : "";
    return `Rate limited by Anthropic, retry in ${formatDuration(
      state.blockedUntil - now
    )}${age}`;
  }
  if (state.error && state.pausedUntil && state.pausedUntil > now) {
    return `${state.error} · auto retry in ${formatDuration(
      state.pausedUntil - now
    )} (or click Refresh)`;
  }
  return state.error;
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
    if (diffH >= 24) return `${Math.floor(diffH / 24)}d ${diffH % 24}h`;
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
