import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { CurrentInfo, ProfileEntry } from "./types";
import type { ToastItem, ToastType } from "./components/Toast";
import type { UsageState } from "./usage";
import {
  getCurrentInfo,
  listProfiles,
  backupProfile,
  switchProfile,
  deleteProfile,
  refreshToken,
  getUsage,
} from "./api";
import TopBar from "./components/TopBar";
import ProfileRow from "./components/ProfileRow";
import AddModal from "./components/AddModal";
import UsageModal from "./components/UsageModal";
import ConfirmDialog from "./components/ConfirmDialog";
import Toast from "./components/Toast";
import VersionLabel from "./components/VersionLabel";

let toastId = 0;

/** Re-query usage when the window regains focus, at most this often. */
const USAGE_FOCUS_THROTTLE_MS = 30_000;

/** Card reorder animation (FLIP) when the active account moves to the top. */
const REORDER_MS = 600;
const REORDER_EASING = "cubic-bezier(0.16, 1, 0.3, 1)";

/** The first profile whose email matches the active account is the Active one. */
function currentProfileName(
  profiles: ProfileEntry[],
  email?: string
): string | null {
  if (!email) return null;
  return profiles.find((p) => p.email === email)?.name ?? null;
}

export default function App() {
  const [info, setInfo] = useState<CurrentInfo | null>(null);
  const [profiles, setProfiles] = useState<ProfileEntry[]>([]);
  const [usage, setUsage] = useState<Record<string, UsageState>>({});
  const lastUsageFetch = useRef(0);
  const [showAddModal, setShowAddModal] = useState(false);
  const [confirmTarget, setConfirmTarget] = useState<string | null>(null);
  const [usageTarget, setUsageTarget] = useState<string | null>(null);
  const [usageLoading, setUsageLoading] = useState(false);
  const [usageError, setUsageError] = useState<string | undefined>();
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [usageData, setUsageData] = useState<any>(undefined);
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const [refreshing, setRefreshing] = useState<Set<string>>(new Set());
  const listRef = useRef<HTMLDivElement>(null);
  const cardEls = useRef(new Map<string, HTMLDivElement>());
  const cardTops = useRef(new Map<string, number>());
  const lastOrder = useRef("");

  const toast = useCallback((message: string, type: ToastType = "info") => {
    setToasts((prev) => [...prev, { id: ++toastId, message, type }]);
  }, []);

  const removeToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  /**
   * Fetch one profile's usage. The usage endpoint is a read-only account
   * query: it runs no model and costs no tokens. Returns true if the
   * profile's token had to be refreshed first.
   */
  const loadUsageFor = useCallback(
    async (p: ProfileEntry, isCurrent: boolean): Promise<boolean> => {
      setUsage((prev) => ({
        ...prev,
        [p.name]: { ...prev[p.name], loading: true },
      }));

      // Expired non-active profile: refresh its token first (same rule as the
      // csw CLI). The active account's token is managed by Claude Code itself.
      let refreshed = false;
      if (p.isExpired && !isCurrent) {
        const r = await refreshToken(p.name);
        refreshed = r.success && !r.skipped;
      }

      // The active profile reads the live keychain token, which Claude Code
      // keeps fresh; the profile file's copy may be stale.
      const result = await getUsage(isCurrent ? null : p.name);
      setUsage((prev) => ({
        ...prev,
        [p.name]: result.success
          ? { loading: false, data: result.data, fetchedAt: Date.now() }
          : {
              loading: false,
              error: result.error || "Failed to fetch usage",
              data: prev[p.name]?.data,
              fetchedAt: prev[p.name]?.fetchedAt,
            },
      }));
      return refreshed;
    },
    []
  );

  /** Fetch usage for every profile in parallel. */
  const loadAllUsage = useCallback(
    async (list: ProfileEntry[], currentEmail?: string) => {
      lastUsageFetch.current = Date.now();
      const current = currentProfileName(list, currentEmail);
      const refreshed = await Promise.all(
        list.map((p) => loadUsageFor(p, p.name === current))
      );
      // A token was rotated: re-list so the status dots catch up.
      if (refreshed.some(Boolean)) setProfiles(await listProfiles());
    },
    [loadUsageFor]
  );

  /** Reload the active account, the profile list and every profile's usage. */
  const reloadAll = useCallback(async () => {
    const [inf, list] = await Promise.all([getCurrentInfo(), listProfiles()]);
    setInfo(inf);
    setProfiles(list);
    await loadAllUsage(list, inf.email);
  }, [loadAllUsage]);

  useEffect(() => {
    reloadAll();
  }, [reloadAll]);

  // Usage is only re-queried on focus (throttled) and after actions; there is
  // no background polling.
  useEffect(() => {
    const onFocus = () => {
      if (document.visibilityState === "hidden") return;
      if (Date.now() - lastUsageFetch.current < USAGE_FOCUS_THROTTLE_MS) return;
      reloadAll();
    };
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onFocus);
    return () => {
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onFocus);
    };
  }, [reloadAll]);

  const handleBackup = async (name: string) => {
    const result = await backupProfile(name);
    if (result.success) {
      toast(`Backed up: ${result.email}`, "success");
      reloadAll();
    } else {
      toast(result.error || "Backup failed", "error");
    }
  };

  const handleSwitch = async (name: string) => {
    const result = await switchProfile(name);
    if (result.success) {
      toast(`Switched: ${result.from} → ${result.to}`, "success");
      reloadAll();
    } else {
      toast(result.error || "Switch failed", "error");
    }
  };

  const handleRefresh = async (profileName: string, force = false) => {
    // One in-flight refresh per profile; repeated clicks would only earn a 429.
    if (refreshing.has(profileName)) return;
    setRefreshing((prev) => new Set(prev).add(profileName));
    toast(`Refreshing ${profileName} token...`, "info");
    try {
      const result = await refreshToken(profileName, force);
      if (result.skipped) {
        toast(
          `${profileName} token is still valid (expires ${result.expiresAt}). Shift+click Refresh to force.`,
          "info"
        );
      } else if (result.success) {
        toast(`Token refreshed! Expires: ${result.expiresAt}`, "success");
        reloadAll();
      } else {
        toast(result.error || "Refresh failed", "error");
      }
    } finally {
      setRefreshing((prev) => {
        const next = new Set(prev);
        next.delete(profileName);
        return next;
      });
    }
  };

  const handleDelete = async () => {
    if (!confirmTarget) return;
    const name = confirmTarget;
    setConfirmTarget(null);
    const result = await deleteProfile(name);
    if (result.success) {
      toast(result.message || "Deleted", "success");
      setUsage((prev) => {
        const next = { ...prev };
        delete next[name];
        return next;
      });
      reloadAll();
    } else {
      toast(result.error || "Delete failed", "error");
    }
  };

  const handleUsage = async (profileName: string) => {
    // Show what the row already has, then refresh it in the background.
    const cached = usage[profileName]?.data;
    setUsageTarget(profileName);
    setUsageLoading(!cached);
    setUsageError(undefined);
    setUsageData(cached);
    const isCurrent =
      profileName === currentProfileName(profiles, info?.email);
    const result = await getUsage(isCurrent ? null : profileName);
    setUsageLoading(false);
    if (result.success) {
      setUsageData(result.data);
      setUsage((prev) => ({
        ...prev,
        [profileName]: {
          loading: false,
          data: result.data,
          fetchedAt: Date.now(),
        },
      }));
    } else {
      setUsageError(result.error || "Failed to fetch usage");
    }
  };

  const handleReload = () => {
    reloadAll();
    toast("Reloaded!", "success");
  };

  const current = currentProfileName(profiles, info?.email);
  // The active account always sits on top; the rest keep their file order.
  const ordered = current
    ? [
        ...profiles.filter((p) => p.name === current),
        ...profiles.filter((p) => p.name !== current),
      ]
    : profiles;
  const orderKey = ordered.map((p) => p.name).join("\n");

  // FLIP: when the order changes, start each card at its old position and
  // glide it to the new one. Positions are recorded after every render so
  // usage loading (which changes card heights) never leaves stale offsets.
  useLayoutEffect(() => {
    const tops = new Map<string, number>();
    cardEls.current.forEach((el, name) =>
      tops.set(name, el.getBoundingClientRect().top)
    );
    const reordered =
      lastOrder.current !== "" && lastOrder.current !== orderKey;
    const reduceMotion = window.matchMedia(
      "(prefers-reduced-motion: reduce)"
    ).matches;
    if (reordered && !reduceMotion) {
      listRef.current?.scrollTo({ top: 0, behavior: "smooth" });
      cardEls.current.forEach((el, name) => {
        const prev = cardTops.current.get(name);
        const now = tops.get(name);
        if (prev === undefined || now === undefined) return;
        const dy = prev - now;
        if (Math.abs(dy) < 1) return;
        // Cards moving up pass over the ones moving down.
        el.style.zIndex = dy > 0 ? "2" : "1";
        el.animate(
          [
            { transform: `translateY(${dy}px)` },
            { transform: "translateY(0)" },
          ],
          { duration: REORDER_MS, easing: REORDER_EASING }
        ).onfinish = () => {
          el.style.zIndex = "";
        };
      });
    }
    cardTops.current = tops;
    lastOrder.current = orderKey;
  });

  return (
    <>
      <TopBar onAdd={() => setShowAddModal(true)} />

      <div className="list-container" ref={listRef}>
        {profiles.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-20 text-text3">
            <div className="w-14 h-14 rounded-2xl bg-surface-light flex items-center justify-center mb-4 ring-1 ring-border">
              <svg
                className="w-7 h-7 opacity-40"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={1.5}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M15.75 6a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0ZM4.501 20.118a7.5 7.5 0 0 1 14.998 0A17.933 17.933 0 0 1 12 21.75c-2.676 0-5.216-.584-7.499-1.632Z"
                />
              </svg>
            </div>
            <span className="text-[14px] text-center leading-relaxed">
              No profiles yet.
              <br />
              Click <strong className="text-text">+</strong> to add an account.
            </span>
          </div>
        ) : (
          ordered.map((p) => (
            <div
              key={p.name}
              className="relative"
              ref={(el) => {
                if (el) cardEls.current.set(p.name, el);
                else cardEls.current.delete(p.name);
              }}
            >
              <ProfileRow
                profile={p}
                isCurrent={p.name === current}
                usage={usage[p.name]}
                onSwitch={() => handleSwitch(p.name)}
                onRefresh={(force) => handleRefresh(p.name, force)}
                refreshing={refreshing.has(p.name)}
                onUsage={() => handleUsage(p.name)}
                onDelete={() => setConfirmTarget(p.name)}
              />
            </div>
          ))
        )}
      </div>

      <AddModal
        open={showAddModal}
        onClose={() => setShowAddModal(false)}
        onReload={handleReload}
        onBackup={handleBackup}
      />

      <UsageModal
        open={!!usageTarget}
        profileName={usageTarget ?? ""}
        loading={usageLoading}
        error={usageError}
        data={usageData}
        onClose={() => setUsageTarget(null)}
      />

      <ConfirmDialog
        open={!!confirmTarget}
        title="Delete Profile"
        message={`Are you sure you want to delete "${confirmTarget}"?`}
        onConfirm={handleDelete}
        onCancel={() => setConfirmTarget(null)}
      />

      <Toast toasts={toasts} onRemove={removeToast} />
      <VersionLabel />
    </>
  );
}
