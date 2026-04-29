import { Repeat, Hand, Sparkles, Clipboard } from "lucide-react";
import type { Settings, RecordingMode } from "./types";
import { Block, Field, PageHero, Pill } from "./primitives";

interface Props {
  settings: Settings;
  setSettings: (s: Settings) => void;
}

interface ModeOption {
  id: RecordingMode;
  title: string;
  desc: string;
  Icon: typeof Repeat;
  enabled: boolean;
  badge?: string;
}

const MODES: ModeOption[] = [
  {
    id: "toggle",
    title: "Toggle",
    desc: "Press the hotkey once to start, again to stop. Best for longer dictation.",
    Icon: Repeat,
    enabled: true,
  },
  {
    id: "push_to_talk",
    title: "Push to talk",
    desc: "Hold the hotkey while speaking, release to transcribe. Best for quick bursts.",
    Icon: Hand,
    enabled: true,
  },
];

export function RecordingSection({ settings, setSettings }: Props) {
  return (
    <div>
      <PageHero
        eyebrow="Behavior"
        title="Recording mode"
        description="How the hotkey controls capture. Pick whatever feels natural — you can switch any time."
        Icon={Sparkles}
      />

      <div className="border-t border-white/[0.06] pt-6">
        <div className="space-y-2">
          {MODES.map((m) => {
            const active = settings.recordingMode === m.id;
            return (
              <button
                key={m.id}
                disabled={!m.enabled}
                onClick={() =>
                  m.enabled && setSettings({ ...settings, recordingMode: m.id })
                }
                className={`w-full text-left rounded-xl border transition-all ${
                  active
                    ? "border-white/25 bg-white/[0.03] shadow-[0_0_24px_-6px_rgba(255,255,255,0.18)]"
                    : "border-white/[0.08] bg-transparent hover:border-white/15"
                } ${m.enabled ? "cursor-pointer" : "opacity-50 cursor-not-allowed"}`}
              >
                <div className="p-4 flex items-start gap-4">
                  <div
                    className={`w-9 h-9 rounded-lg flex items-center justify-center shrink-0 border ${
                      active
                        ? "bg-white/[0.06] border-white/15"
                        : "bg-transparent border-white/[0.08]"
                    }`}
                  >
                    <m.Icon
                      className={`w-4 h-4 ${active ? "text-white" : "text-zinc-400"}`}
                    />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-0.5">
                      <h3 className="text-[13px] font-medium text-white">
                        {m.title}
                      </h3>
                      {m.badge && <Pill tone="soon">{m.badge}</Pill>}
                      {active && <Pill tone="ok">Active</Pill>}
                    </div>
                    <p className="text-[12px] text-zinc-500 leading-relaxed">
                      {m.desc}
                    </p>
                  </div>
                  <div
                    className={`w-4 h-4 rounded-full border-2 shrink-0 mt-1 transition-all ${
                      active
                        ? "border-white bg-white shadow-[0_0_0_3px_rgba(255,255,255,0.12)]"
                        : "border-white/15"
                    }`}
                  />
                </div>
              </button>
            );
          })}
        </div>
      </div>

      <Block className="mt-8">
        <Field label="Post-processing">
          <ToggleRow
            Icon={Clipboard}
            title="Restore previous clipboard"
            description="Snapshots whatever was on your clipboard before each dictation and puts it back a moment after pasting. Off = the v0.1 behaviour (clipboard stays overwritten)."
            checked={settings.clipboardRestore}
            onChange={(v) =>
              setSettings({ ...settings, clipboardRestore: v })
            }
          />
        </Field>
      </Block>
    </div>
  );
}

interface ToggleRowProps {
  Icon: typeof Clipboard;
  title: string;
  description: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  badge?: string;
}

function ToggleRow({
  Icon,
  title,
  description,
  checked,
  onChange,
  badge,
}: ToggleRowProps) {
  return (
    <div className="flex items-start gap-3 py-3 first:pt-1">
      <div className="w-9 h-9 rounded-lg border border-white/[0.08] flex items-center justify-center shrink-0 mt-0.5">
        <Icon className="w-4 h-4 text-zinc-400" strokeWidth={2} />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-0.5">
          <p className="text-[13px] font-medium text-white">{title}</p>
          {badge && <Pill tone="soon">{badge}</Pill>}
        </div>
        <p className="text-[11.5px] text-zinc-500 leading-relaxed pr-4">
          {description}
        </p>
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`relative inline-flex shrink-0 h-[22px] w-[38px] mt-1 rounded-full border transition-colors focus:outline-none ${
          checked
            ? "bg-white border-white"
            : "bg-transparent border-white/15 hover:border-white/30"
        }`}
      >
        <span
          className={`absolute top-[2px] w-[16px] h-[16px] rounded-full transition-all ${
            checked ? "left-[19px] bg-black" : "left-[2px] bg-zinc-400"
          }`}
        />
      </button>
    </div>
  );
}
