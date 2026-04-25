import { Mic2, Keyboard, Sparkles, Info, KeyRound } from "lucide-react";
import type { SectionId } from "./types";

interface NavItem {
  id: SectionId;
  label: string;
  Icon: typeof Mic2;
}

const ITEMS: NavItem[] = [
  { id: "api-key", label: "API key", Icon: KeyRound },
  { id: "audio", label: "Audio", Icon: Mic2 },
  { id: "hotkey", label: "Hotkey", Icon: Keyboard },
  { id: "recording", label: "Recording", Icon: Sparkles },
  { id: "about", label: "About", Icon: Info },
];

interface Props {
  current: SectionId;
  onSelect: (id: SectionId) => void;
  apiKeyOk: boolean;
}

export function Sidebar({ current, onSelect, apiKeyOk }: Props) {
  return (
    <aside className="w-48 shrink-0 bg-black border-r border-[var(--color-line)] flex flex-col">
      <div className="px-5 pt-6 pb-7">
        <div className="flex items-center gap-2.5">
          <div className="relative w-7 h-7 rounded-lg bg-gradient-to-br from-brand-300 via-brand-500 to-brand-600 flex items-center justify-center">
            <div className="absolute inset-0 rounded-lg bg-gradient-to-br from-white/30 to-transparent opacity-50" />
            <svg
              viewBox="0 0 24 24"
              fill="none"
              className="relative w-3.5 h-3.5 text-zinc-950"
              stroke="currentColor"
              strokeWidth="2.75"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M3 12c2 0 2-4 4-4s2 8 4 8 2-12 4-12 2 16 4 16 2-4 4-4" />
            </svg>
          </div>
          <div className="leading-tight">
            <div className="text-[13px] font-semibold tracking-tight text-zinc-50">
              AirWrite
            </div>
            <div className="text-[10px] text-zinc-600 font-mono">
              v0.1.0
            </div>
          </div>
        </div>
      </div>

      <nav className="px-2.5 flex-1 space-y-0.5">
        {ITEMS.map(({ id, label, Icon }) => {
          const active = id === current;
          const showWarn = id === "api-key" && !apiKeyOk;
          return (
            <button
              key={id}
              onClick={() => onSelect(id)}
              className={`group relative w-full flex items-center gap-2.5 pl-3 pr-2.5 py-2 rounded-lg text-[12.5px] transition-all ${
                active
                  ? "bg-[var(--color-surface-3)] text-zinc-50"
                  : "text-zinc-500 hover:text-zinc-200 hover:bg-[var(--color-surface-2)]"
              }`}
            >
              {active && (
                <span className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-4 rounded-r-full bg-brand-400" />
              )}
              <Icon
                className={`w-3.5 h-3.5 ${active ? "text-brand-400" : "text-zinc-600 group-hover:text-zinc-400"}`}
                strokeWidth={2}
              />
              <span className="flex-1 text-left">{label}</span>
              {showWarn && (
                <span
                  className="w-1.5 h-1.5 rounded-full bg-amber-400 shadow-[0_0_6px_rgba(251,191,36,0.6)]"
                  title="API key not set"
                />
              )}
            </button>
          );
        })}
      </nav>

      <div className="px-5 py-4 border-t border-[var(--color-line)] text-[10px] text-zinc-600 leading-relaxed">
        Local-only · Powered by Groq
      </div>
    </aside>
  );
}
