import { Info, Cpu, ShieldCheck, Code2 } from "lucide-react";
import { Block, PageHero } from "./primitives";

export function AboutSection() {
  return (
    <div>
      <PageHero
        eyebrow="Info"
        title="About"
        description="What's powering this and how your data flows."
        Icon={Info}
      />

      <Block>
        <div className="flex items-start gap-4">
          <div className="w-9 h-9 rounded-lg border border-white/[0.08] flex items-center justify-center shrink-0">
            <Cpu className="w-4 h-4 text-zinc-400" />
          </div>
          <div className="flex-1">
            <p className="text-[13px] font-medium text-white mb-1">
              Speech model
            </p>
            <p className="text-[12px] text-zinc-500 leading-relaxed">
              AirWrite uses Groq's hosted{" "}
              <span className="font-mono text-zinc-200">whisper-large-v3-turbo</span>
              . Audio is captured locally, downsampled to 16 kHz mono, and sent
              to <span className="font-mono text-zinc-300">api.groq.com</span>{" "}
              only when you finish a recording.
            </p>
          </div>
        </div>
      </Block>

      <Block className="mt-8">
        <div className="flex items-start gap-4">
          <div className="w-9 h-9 rounded-lg border border-white/[0.08] flex items-center justify-center shrink-0">
            <ShieldCheck className="w-4 h-4 text-zinc-400" />
          </div>
          <div className="flex-1">
            <p className="text-[13px] font-medium text-white mb-1">
              Local-first
            </p>
            <p className="text-[12px] text-zinc-500 leading-relaxed">
              No analytics, no telemetry, no third-party trackers. The Groq API
              key sits in Windows Credential Manager. Audio never touches disk
              for longer than the round-trip to the model.
            </p>
          </div>
        </div>
      </Block>

      <Block className="mt-8">
        <div className="flex items-start gap-4">
          <div className="w-9 h-9 rounded-lg border border-white/[0.08] flex items-center justify-center shrink-0">
            <Code2 className="w-4 h-4 text-zinc-400" />
          </div>
          <div className="flex-1">
            <p className="text-[13px] font-medium text-white mb-1">Built with</p>
            <p className="text-[12px] text-zinc-500 leading-relaxed font-mono">
              Tauri 2 · Rust · React 18 · Vite · Tailwind 4
            </p>
          </div>
        </div>
      </Block>
    </div>
  );
}
