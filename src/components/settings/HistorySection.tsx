import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { History as HistoryIcon, Clipboard, Trash2 } from "lucide-react";
import type { HistoryEntry } from "./types";
import { Block, Button, PageHero } from "./primitives";

const RELATIVE = new Intl.RelativeTimeFormat("en", { numeric: "auto" });

function formatAge(unixSecs: number): string {
  const diff = Math.floor(Date.now() / 1000) - unixSecs;
  if (diff < 5) return "just now";
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return RELATIVE.format(-Math.floor(diff / 60), "minute");
  if (diff < 86400) return RELATIVE.format(-Math.floor(diff / 3600), "hour");
  if (diff < 86400 * 7)
    return RELATIVE.format(-Math.floor(diff / 86400), "day");
  return new Date(unixSecs * 1000).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatDuration(secs: number): string {
  if (secs < 1) return `${Math.round(secs * 1000)}ms`;
  if (secs < 60) return `${secs.toFixed(1)}s`;
  const m = Math.floor(secs / 60);
  const s = Math.round(secs - m * 60);
  return `${m}m ${s}s`;
}

export function HistorySection() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loadError, setLoadError] = useState("");
  const [actionError, setActionError] = useState("");
  const [pastingIndex, setPastingIndex] = useState<number | null>(null);
  const [confirmingClear, setConfirmingClear] = useState(false);
  // Force re-render of relative timestamps every 30s so "just now" doesn't
  // get stuck while the panel sits open.
  const [, setNowTick] = useState(0);

  async function refresh() {
    try {
      const list = await invoke<HistoryEntry[]>("get_history");
      setEntries(list);
      setLoadError("");
    } catch (e) {
      setLoadError(String(e));
    }
  }

  useEffect(() => {
    refresh();
    const tick = setInterval(() => setNowTick((n) => n + 1), 30_000);
    const unsub = listen("history-updated", () => refresh());
    return () => {
      clearInterval(tick);
      unsub.then((f) => f());
    };
  }, []);

  async function paste(index: number) {
    setPastingIndex(index);
    setActionError("");
    try {
      await invoke("paste_history_entry", { index });
    } catch (e) {
      setActionError(String(e));
    } finally {
      // Brief delay so the user sees the "Pasting…" state register before
      // the row snaps back, even on a fast machine.
      setTimeout(() => setPastingIndex(null), 200);
    }
  }

  async function clearAll() {
    setActionError("");
    try {
      await invoke("clear_history");
      setEntries([]);
    } catch (e) {
      setActionError(String(e));
    } finally {
      setConfirmingClear(false);
    }
  }

  return (
    <div>
      <PageHero
        eyebrow="Recent"
        title="History"
        description="The last 20 successful dictations. Click any entry to re-paste it into the currently focused window — handy when paste landed in the wrong place."
        Icon={HistoryIcon}
      />

      {loadError && (
        <p className="text-[12px] text-red-400 mb-4">{loadError}</p>
      )}

      <Block>
        <div className="flex items-center justify-between mb-4">
          <span className="text-[10.5px] font-semibold uppercase tracking-[0.14em] text-zinc-500">
            {entries.length === 0
              ? "No entries yet"
              : `${entries.length} entr${entries.length === 1 ? "y" : "ies"}`}
          </span>
          {entries.length > 0 &&
            (confirmingClear ? (
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-zinc-500">Clear all?</span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setConfirmingClear(false)}
                >
                  Cancel
                </Button>
                <Button variant="danger" size="sm" onClick={clearAll}>
                  Confirm
                </Button>
              </div>
            ) : (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setConfirmingClear(true)}
              >
                <Trash2 className="w-3 h-3" />
                Clear
              </Button>
            ))}
        </div>

        {entries.length === 0 ? (
          <div className="rounded-xl border border-dashed border-white/[0.08] px-5 py-10 text-center">
            <p className="text-[12.5px] text-zinc-500">
              Once you've dictated something, it'll appear here.
            </p>
          </div>
        ) : (
          <ul className="space-y-2">
            {entries.map((entry, i) => (
              <HistoryRow
                key={`${entry.timestamp}-${i}`}
                entry={entry}
                pasting={pastingIndex === i}
                onPaste={() => paste(i)}
              />
            ))}
          </ul>
        )}

        {actionError && (
          <p className="mt-4 text-[11.5px] text-red-400">{actionError}</p>
        )}
      </Block>
    </div>
  );
}

function HistoryRow({
  entry,
  pasting,
  onPaste,
}: {
  entry: HistoryEntry;
  pasting: boolean;
  onPaste: () => void;
}) {
  return (
    <li>
      <button
        type="button"
        onClick={onPaste}
        disabled={pasting}
        className="group w-full text-left rounded-xl border border-white/[0.06] hover:border-white/15 bg-transparent hover:bg-white/[0.02] transition-all p-3.5 disabled:opacity-60 disabled:cursor-wait"
      >
        <div className="flex items-start gap-3">
          <div className="flex-1 min-w-0">
            <p className="text-[13px] text-zinc-100 leading-relaxed line-clamp-2 break-words">
              {entry.text}
            </p>
            <div className="mt-2 flex items-center gap-2 text-[10.5px] text-zinc-600 font-mono">
              <span>{formatAge(entry.timestamp)}</span>
              <span className="text-zinc-700">·</span>
              <span>
                {entry.word_count} {entry.word_count === 1 ? "word" : "words"}
              </span>
              <span className="text-zinc-700">·</span>
              <span>{formatDuration(entry.duration_secs)}</span>
            </div>
          </div>
          <div className="shrink-0 inline-flex items-center gap-1.5 text-[10.5px] uppercase tracking-[0.14em] text-zinc-600 group-hover:text-zinc-300 transition-colors">
            <Clipboard className="w-3 h-3" strokeWidth={2} />
            {pasting ? "Pasting" : "Re-paste"}
          </div>
        </div>
      </button>
    </li>
  );
}
