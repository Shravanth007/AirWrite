import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Check, AlertTriangle, Loader2, Mic, MicOff } from "lucide-react";

type RecordingState =
  | "recording"
  | "transcribing"
  | "done"
  | "error"
  | "muted"
  | "unmuted";

const WAVE_DELAYS = [0, 90, 180, 90, 0];

function Waveform() {
  return (
    <div className="flex items-center gap-[2px] h-3">
      {WAVE_DELAYS.map((delay, i) => (
        <span
          key={i}
          className="w-[2px] h-full rounded-full bg-red-400 animate-wave"
          style={{ animationDelay: `${delay}ms` }}
        />
      ))}
    </div>
  );
}

export function Overlay() {
  const [state, setState] = useState<RecordingState | null>(null);
  const [errorMsg, setErrorMsg] = useState("");

  useEffect(() => {
    let clearTimer: ReturnType<typeof setTimeout> | null = null;
    const reset = (ms: number) => {
      if (clearTimer) clearTimeout(clearTimer);
      clearTimer = setTimeout(() => {
        setState(null);
        setErrorMsg("");
      }, ms);
    };

    const unsubs = [
      listen<string>("recording-state", (e) => {
        const s = e.payload as RecordingState;
        if (s !== "recording" && s !== "transcribing" && s !== "done") return;
        if (clearTimer) clearTimeout(clearTimer);
        setState(s);
        setErrorMsg("");
        if (s === "done") reset(900);
      }),
      listen<string>("recording-error", (e) => {
        setState("error");
        setErrorMsg(typeof e.payload === "string" ? e.payload : "Unknown error");
        reset(5000);
      }),
      listen<boolean>("mic-mute", (e) => {
        if (clearTimer) clearTimeout(clearTimer);
        setState(e.payload ? "muted" : "unmuted");
        setErrorMsg("");
        reset(1500);
      }),
    ];

    return () => {
      if (clearTimer) clearTimeout(clearTimer);
      unsubs.forEach((p) => p.then((f) => f()));
    };
  }, []);

  if (!state) return null;

  return (
    <div className="h-full w-full flex items-center justify-center p-1">
      <Pill state={state} errorMsg={errorMsg} />
    </div>
  );
}

function Pill({
  state,
  errorMsg,
}: {
  state: RecordingState;
  errorMsg: string;
}) {
  if (state === "recording") {
    return (
      <button
        type="button"
        className="group inline-flex items-center gap-2.5 h-9 pl-3 pr-3.5 rounded-full bg-black border border-zinc-800 shadow-[0_8px_24px_-6px_rgba(0,0,0,0.7),inset_0_1px_0_rgba(255,255,255,0.04)] animate-overlay-in cursor-default select-none"
      >
        <span className="relative flex w-2 h-2">
          <span className="absolute inset-0 rounded-full bg-red-500/50 animate-ping" />
          <span className="relative w-2 h-2 rounded-full bg-red-500" />
        </span>
        <span className="text-[11.5px] font-medium tracking-tight text-zinc-100">
          Recording
        </span>
        <span className="w-px h-3 bg-zinc-800" />
        <Waveform />
      </button>
    );
  }

  if (state === "transcribing") {
    return (
      <div className="relative inline-flex items-center gap-2 h-9 px-3.5 rounded-full bg-black border border-zinc-800 shadow-[0_8px_24px_-6px_rgba(0,0,0,0.7)] animate-overlay-in select-none overflow-hidden">
        <div className="absolute inset-0 animate-shimmer pointer-events-none" />
        <Loader2 className="relative w-3 h-3 text-brand-300 animate-spin" />
        <span className="relative text-[11.5px] font-medium text-zinc-300">
          Transcribing
        </span>
      </div>
    );
  }

  if (state === "done") {
    return (
      <div className="inline-flex items-center gap-2 h-9 px-3.5 rounded-full bg-black border border-emerald-500/30 shadow-[0_8px_24px_-6px_rgba(0,0,0,0.7),0_0_0_3px_rgba(16,185,129,0.08)] animate-overlay-in select-none">
        <Check className="w-3 h-3 text-emerald-400" strokeWidth={3} />
        <span className="text-[11.5px] font-medium text-emerald-300">
          Pasted
        </span>
      </div>
    );
  }

  if (state === "muted") {
    return (
      <div className="inline-flex items-center gap-2 h-9 px-3.5 rounded-full bg-black border border-red-500/40 shadow-[0_8px_24px_-6px_rgba(0,0,0,0.7),0_0_0_3px_rgba(239,68,68,0.08)] animate-overlay-in select-none">
        <MicOff className="w-3 h-3 text-red-400" strokeWidth={2.5} />
        <span className="text-[11.5px] font-medium text-red-200">Mic muted</span>
      </div>
    );
  }

  if (state === "unmuted") {
    return (
      <div className="inline-flex items-center gap-2 h-9 px-3.5 rounded-full bg-black border border-emerald-500/30 shadow-[0_8px_24px_-6px_rgba(0,0,0,0.7),0_0_0_3px_rgba(16,185,129,0.08)] animate-overlay-in select-none">
        <Mic className="w-3 h-3 text-emerald-400" strokeWidth={2.5} />
        <span className="text-[11.5px] font-medium text-emerald-300">Mic on</span>
      </div>
    );
  }

  return (
    <div
      className="inline-flex items-center gap-2 h-9 max-w-full px-3.5 rounded-full bg-black border border-red-500/40 shadow-[0_8px_24px_-6px_rgba(0,0,0,0.7),0_0_0_3px_rgba(239,68,68,0.08)] animate-overlay-in select-text overflow-hidden"
      title={errorMsg}
    >
      <AlertTriangle
        className="w-3 h-3 text-red-400 shrink-0"
        strokeWidth={2.5}
      />
      <span className="text-[11px] text-red-200 truncate">
        {errorMsg || "Something went wrong"}
      </span>
    </div>
  );
}
