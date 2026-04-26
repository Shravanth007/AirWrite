import { useEffect, useRef, useState } from "react";
import { Keyboard, Mic, Settings as SettingsIcon } from "lucide-react";
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

// Map a `KeyboardEvent.code` to the keycode token Tauri's global-shortcut
// parser accepts. Tauri uses keyboard-types `Code` names (Comma, Period,
// Slash, ...) rather than printable characters, so capturing `e.key` directly
// produces strings like `,` that fail to register at runtime.
function codeToTauri(code: string, key: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3); // KeyA -> A
  if (/^Digit\d$/.test(code)) return code.slice(5); // Digit5 -> 5
  if (/^F\d{1,2}$/.test(code)) return code; // F1..F12
  if (code === "Space") return "Space";
  if (code === "Enter") return "Enter";
  if (code === "Tab") return "Tab";
  if (code === "Backspace") return "Backspace";
  if (code.startsWith("Arrow")) return code.replace("Arrow", "");
  // Numpad and punctuation come through as their Code name verbatim
  // (Comma, Period, Slash, Semicolon, Quote, Backquote, Minus, Equal,
  //  BracketLeft, BracketRight, Backslash, Numpad0..9).
  if (
    /^(Comma|Period|Slash|Semicolon|Quote|Backquote|Minus|Equal|BracketLeft|BracketRight|Backslash|IntlBackslash|Numpad\d|NumpadAdd|NumpadSubtract|NumpadMultiply|NumpadDivide|NumpadDecimal|NumpadEnter|PageUp|PageDown|Home|End|Insert|Delete)$/.test(
      code,
    )
  ) {
    return code;
  }
  // Fallback: if `key` is a single printable letter, accept it.
  if (key.length === 1 && /[A-Za-z]/.test(key)) return key.toUpperCase();
  return null;
}

function formatAccelerator(e: KeyboardEvent): string | null {
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("CmdOrCtrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (MODIFIER_KEYS.has(e.key)) return null;

  const key = codeToTauri(e.code, e.key);
  if (!key) return null;
  if (parts.length === 0) return null;
  parts.push(key);
  return parts.join("+");
}

// Prettify Tauri code names for display in the chips (storage stays raw).
const KEY_DISPLAY: Record<string, string> = {
  CmdOrCtrl: "Ctrl",
  Cmd: "Ctrl",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Semicolon: ";",
  Quote: "'",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Up: "↑",
  Down: "↓",
  Left: "←",
  Right: "→",
};
const displayKey = (k: string) => KEY_DISPLAY[k] ?? k;

function Chips({ combo }: { combo: string }) {
  return (
    <div className="flex items-center gap-1.5">
      {combo.split("+").map((k, i, arr) => (
        <span key={i} className="inline-flex items-center">
          <kbd className="inline-flex items-center justify-center min-w-[28px] h-[28px] px-2 rounded-md bg-white/[0.04] border border-white/[0.1] text-[11px] font-mono text-white">
            {displayKey(k)}
          </kbd>
          {i < arr.length - 1 && (
            <span className="mx-1 text-zinc-600 text-[11px]">+</span>
          )}
        </span>
      ))}
    </div>
  );
}

type CaptureTarget = "recording" | "settings" | null;

interface BindingRowProps {
  Icon: typeof Mic;
  label: string;
  hint: string;
  combo: string;
  capturing: boolean;
  onClick: () => void;
}

function BindingRow({
  Icon,
  label,
  hint,
  combo,
  capturing,
  onClick,
}: BindingRowProps) {
  return (
    <div className="flex items-start gap-3">
      <div className="w-9 h-9 rounded-lg border border-white/[0.08] flex items-center justify-center shrink-0 mt-0.5">
        <Icon className="w-4 h-4 text-zinc-400" strokeWidth={2} />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-[13px] font-medium text-white">{label}</p>
        <p className="text-[11px] text-zinc-500 mb-2.5">{hint}</p>
        <div
          tabIndex={0}
          role="button"
          onClick={onClick}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") onClick();
          }}
          className={`flex items-center justify-between rounded-xl px-4 py-3.5 cursor-pointer transition-all focus:outline-none border ${
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
            <Chips combo={combo} />
          )}
          <span className="text-[10.5px] uppercase tracking-[0.14em] text-zinc-600">
            Click to {capturing ? "cancel" : "rebind"}
          </span>
        </div>
      </div>
    </div>
  );
}

interface Props {
  settings: Settings;
  setSettings: (s: Settings) => void;
}

export function HotkeySection({ settings, setSettings }: Props) {
  const [capturing, setCapturing] = useState<CaptureTarget>(null);
  const captureRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!capturing) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setCapturing(null);
        return;
      }
      const accel = formatAccelerator(e);
      if (!accel) return;
      if (capturing === "recording") {
        setSettings({ ...settings, hotkey: accel });
      } else if (capturing === "settings") {
        setSettings({ ...settings, settingsHotkey: accel });
      }
      setCapturing(null);
    };
    window.addEventListener("keydown", handler, { capture: true });
    captureRef.current?.focus();
    return () =>
      window.removeEventListener("keydown", handler, { capture: true });
  }, [capturing, settings, setSettings]);

  // Visual warning if both fields end up bound to the same combo.
  const conflict =
    settings.hotkey &&
    settings.settingsHotkey &&
    settings.hotkey === settings.settingsHotkey;

  return (
    <div ref={captureRef}>
      <PageHero
        eyebrow="Triggers"
        title="Hotkeys"
        description="System-wide key combinations that work from any app. Save to apply changes — no restart needed."
        Icon={Keyboard}
      />

      <Block>
        <Field label="Bindings">
          <div className="space-y-6 pt-1">
            <BindingRow
              Icon={Mic}
              label="Start / stop recording"
              hint="Begins dictation. In push-to-talk mode, hold this key while you speak."
              combo={settings.hotkey}
              capturing={capturing === "recording"}
              onClick={() =>
                setCapturing(capturing === "recording" ? null : "recording")
              }
            />
            <BindingRow
              Icon={SettingsIcon}
              label="Open settings"
              hint="Brings this window forward from anywhere on your desktop."
              combo={settings.settingsHotkey}
              capturing={capturing === "settings"}
              onClick={() =>
                setCapturing(capturing === "settings" ? null : "settings")
              }
            />
          </div>
        </Field>

        {conflict && (
          <p className="mt-4 text-[11.5px] text-amber-400">
            Both hotkeys are bound to the same combination — assign different keys.
          </p>
        )}
      </Block>
    </div>
  );
}
