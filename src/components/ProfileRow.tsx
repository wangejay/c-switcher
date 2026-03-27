import type { ProfileEntry } from "../types";

interface Props {
  profile: ProfileEntry;
  isCurrent: boolean;
  onSwitch: () => void;
  onDelete: () => void;
  onUsage: () => void;
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

export default function ProfileRow({
  profile,
  isCurrent,
  onSwitch,
  onDelete,
  onUsage,
}: Props) {
  const colorClass = getAvatarColor(profile.name);
  const initial = profile.name[0]?.toUpperCase() ?? "?";

  return (
    <div
      className={`profile-card group relative flex items-center border transition-all duration-200 ${
        isCurrent
          ? "profile-card-active border-blue-500/30 bg-blue-500/[0.04]"
          : "border-border2 bg-surface hover:border-white/15 hover:bg-surface-light/50"
      }`}
    >
      {/* Inner top highlight */}
      <div className="absolute inset-x-0 top-0 h-px rounded-t-2xl bg-gradient-to-r from-transparent via-white/[0.06] to-transparent pointer-events-none" />

      {/* Avatar */}
      <div
        className={`avatar bg-gradient-to-br ${colorClass} flex items-center justify-center text-[16px] font-bold shrink-0 shadow-lg ring-1 ring-white/20`}
      >
        {initial}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2.5">
          <span className="text-[16px] font-semibold truncate leading-tight">
            {profile.name}
          </span>
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
        <div className="text-[14px] text-text2 truncate mt-1 leading-tight">
          {profile.email}
          {profile.organization && profile.organization !== "N/A" && (
            <>
              <span className="text-text3 mx-1.5">|</span>
              <span className="text-text3">{profile.organization}</span>
            </>
          )}
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-1 shrink-0">
        {isCurrent ? (
          <span className="badge-active text-[14px] font-bold bg-blue-500/15 text-blue-400 ring-1 ring-blue-400/25 tracking-wide">
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
  );
}
