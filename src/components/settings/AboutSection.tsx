import { Info, Cpu, ShieldCheck, Code2 } from "lucide-react";
import { Card, PageHero, Pill } from "./primitives";

export function AboutSection() {
  return (
    <div className="space-y-7">
      <PageHero
        eyebrow="Info"
        title="About"
        description="What's powering this and how your data flows."
        Icon={Info}
      />

      <Card className="p-5">
        <div className="flex items-start gap-4">
          <div className="w-10 h-10 rounded-xl bg-brand-500/10 border border-brand-500/30 flex items-center justify-center shrink-0">
            <Cpu className="w-4 h-4 text-brand-400" />
          </div>
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <p className="text-[13px] font-medium text-zinc-50">
                Speech model
              </p>
              <Pill tone="brand">Whisper v3 Turbo</Pill>
            </div>
            <p className="text-[12px] text-zinc-500 leading-relaxed">
              Audio is captured locally, downsampled to 16 kHz mono, and sent
              to{" "}
              <span className="font-mono text-zinc-300">api.groq.com</span>{" "}
              only when you finish a recording.
            </p>
          </div>
        </div>
      </Card>

      <Card className="p-5">
        <div className="flex items-start gap-4">
          <div className="w-10 h-10 rounded-xl bg-[var(--color-surface)] border border-[var(--color-line)] flex items-center justify-center shrink-0">
            <ShieldCheck className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="flex-1">
            <p className="text-[13px] font-medium text-zinc-50 mb-1">
              Local-first
            </p>
            <p className="text-[12px] text-zinc-500 leading-relaxed">
              No analytics, no telemetry, no third-party trackers. The Groq API
              key sits in Windows Credential Manager. Audio never touches disk
              for longer than the round-trip to the model.
            </p>
          </div>
        </div>
      </Card>

      <Card className="p-5">
        <div className="flex items-start gap-4">
          <div className="w-10 h-10 rounded-xl bg-[var(--color-surface)] border border-[var(--color-line)] flex items-center justify-center shrink-0">
            <Code2 className="w-4 h-4 text-zinc-400" />
          </div>
          <div className="flex-1">
            <p className="text-[13px] font-medium text-zinc-50 mb-1">
              Built with
            </p>
            <p className="text-[12px] text-zinc-500 leading-relaxed font-mono">
              Tauri 2 · Rust · React 18 · Vite · Tailwind 4
            </p>
          </div>
        </div>
      </Card>
    </div>
  );
}
