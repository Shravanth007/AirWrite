import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { AlertTriangle, X } from "lucide-react";

/**
 * Persistent, dismissable red banner that pins to the top of the Settings
 * main pane. Catches `recording-error` events while the window is open and
 * auto-clears on the next successful `recording-state: done` (so a hiccup
 * that sorts itself out doesn't linger).
 */
export function ErrorBanner() {
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    const unsubs = [
      listen<string>("recording-error", (e) => {
        if (typeof e.payload === "string" && e.payload.trim()) {
          setMessage(e.payload);
        }
      }),
      listen<string>("recording-state", (e) => {
        if (e.payload === "done") setMessage(null);
      }),
    ];
    return () => {
      unsubs.forEach((p) => p.then((f) => f()));
    };
  }, []);

  if (!message) return null;

  return (
    <div className="relative flex items-start gap-3 px-5 py-3 bg-black border-b border-white/[0.06]">
      <div className="absolute inset-y-0 left-0 w-[3px] bg-red-500/80" />
      <div className="w-7 h-7 shrink-0 rounded-md bg-red-500/10 border border-red-500/30 flex items-center justify-center mt-0.5">
        <AlertTriangle className="w-3.5 h-3.5 text-red-400" strokeWidth={2.4} />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-[10.5px] font-semibold uppercase tracking-[0.16em] text-red-400 mb-0.5">
          Something went wrong
        </p>
        <p className="text-[12.5px] text-zinc-200 leading-snug">{message}</p>
      </div>
      <button
        onClick={() => setMessage(null)}
        className="shrink-0 w-6 h-6 rounded-md flex items-center justify-center text-zinc-500 hover:text-white hover:bg-white/[0.04] transition-colors"
        aria-label="Dismiss"
        title="Dismiss"
      >
        <X className="w-3.5 h-3.5" strokeWidth={2} />
      </button>
    </div>
  );
}
