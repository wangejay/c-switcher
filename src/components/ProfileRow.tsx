import type { ReactNode } from "react";
import type { ProfileEntry } from "../types";
import {
  extractUsageBars,
  formatResetTime,
  usageNotice,
  usageTone,
  type UsageBar,
  type UsageState,
} from "../usage";

interface Props {
  profile: ProfileEntry;
  isCurrent: boolean;
  onSwitch: () => void;
  /** `force` is true on Shift+click: refresh even if the token is still valid. */
  onRefresh: (force: boolean) => void;
  onDelete: () => void;
  onUsage: () => void;
  refreshing?: boolean;
  usage?: UsageState;
}

const avatarColors = [
  "from-violet-500 to-purple-600",
  "from-rose-500 to-pink-600",
  "from-cyan-500 to-blue-600",
  "from-amber-500 to-orange-600",
  "from-emerald-500 to-teal-600",
  "from-fuchsia-500 to-pink-600",
  "from-sky-500 to-indigo-600",
  "from-lime-500 to-green-600",
];

function getAvatarColor(name: string) {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  return avatarColors[Math.abs(hash) % avatarColors.length];
}

/* Avatar (44px) + top-row gap (16px): indents the usage strip under the text. */
const INFO_INDENT = 60;

export default function ProfileRow({
  profile,
  isCurrent,
  onSwitch,
  onRefresh,
  onDelete,
  onUsage,
  refreshing = false,
  usage,
}: Props) {
  const colorClass = getAvatarColor(profile.name);
  const initial = profile.name[0]?.toUpperCase() ?? "?";
  const organization =
    profile.organization && profile.organization !== "N/A"
      ? profile.organization
      : null;

  return (
    <div
      className={`profile-card group relative flex flex-col border transition-all duration-200 ${
        isCurrent
          ? "profile-card-active border-blue-400/45"
          : "border-border2 bg-surface hover:border-white/15 hover:bg-surface-light/50"
      }`}
      style={{ gap: 10 }}
    >
      {/* Inner top highlight */}
      <div
        className={`absolute inset-x-0 top-0 h-px rounded-t-2xl bg-gradient-to-r from-transparent to-transparent pointer-events-none ${
          isCurrent ? "via-blue-300/40" : "via-white/[0.06]"
        }`}
      />

      {isCurrent && (
        <>
          {/* Accent rail on the left edge */}
          <div className="active-rail absolute left-0 pointer-events-none" />
          {/* Light sweep across the card, once on activation then idle */}
          <div className="active-sweep absolute inset-0 rounded-2xl pointer-events-none" />
        </>
      )}

      {/* Top row: avatar · identity · actions */}
      <div className="flex items-center gap-4">
        {/* Avatar */}
        <div
          className={`avatar bg-gradient-to-br ${colorClass} flex items-center justify-center text-[16px] font-bold shrink-0 shadow-lg ${
            isCurrent
              ? "ring-2 ring-blue-400/80 ring-offset-2 ring-offset-bg"
              : "ring-1 ring-white/20"
          }`}
        >
          {initial}
        </div>

        {/* Info */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2.5">
            <span className="text-[16px] font-semibold truncate leading-tight">
              {profile.name}
            </span>
            {profile.plan && (
              <span
                className="shrink-0 text-[11px] font-semibold leading-none rounded-md bg-amber-400/12 text-amber-300 ring-1 ring-amber-300/25"
                style={{ padding: "4px 7px" }}
                title="Subscription plan"
              >
                {profile.plan}
              </span>
            )}
            {/* Status dot */}
            <div
              className={`w-2 h-2 rounded-full shrink-0 ${
                profile.isExpired
                  ? "bg-warning shadow-[0_0_6px] shadow-warning/50"
                  : "bg-success shadow-[0_0_6px] shadow-success/50"
              }`}
              title={profile.isExpired ? "Token expired" : "Token valid"}
            />
          </div>
          <div
            className="text-[14px] text-text2 truncate mt-1 leading-tight"
            title={profile.email}
          >
            {profile.email}
          </div>
          {organization && (
            <div
              className="text-[13px] text-text3 truncate mt-0.5 leading-tight"
              title={organization}
            >
              {organization}
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center gap-1 shrink-0">
          {isCurrent ? (
            <span className="badge-active inline-flex items-center gap-2 text-[14px] font-bold bg-blue-500/20 text-blue-300 ring-1 ring-blue-400/50 tracking-wide">
              <span className="relative flex w-2 h-2">
                <span className="absolute inset-0 rounded-full bg-blue-400 animate-ping opacity-75" />
                <span className="relative w-2 h-2 rounded-full bg-blue-400" />
              </span>
              Active
            </span>
          ) : (
            <button
              className="action-btn opacity-0 group-hover:opacity-100 hover:bg-accent2/15 hover:text-accent2"
              onClick={onSwitch}
            >
              <svg
                className="w-[13px] h-[13px]"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M7.5 21 3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5"
                />
              </svg>
              Enable
            </button>
          )}
          <button
            className={`action-btn hover:bg-sky-500/15 hover:text-sky-400 ${
              isCurrent ? "" : "opacity-0 group-hover:opacity-100"
            }`}
            onClick={onUsage}
            title="Usage details"
          >
            <svg
              className="w-[13px] h-[13px]"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 0 1 3 19.875v-6.75ZM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V8.625ZM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 0 1-1.125-1.125V4.125Z"
              />
            </svg>
            Usage
          </button>
          <button
            className={`action-btn hover:bg-surface-lighter hover:text-text disabled:opacity-50 disabled:cursor-default ${
              isCurrent || refreshing ? "" : "opacity-0 group-hover:opacity-100"
            }`}
            onClick={(e) => onRefresh(e.shiftKey)}
            disabled={refreshing}
            title="Refresh token (Shift+click to force while still valid)"
          >
            <svg
              className={`w-[13px] h-[13px] ${refreshing ? "animate-spin" : ""}`}
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182"
              />
            </svg>
            {refreshing ? "Refreshing…" : "Refresh"}
          </button>
          <button
            className={`action-btn hover:bg-danger/12 hover:text-danger ${
              isCurrent ? "" : "opacity-0 group-hover:opacity-100"
            }`}
            onClick={onDelete}
          >
            <svg
              className="w-[13px] h-[13px]"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0"
              />
            </svg>
            Delete
          </button>
        </div>
      </div>

      {/* Usage strip */}
      <UsageStrip usage={usage} />
    </div>
  );
}

/* ── Usage strip: one compact bar per limit (5h · 7d · Fable 7d …) ── */
function UsageStrip({ usage }: { usage?: UsageState }) {
  const bars = extractUsageBars(usage?.data);
  const stale = usage?.loading && bars.length > 0;

  const notice = usageNotice(usage);

  let body: ReactNode;
  if (bars.length > 0) {
    body = (
      <>
        {bars.map((b) => (
          <UsageChip key={b.key} bar={b} />
        ))}
        {notice && !usage?.loading && (
          <span
            className="text-[12px] text-warning/90 truncate"
            style={{ gridColumn: "1 / -1" }}
            title={notice}
          >
            ⏸ {notice}
          </span>
        )}
      </>
    );
  } else if (usage?.loading) {
    body = (
      <span className="flex items-center gap-2 text-[13px] text-text3">
        <span
          className="w-3 h-3 rounded-full border-2 border-border2 animate-spin"
          style={{ borderTopColor: "var(--color-accent2)" }}
        />
        Loading usage…
      </span>
    );
  } else if (notice) {
    body = (
      <span
        className={`text-[13px] truncate ${
          (usage?.blockedUntil ?? 0) > Date.now() ||
          (usage?.pausedUntil ?? 0) > Date.now()
            ? "text-warning/90"
            : "text-danger/80"
        }`}
        title={notice}
      >
        Usage unavailable · {notice}
      </span>
    );
  } else {
    body = <span className="text-[13px] text-text3">No usage data</span>;
  }

  return (
    <div
      className={`grid items-center gap-x-5 gap-y-1.5 min-w-0 transition-opacity ${
        stale ? "opacity-60" : ""
      }`}
      style={{
        marginLeft: INFO_INDENT,
        // Same column width on every card, so chips line up vertically.
        gridTemplateColumns:
          bars.length > 0 ? "repeat(auto-fill, minmax(200px, 1fr))" : "1fr",
      }}
    >
      {body}
    </div>
  );
}

function UsageChip({ bar }: { bar: UsageBar }) {
  const tone = usageTone(bar.pct);
  const resets = bar.resetsAt ? formatResetTime(bar.resetsAt) : null;
  return (
    <div
      className="grid items-center gap-1.5 text-[13px] whitespace-nowrap min-w-0"
      style={{ gridTemplateColumns: "54px minmax(32px, 1fr) 38px 60px" }}
      title={`${bar.label}: ${bar.pct}% used${resets ? `, resets in ${resets}` : ""}`}
    >
      <span className="text-text2 font-medium truncate">{bar.label}</span>
      <div className="h-2 rounded-full bg-white/10 overflow-hidden">
        <div
          className={`h-full rounded-full ${tone.bar}`}
          style={{
            width: `${Math.min(bar.pct, 100)}%`,
            transition: "width 0.5s ease",
          }}
        />
      </div>
      <span className={`font-semibold tabular-nums text-right ${tone.text}`}>
        {bar.pct}%
      </span>
      <span className="text-[12px] text-text2 tabular-nums truncate">
        {resets ? `↻ ${resets}` : ""}
      </span>
    </div>
  );
}
