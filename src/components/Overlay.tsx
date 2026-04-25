import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

type RecordingState = "recording" | "transcribing" | "done" | "error";

const STATIC_LABELS: Record<Exclude<RecordingState, "error">, string> = {
  recording: "Recording… press hotkey to stop",
  transcribing: "Transcribing…",
  done: "Done",
};

const DOT_CLASS: Record<RecordingState, string> = {
  recording: "bg-red-500 animate-pulse",
  transcribing: "bg-yellow-400 animate-bounce",
  done: "bg-green-400",
  error: "bg-red-600",
};

export function Overlay() {
  const [state, setState] = useState<RecordingState | null>(null);
  const [errorMsg, setErrorMsg] = useState<string>("");
  const [transcription, setTranscription] = useState<string>("");

  useEffect(() => {
    let clearTimer: ReturnType<typeof setTimeout> | null = null;
    const reset = (ms: number) => {
      if (clearTimer) clearTimeout(clearTimer);
      clearTimer = setTimeout(() => {
        setState(null);
        setErrorMsg("");
        setTranscription("");
      }, ms);
    };

    const unsubs = [
      listen<string>("recording-state", (e) => {
        const s = e.payload as RecordingState;
        if (!(s in DOT_CLASS) || s === "error") return;
        if (clearTimer) clearTimeout(clearTimer);
        setState(s);
        setErrorMsg("");
        if (s !== "done") setTranscription("");
        if (s === "done") reset(4000);
      }),
      listen<string>("recording-transcription", (e) => {
        if (typeof e.payload === "string") setTranscription(e.payload);
      }),
      listen<string>("recording-error", (e) => {
        setState("error");
        setErrorMsg(typeof e.payload === "string" ? e.payload : "Unknown error");
        setTranscription("");
        reset(6000);
      }),
    ];

    return () => {
      if (clearTimer) clearTimeout(clearTimer);
      unsubs.forEach((p) => p.then((f) => f()));
    };
  }, []);

  if (!state) return null;

  let label: string;
  if (state === "error") label = errorMsg || "Error";
  else if (state === "done" && transcription) label = transcription;
  else label = STATIC_LABELS[state as Exclude<RecordingState, "error">];

  return (
    <div
      className="flex items-start gap-2.5 px-4 py-2 h-full w-full rounded-xl bg-zinc-900/95 backdrop-blur-sm shadow-2xl border border-white/10 text-white text-xs select-text cursor-default overflow-hidden"
      title={label}
    >
      <span
        className={`w-2.5 h-2.5 rounded-full flex-shrink-0 mt-0.5 ${DOT_CLASS[state]}`}
      />
      <span className="line-clamp-3 opacity-90 leading-snug">{label}</span>
    </div>
  );
}
