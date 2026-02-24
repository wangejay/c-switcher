import { useState } from "react";

interface Props {
  open: boolean;
  onClose: () => void;
  onReload: () => void;
  onBackup: (name: string) => void;
}

export default function AddModal({
  open,
  onClose,
  onReload,
  onBackup,
}: Props) {
  const [name, setName] = useState("");

  if (!open) return null;

  const handleBackup = () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    onBackup(trimmed);
    setName("");
    onClose();
  };

  return (
    <div
      className="dialog-overlay fixed inset-0 bg-black/60 backdrop-blur-sm z-[9998] flex items-center justify-center"
      onClick={onClose}
    >
      <div
        className="dialog-content modal-panel bg-surface border border-border2 max-w-[420px] w-[90%] shadow-2xl shadow-black/50 relative"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Close button */}
        <button
          className="absolute top-4 right-4 w-7 h-7 rounded-lg flex items-center justify-center text-text3 hover:text-text hover:bg-surface-lighter transition-all"
          onClick={onClose}
        >
          <svg
            className="w-4 h-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M6 18 18 6M6 6l12 12"
            />
          </svg>
        </button>

        {/* Title */}
        <h3 className="text-[16px] font-semibold mb-lg">Add New Account</h3>

        {/* Steps */}
        <div className="steps-container relative">
          <div className="absolute left-[10px] top-[20px] bottom-[20px] w-px bg-gradient-to-b from-accent2/25 via-accent/20 to-success/25" />
          <Step n={1} color="bg-accent2">
            Open Claude Code in terminal, type{" "}
            <code className="font-mono text-[14px] text-accent bg-accent/8 px-1.5 py-0.5 rounded-md ring-1 ring-accent/10">
              /login
            </code>{" "}
            and login with the new account
          </Step>
          <Step n={2} color="bg-accent">
            Come back here and click{" "}
            <strong className="text-text font-semibold">
              Reload Current Account
            </strong>{" "}
            below
          </Step>
          <Step n={3} color="bg-success">
            Enter a profile name and click{" "}
            <strong className="text-text font-semibold">Backup</strong> to save
            it
          </Step>
        </div>

        {/* Reload button */}
        <button
          className="btn bg-surface-light text-text ring-1 ring-border2 hover:bg-surface-lighter hover:ring-white/18 mb-md w-full justify-center"
          onClick={onReload}
        >
          <svg
            className="w-3.5 h-3.5 text-success"
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
          Reload Current Account
        </button>

        {/* Backup input */}
        <div className="flex gap-sm items-center">
          <input
            className="input-field flex-1"
            placeholder="Profile name (e.g. jennifer)"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleBackup()}
          />
          <button
            className="btn bg-gradient-to-r from-accent to-accent-pink text-white shadow-lg shadow-accent/25 hover:shadow-xl hover:shadow-accent/35 hover:brightness-110 disabled:shadow-none disabled:hover:brightness-100"
            onClick={handleBackup}
            disabled={!name.trim()}
          >
            Backup
          </button>
        </div>
      </div>
    </div>
  );
}

function Step({
  n,
  color,
  children,
}: {
  n: number;
  color: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex gap-3 items-start text-[14px] text-text2 leading-relaxed relative z-10">
      <div
        className={`w-[22px] h-[22px] rounded-full ${color} text-white flex items-center justify-center text-[14px] font-bold shrink-0 shadow-md ring-2 ring-surface`}
      >
        {n}
      </div>
      <div className="pt-[2px]">{children}</div>
    </div>
  );
}
