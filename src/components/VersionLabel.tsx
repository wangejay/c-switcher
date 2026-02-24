import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

export default function VersionLabel() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    getVersion()
      .then((v) => setVersion(v))
      .catch(() => {});
  }, []);

  if (!version) return null;

  return (
    <span className="fixed bottom-2 right-3 text-[11px] text-text3 opacity-60 pointer-events-none z-10">
      v{version}
    </span>
  );
}
