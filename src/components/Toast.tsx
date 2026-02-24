import { useEffect } from "react";

export type ToastType = "success" | "error" | "info";

export interface ToastItem {
  id: number;
  message: string;
  type: ToastType;
}

interface Props {
  toasts: ToastItem[];
  onRemove: (id: number) => void;
}

const styleMap: Record<
  ToastType,
  { bg: string; ring: string; icon: string; iconColor: string }
> = {
  success: {
    bg: "bg-[#0d2818]",
    ring: "ring-success/25",
    icon: "M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
    iconColor: "text-success",
  },
  error: {
    bg: "bg-[#2a0f0f]",
    ring: "ring-danger/25",
    icon: "M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z",
    iconColor: "text-danger",
  },
  info: {
    bg: "bg-surface-light",
    ring: "ring-white/10",
    icon: "m11.25 11.25.041-.02a.75.75 0 0 1 1.063.852l-.708 2.836a.75.75 0 0 0 1.063.853l.041-.021M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-3.75h.008v.008H12V8.25Z",
    iconColor: "text-text2",
  },
};

export default function Toast({ toasts, onRemove }: Props) {
  return (
    <div className="fixed bottom-4 right-4 z-[9999] flex flex-col gap-2 pointer-events-none">
      {toasts.map((t) => (
        <ToastEntry key={t.id} toast={t} onRemove={onRemove} />
      ))}
    </div>
  );
}

function ToastEntry({
  toast,
  onRemove,
}: {
  toast: ToastItem;
  onRemove: (id: number) => void;
}) {
  useEffect(() => {
    const timer = setTimeout(() => onRemove(toast.id), 3000);
    return () => clearTimeout(timer);
  }, [toast.id, onRemove]);

  const s = styleMap[toast.type];

  return (
    <div
      className={`toast-enter pointer-events-auto flex items-center gap-2.5 px-4 py-3 rounded-xl text-[15px] text-text ring-1 shadow-xl shadow-black/40 max-w-[360px] ${s.bg} ${s.ring}`}
    >
      <svg
        className={`w-4 h-4 shrink-0 ${s.iconColor}`}
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        strokeWidth={1.5}
      >
        <path strokeLinecap="round" strokeLinejoin="round" d={s.icon} />
      </svg>
      <span className="leading-snug">{toast.message}</span>
    </div>
  );
}
