import { useCallback, useEffect, useState } from "react";
import type { CurrentInfo, ProfileEntry } from "./types";
import type { ToastItem, ToastType } from "./components/Toast";
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

export default function App() {
  const [info, setInfo] = useState<CurrentInfo | null>(null);
  const [profiles, setProfiles] = useState<ProfileEntry[]>([]);
  const [showAddModal, setShowAddModal] = useState(false);
  const [confirmTarget, setConfirmTarget] = useState<string | null>(null);
  const [usageTarget, setUsageTarget] = useState<string | null>(null);
  const [usageLoading, setUsageLoading] = useState(false);
  const [usageError, setUsageError] = useState<string | undefined>();
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [usageData, setUsageData] = useState<any>(undefined);
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const toast = useCallback((message: string, type: ToastType = "info") => {
    setToasts((prev) => [...prev, { id: ++toastId, message, type }]);
  }, []);

  const removeToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const loadCurrent = useCallback(async () => {
    const result = await getCurrentInfo();
    setInfo(result);
  }, []);

  const loadProfiles = useCallback(async () => {
    const result = await listProfiles();
    setProfiles(result);
  }, []);

  useEffect(() => {
    loadCurrent();
    loadProfiles();
  }, [loadCurrent, loadProfiles]);

  const handleBackup = async (name: string) => {
    const result = await backupProfile(name);
    if (result.success) {
      toast(`Backed up: ${result.email}`, "success");
      loadProfiles();
    } else {
      toast(result.error || "Backup failed", "error");
    }
  };

  const handleSwitch = async (name: string) => {
    const result = await switchProfile(name);
    if (result.success) {
      toast(`Switched: ${result.from} → ${result.to}`, "success");
      loadCurrent();
      loadProfiles();
    } else {
      toast(result.error || "Switch failed", "error");
    }
  };

  const handleRefresh = async (profileName: string | null) => {
    const label = profileName ?? "current";
    toast(`Refreshing ${label} token...`, "info");
    const result = await refreshToken(profileName);
    if (result.success) {
      toast(`Token refreshed! Expires: ${result.expiresAt}`, "success");
      loadCurrent();
      loadProfiles();
    } else {
      toast(result.error || "Refresh failed", "error");
    }
  };

  const handleDelete = async () => {
    if (!confirmTarget) return;
    const name = confirmTarget;
    setConfirmTarget(null);
    const result = await deleteProfile(name);
    if (result.success) {
      toast(result.message || "Deleted", "success");
      loadProfiles();
    } else {
      toast(result.error || "Delete failed", "error");
    }
  };

  const handleUsage = async (profileName: string) => {
    setUsageTarget(profileName);
    setUsageLoading(true);
    setUsageError(undefined);
    setUsageData(undefined);
    const result = await getUsage(profileName);
    setUsageLoading(false);
    if (result.success) {
      setUsageData(result.data);
    } else {
      setUsageError(result.error || "Failed to fetch usage");
    }
  };

  const handleReload = () => {
    loadCurrent();
    loadProfiles();
    toast("Reloaded!", "success");
  };

  return (
    <>
      <TopBar onAdd={() => setShowAddModal(true)} />

      <div className="list-container">
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
          (() => {
            let matched = false;
            return profiles.map((p) => {
              const isCurrent = !matched && p.email === info?.email;
              if (isCurrent) matched = true;
              return (
                <ProfileRow
                  key={p.name}
                  profile={p}
                  isCurrent={isCurrent}
                  onSwitch={() => handleSwitch(p.name)}
                  onRefresh={() => handleRefresh(p.name)}
                  onUsage={() => handleUsage(p.name)}
                  onDelete={() => setConfirmTarget(p.name)}
                />
              );
            });
          })()
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
