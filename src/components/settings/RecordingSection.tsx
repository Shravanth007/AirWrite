import { Repeat, Hand, Sparkles } from "lucide-react";
import type { Settings, RecordingMode } from "./types";
import { PageHero, Pill } from "./primitives";

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
    enabled: false,
    badge: "Soon",
  },
];

export function RecordingSection({ settings, setSettings }: Props) {
  return (
    <div>
      <PageHero
        eyebrow="Behavior"
        title="Recording mode"
        description="How the hotkey controls capture. Push-to-talk is wired into the data model and will land in the next release."
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
    </div>
  );
}
