import { extractUsageBars, formatResetTime } from "../usage";

interface Props {
  open: boolean;
  profileName: string;
  loading: boolean;
  error?: string;
  /** Shown above the data, e.g. a rate-limit back-off over cached numbers. */
  notice?: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  data?: any;
  onClose: () => void;
}

export default function UsageModal({
  open,
  profileName,
  loading,
  error,
  notice,
  data,
  onClose,
}: Props) {
  if (!open) return null;

  return (
    <div
      className="dialog-overlay fixed inset-0 bg-black/60 backdrop-blur-sm z-[9998] flex items-center justify-center"
      onClick={onClose}
    >
      <div
        className="dialog-content bg-surface border border-border2 shadow-2xl shadow-black/50 relative overflow-hidden w-[400px] max-w-[92%]"
        style={{ borderRadius: 20 }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="modal-header relative">
          <button
            className="absolute top-[16px] right-[16px] w-7 h-7 rounded-lg flex items-center justify-center text-text3 hover:text-text hover:bg-surface-lighter transition-all"
            onClick={onClose}
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
          <h3 className="text-[18px] font-bold">{profileName}</h3>
          <p className="text-[14px] text-text3 mt-0.5">Usage & Rate Limits</p>
        </div>

        {/* Loading */}
        {loading && (
          <div className="flex items-center justify-center py-16 pb-20">
            <div
              className="w-7 h-7 rounded-full border-[2.5px] border-border2 animate-spin"
              style={{ borderTopColor: "var(--color-accent2)" }}
            />
          </div>
        )}

        {/* Error */}
        {error && (
          <div className="modal-body">
            <div className="rate-card text-danger text-[14px] ring-1 ring-danger/15 leading-relaxed" style={{ background: "rgba(248,113,113,0.08)" }}>
              {error}
            </div>
          </div>
        )}

        {/* Notice over cached data */}
        {!error && data && notice && (
          <div className="modal-body" style={{ paddingBottom: 0 }}>
            <div
              className="rate-card text-warning text-[13px] ring-1 ring-warning/20 leading-relaxed"
              style={{ background: "rgba(251,191,36,0.08)" }}
            >
              {notice}
            </div>
          </div>
        )}

        {/* Content */}
        {!loading && !error && data && <UsageContent data={data} />}
      </div>
    </div>
  );
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function UsageContent({ data }: { data: any }) {
  const monthlyLimit = data.monthly_limit;
  const usedCredits = data.used_credits ?? 0;
  const utilization = data.utilization ?? 0;
  const bars = extractUsageBars(data);

  const hasMonthly = monthlyLimit !== undefined;

  if (!hasMonthly && bars.length === 0) {
    return (
      <div className="modal-body">
        <pre className="text-[14px] text-text2 bg-bg2 rounded-xl overflow-auto max-h-[300px] ring-1 ring-border whitespace-pre-wrap break-all" style={{ padding: 16 }}>
          {JSON.stringify(data, null, 2)}
        </pre>
      </div>
    );
  }

  const remaining = monthlyLimit - usedCredits;

  return (
    <div className="modal-body flex flex-col" style={{ gap: 14 }}>
      {/* Monthly — hero ring */}
      {hasMonthly && (
        <div className="flex items-center gap-5">
          {/* Ring */}
          <Ring pct={utilization} size={96} strokeWidth={8} />

          {/* Stats */}
          <div className="flex-1 space-y-2.5">
            <StatRow label="Used" value={usedCredits.toLocaleString()} accent />
            <StatRow label="Limit" value={monthlyLimit.toLocaleString()} />
            <StatRow label="Remaining" value={remaining.toLocaleString()} />
          </div>
        </div>
      )}

      {/* Rate limits: 5h · 7d · model-scoped (e.g. Fable) */}
      {bars.length > 0 && (
        <div className="flex flex-col gap-sm">
          {bars.map((b) => (
            <RateBar key={b.key} label={b.label} pct={b.pct} resetsAt={b.resetsAt} />
          ))}
        </div>
      )}
    </div>
  );
}

/* ── Ring ── */
function Ring({ pct, size, strokeWidth }: { pct: number; size: number; strokeWidth: number }) {
  const r = (size - strokeWidth) / 2;
  const circ = 2 * Math.PI * r;
  const filled = circ * (Math.min(pct, 100) / 100);
  const color = pct >= 80 ? "#f87171" : pct >= 50 ? "#fbbf24" : "#34d399";
  const textColor = pct >= 80 ? "text-danger" : pct >= 50 ? "text-warning" : "text-success";

  return (
    <div className="relative shrink-0" style={{ width: size, height: size }}>
      <svg width={size} height={size} className="transform -rotate-90">
        {/* Track */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="rgba(255,255,255,0.06)"
          strokeWidth={strokeWidth}
        />
        {/* Fill */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={color}
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          strokeDasharray={circ}
          strokeDashoffset={circ - filled}
          style={{ filter: `drop-shadow(0 0 6px ${color}50)`, transition: "stroke-dashoffset 0.6s ease" }}
        />
      </svg>
      {/* Center text */}
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <span className={`text-[22px] font-bold leading-none ${textColor}`}>
          {pct.toFixed(0)}
        </span>
        <span className="text-[14px] text-text3 mt-0.5">% used</span>
      </div>
    </div>
  );
}

/* ── StatRow ── */
function StatRow({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-[14px] text-text3">{label}</span>
      <span className={`text-[15px] font-semibold tabular-nums ${accent ? "text-text" : "text-text2"}`}>
        {value}
      </span>
    </div>
  );
}

/* ── RateBar ── */
function RateBar({ label, pct, resetsAt }: { label: string; pct: number; resetsAt?: string }) {
  const color = pct >= 80 ? "bg-danger" : pct >= 50 ? "bg-warning" : "bg-success";
  const glowColor = pct >= 80 ? "shadow-danger/25" : pct >= 50 ? "shadow-warning/20" : "shadow-success/20";
  const textColor = pct >= 80 ? "text-danger" : pct >= 50 ? "text-warning" : "text-success";

  return (
    <div className="rate-card bg-bg2 ring-1 ring-border">
      <div className="flex items-center justify-between mb-2">
        <span className="text-[14px] text-text2 font-medium">{label}</span>
        <div className="flex items-center gap-2">
          {resetsAt && (
            <span className="text-[14px] text-text3">
              resets {formatResetTime(resetsAt)}
            </span>
          )}
          <span className={`text-[15px] font-bold tabular-nums ${textColor}`}>
            {pct}%
          </span>
        </div>
      </div>
      <div className="h-2 rounded-full bg-white/10 overflow-hidden">
        <div
          className={`h-full rounded-full ${color} shadow-md ${glowColor}`}
          style={{ width: `${Math.min(pct, 100)}%`, transition: "width 0.5s ease" }}
        />
      </div>
    </div>
  );
}
