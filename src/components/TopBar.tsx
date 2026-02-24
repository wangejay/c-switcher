interface Props {
  onAdd: () => void;
}

export default function TopBar({ onAdd }: Props) {
  return (
    <div
      className="topbar relative flex items-center justify-between shrink-0 select-none bg-bg"
      data-tauri-drag-region
    >
      <span
        className="text-[16px] font-bold tracking-[-0.01em]"
        data-tauri-drag-region
      >
        C-Switcher
      </span>
      <button
        className="w-8 h-8 rounded-full bg-gradient-to-br from-orange-400 to-orange-600 text-white flex items-center justify-center shadow-lg shadow-orange-500/30 transition-all duration-200 hover:shadow-xl hover:shadow-orange-500/40 hover:scale-105 active:scale-95 ring-1 ring-white/15"
        onClick={onAdd}
        title="Add account"
      >
        <svg
          className="w-4 h-4"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={2.5}
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M12 4.5v15m7.5-7.5h-15"
          />
        </svg>
      </button>
      {/* Bottom border */}
      <div className="absolute inset-x-0 bottom-0 h-px bg-gradient-to-r from-transparent via-border2 to-transparent" />
    </div>
  );
}
