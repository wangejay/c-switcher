interface Props {
  open: boolean;
  title: string;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function ConfirmDialog({
  open,
  title,
  message,
  onConfirm,
  onCancel,
}: Props) {
  if (!open) return null;

  return (
    <div
      className="dialog-overlay fixed inset-0 bg-black/60 backdrop-blur-sm z-[9998] flex items-center justify-center"
      onClick={onCancel}
    >
      <div
        className="dialog-content modal-panel bg-surface border border-border2 max-w-[400px] w-[90%] shadow-2xl shadow-black/50"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="w-10 h-10 rounded-xl bg-danger/10 flex items-center justify-center mb-md ring-1 ring-danger/20">
          <svg
            className="w-5 h-5 text-danger"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z"
            />
          </svg>
        </div>
        <h3 className="text-[16px] font-semibold mb-sm">{title}</h3>
        <p className="text-text2 text-[15px] mb-lg leading-relaxed">
          {message}
        </p>
        <div className="flex gap-sm justify-end">
          <button
            className="btn bg-surface-light text-text ring-1 ring-border2 hover:bg-surface-lighter"
            onClick={onCancel}
          >
            Cancel
          </button>
          <button
            className="btn bg-danger text-white shadow-lg shadow-danger/25 hover:brightness-110"
            onClick={onConfirm}
          >
            Delete
          </button>
        </div>
      </div>
    </div>
  );
}
