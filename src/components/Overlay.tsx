import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

type RecordingState = "recording" | "transcribing" | "done" | "error";

const STATE_CONFIG: Record<RecordingState, { dot: string; label: string }> = {
  recording: {
    dot: "bg-red-500 animate-pulse",
    label: "Recording… press hotkey to stop",
  },
  transcribing: {
    dot: "bg-yellow-400 animate-bounce",
    label: "Transcribing…",
  },
  done: {
    dot: "bg-green-400",
    label: "Done",
  },
  error: {
    dot: "bg-red-600",
    label: "Error — check API key in settings",
  },
};

export function Overlay() {
  const [state, setState] = useState<RecordingState>("recording");

  useEffect(() => {
    const unsubs = [
      listen<string>("recording-state", (e) => {
        const s = e.payload as RecordingState;
        if (s in STATE_CONFIG) setState(s);
      }),
      listen<string>("recording-error", () => setState("error")),
    ];
    return () => {
      unsubs.forEach((p) => p.then((f) => f()));
    };
  }, []);

  const { dot, label } = STATE_CONFIG[state];

  return (
    <div className="flex items-center gap-2.5 px-4 h-full w-full rounded-xl bg-zinc-900/90 backdrop-blur-sm shadow-2xl border border-white/10 text-white text-xs select-none cursor-default overflow-hidden">
      <span className={`w-2.5 h-2.5 rounded-full flex-shrink-0 ${dot}`} />
      <span className="truncate opacity-90">{label}</span>
    </div>
  );
}
