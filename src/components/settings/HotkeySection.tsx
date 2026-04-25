import { useEffect, useRef, useState } from "react";
import { Keyboard } from "lucide-react";
import type { Settings } from "./types";
import { Block, Field, PageHero } from "./primitives";

const MODIFIER_KEYS = new Set([
  "Control",
  "Shift",
  "Alt",
  "AltGraph",
  "Meta",
  "OS",
]);

function formatAccelerator(e: KeyboardEvent): string | null {
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("CmdOrCtrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (MODIFIER_KEYS.has(e.key)) return null;

  let key: string;
  if (e.key === " " || e.code === "Space") key = "Space";
  else if (e.key.length === 1) key = e.key.toUpperCase();
  else if (/^F\d{1,2}$/.test(e.key)) key = e.key;
  else if (e.key.startsWith("Arrow")) key = e.key.replace("Arrow", "");
  else key = e.key;

  if (parts.length === 0) return null;
  parts.push(key);
  return parts.join("+");
}

function Chips({ combo }: { combo: string }) {
  return (
    <div className="flex items-center gap-1.5">
      {combo.split("+").map((k, i, arr) => (
        <span key={i} className="inline-flex items-center">
          <kbd className="inline-flex items-center justify-center min-w-[28px] h-[28px] px-2 rounded-md bg-white/[0.04] border border-white/[0.1] text-[11px] font-mono text-white">
            {k}
          </kbd>
          {i < arr.length - 1 && (
            <span className="mx-1 text-zinc-600 text-[11px]">+</span>
          )}
        </span>
      ))}
    </div>
  );
}

interface Props {
  settings: Settings;
  setSettings: (s: Settings) => void;
}

export function HotkeySection({ settings, setSettings }: Props) {
  const [capturing, setCapturing] = useState(false);
  const captureRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!capturing) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setCapturing(false);
        return;
      }
      const accel = formatAccelerator(e);
      if (!accel) return;
      setSettings({ ...settings, hotkey: accel });
      setCapturing(false);
    };
    window.addEventListener("keydown", handler, { capture: true });
    captureRef.current?.focus();
    return () =>
      window.removeEventListener("keydown", handler, { capture: true });
  }, [capturing, settings, setSettings]);

  return (
    <div>
      <PageHero
        eyebrow="Trigger"
        title="Hotkey"
        description="The system-wide key combination that starts and stops dictation. Hit save to apply changes — no restart needed."
        Icon={Keyboard}
      />

      <Block>
        <Field label="Combination">
          <div
            ref={captureRef}
            tabIndex={0}
            role="button"
            onClick={() => setCapturing(true)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") setCapturing(true);
            }}
            className={`flex items-center justify-between rounded-xl px-4 py-4 cursor-pointer transition-all focus:outline-none border ${
              capturing
                ? "bg-white/[0.04] border-white/30 shadow-[0_0_24px_-6px_rgba(255,255,255,0.18)]"
                : "bg-transparent border-white/[0.08] hover:border-white/20"
            }`}
          >
            {capturing ? (
              <div className="flex items-center gap-2.5">
                <span className="relative flex w-2 h-2">
                  <span className="absolute inset-0 rounded-full bg-white/40 animate-ping" />
                  <span className="relative w-2 h-2 rounded-full bg-white" />
                </span>
                <span className="text-[12.5px] text-zinc-200">
                  Press your combination… Esc to cancel
                </span>
              </div>
            ) : (
              <Chips combo={settings.hotkey} />
            )}
            <span className="text-[10.5px] uppercase tracking-[0.14em] text-zinc-600">
              Click to {capturing ? "cancel" : "rebind"}
            </span>
          </div>
        </Field>
      </Block>
    </div>
  );
}
