import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Mic2,
  Keyboard,
  Sparkles,
  Info,
  KeyRound,
  Search,
  ExternalLink,
  LifeBuoy,
  LogOut,
  History as HistoryIcon,
} from "lucide-react";
import type { SectionId, Settings } from "./types";

interface NavItem {
  id: SectionId;
  label: string;
  Icon: typeof Mic2;
}

const PRIMARY: NavItem[] = [
  { id: "api-key", label: "API key", Icon: KeyRound },
  { id: "audio", label: "Audio", Icon: Mic2 },
  { id: "hotkey", label: "Hotkey", Icon: Keyboard },
  { id: "recording", label: "Recording", Icon: Sparkles },
  { id: "history", label: "History", Icon: HistoryIcon },
  { id: "about", label: "About", Icon: Info },
];

interface Props {
  current: SectionId;
  onSelect: (id: SectionId) => void;
  settings: Settings;
  query: string;
  setQuery: (v: string) => void;
}

export function Sidebar({
  current,
  onSelect,
  settings,
  query,
  setQuery,
}: Props) {
  const apiKeyOk = settings.groqApiKey.trim().length > 0;
  const searchRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "/" || e.ctrlKey || e.altKey || e.metaKey) return;
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName;
      if (
        tag === "INPUT" ||
        tag === "TEXTAREA" ||
        tag === "SELECT" ||
        target?.isContentEditable
      ) {
        return;
      }
      e.preventDefault();
      searchRef.current?.focus();
      searchRef.current?.select();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <aside className="relative w-[260px] shrink-0 flex flex-col bg-black border-r border-white/[0.06]">
      <BrandBlock />
      <div className="px-4 mt-1">
        <SearchField inputRef={searchRef} value={query} onChange={setQuery} />
      </div>

      <div className="px-3 mt-4">
        <NavGroup>
          {PRIMARY.map((item) => (
            <NavRow
              key={item.id}
              {...item}
              active={current === item.id}
              onClick={() => onSelect(item.id)}
              warn={item.id === "api-key" && !apiKeyOk}
            />
          ))}
        </NavGroup>
      </div>

      <Divider />
      <Eyebrow>Resources</Eyebrow>
      <div className="px-3">
        <NavGroup>
          <ExternalRow
            label="Get an API key"
            Icon={ExternalLink}
            url="https://console.groq.com"
          />
          <ExternalRow
            label="Support"
            Icon={LifeBuoy}
            url="https://github.com/Shravanth007/AirWrite/issues"
          />
        </NavGroup>
      </div>

      <div className="flex-1" />

      <ExitButton />
    </aside>
  );
}

function BrandBlock() {
  return (
    <div className="px-5 pt-5 pb-4 flex items-center gap-2.5">
      <div className="w-7 h-7 rounded-lg bg-white flex items-center justify-center">
        <svg
          viewBox="0 0 24 24"
          fill="none"
          className="w-4 h-4 text-black"
          stroke="currentColor"
          strokeWidth="2.6"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M3 12c2 0 2-4 4-4s2 8 4 8 2-12 4-12 2 16 4 16 2-4 4-4" />
        </svg>
      </div>
      <div className="leading-tight">
        <div className="text-[14px] font-semibold tracking-tight text-zinc-50">
          AirWrite
        </div>
        <div className="text-[10px] text-zinc-600 font-mono mt-0.5">
          v0.1.0
        </div>
      </div>
    </div>
  );
}

function SearchField({
  inputRef,
  value,
  onChange,
}: {
  inputRef?: React.Ref<HTMLInputElement>;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div className="relative">
      <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-zinc-600" />
      <input
        ref={inputRef}
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Search settings"
        className="w-full bg-transparent border border-white/[0.08] rounded-lg pl-8 pr-12 py-2 text-[12.5px] text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-white/20 transition-colors"
      />
      <kbd className="absolute right-2 top-1/2 -translate-y-1/2 inline-flex items-center justify-center min-w-[18px] h-[18px] px-1 rounded-md border border-white/[0.08] text-[10px] font-mono text-zinc-500">
        /
      </kbd>
    </div>
  );
}

function NavGroup({ children }: { children: React.ReactNode }) {
  return <div className="space-y-0.5">{children}</div>;
}

function NavRow({
  label,
  Icon,
  active,
  onClick,
  warn,
}: NavItem & { active: boolean; onClick: () => void; warn?: boolean }) {
  return (
    <button
      onClick={onClick}
      className={`group relative w-full flex items-center gap-2.5 pl-2.5 pr-2 py-2 rounded-lg text-[12.5px] transition-all ${
        active
          ? "text-white bg-white/[0.04] shadow-[inset_0_0_0_1px_rgba(255,255,255,0.1),0_0_24px_-6px_rgba(255,255,255,0.18)]"
          : "text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.025]"
      }`}
    >
      <Icon
        className={`w-4 h-4 ${active ? "text-white" : "text-zinc-500 group-hover:text-zinc-300"}`}
        strokeWidth={2}
      />
      <span className="flex-1 text-left">{label}</span>
      {warn && (
        <span
          className="w-1.5 h-1.5 rounded-full bg-white/80"
          title="API key not set"
        />
      )}
    </button>
  );
}

function ExternalRow({
  label,
  Icon,
  url,
}: {
  label: string;
  Icon: typeof Mic2;
  url: string;
}) {
  return (
    <a
      href={url}
      target="_blank"
      rel="noreferrer"
      className="group w-full flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-[12.5px] text-zinc-500 hover:text-zinc-200 hover:bg-white/[0.025] transition-colors"
    >
      <Icon
        className="w-3.5 h-3.5 text-zinc-600 group-hover:text-zinc-400"
        strokeWidth={2}
      />
      <span className="flex-1 text-left">{label}</span>
      <ExternalLink className="w-3 h-3 text-zinc-700 group-hover:text-zinc-500" />
    </a>
  );
}

function Divider() {
  return <div className="mx-5 my-4 h-px bg-white/[0.06]" />;
}

function Eyebrow({ children }: { children: React.ReactNode }) {
  return (
    <div className="px-5 mb-2 text-[10px] font-semibold uppercase tracking-[0.18em] text-zinc-600">
      {children}
    </div>
  );
}

function ExitButton() {
  return (
    <div className="px-3 pb-3 pt-3 border-t border-white/[0.06]">
      <button
        onClick={() => invoke("quit").catch(() => {})}
        className="group w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-[12.5px] text-zinc-400 border border-white/[0.08] hover:text-white hover:border-white/20 hover:bg-white/[0.03] transition-all"
      >
        <LogOut className="w-3.5 h-3.5 text-zinc-500 group-hover:text-white" strokeWidth={2} />
        <span>Quit AirWrite</span>
      </button>
    </div>
  );
}
