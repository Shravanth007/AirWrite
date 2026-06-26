import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  RefreshCw,
  ShieldAlert,
  Mic2,
  Waves,
  Volume2,
} from "lucide-react";
import type { Settings, MicDevice, MicTestResult } from "./types";
import {
  Block,
  Button,
  Field,
  PageHero,
  Pill,
  Select,
} from "./primitives";

interface Props {
  settings: Settings;
  setSettings: (s: Settings) => void;
  mics: MicDevice[];
  refreshMics: () => Promise<void>;
}

function levelTone(peak: number) {
  if (peak < 0.001) return "text-red-400";
  if (peak < 0.005) return "text-orange-300";
  if (peak < 0.05) return "text-yellow-300";
  return "text-emerald-300";
}

function barColor(peak: number) {
  if (peak < 0.001) return "bg-red-500";
  if (peak < 0.005) return "bg-orange-400";
  if (peak < 0.05) return "bg-yellow-400";
  return "bg-emerald-400";
}

function barWidth(peak: number) {
  const normalized = Math.min(1, Math.sqrt(peak));
  return `${Math.max(2, normalized * 100).toFixed(0)}%`;
}

export function AudioSection({
  settings,
  setSettings,
  mics,
  refreshMics,
}: Props) {
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState<MicTestResult | null>(null);
  const [err, setErr] = useState("");

  async function runTest() {
    setTesting(true);
    setErr("");
    setResult(null);
    try {
      const r = await invoke<MicTestResult>("test_microphone", {
        mic: settings.microphone,
      });
      setResult(r);
    } catch (e) {
      setErr(String(e));
    } finally {
      setTesting(false);
    }
  }

  async function openPrivacy() {
    try {
      await invoke("open_mic_privacy_settings");
    } catch (e) {
      setErr(String(e));
    }
  }

  const options = [
    { value: "default", label: "System default" },
    ...mics.map((m) => ({
      value: m.name,
      label: `${m.name}${m.is_default ? " (default)" : ""}`,
    })),
  ];

  return (
    <div>
      <PageHero
        eyebrow="Input"
        title="Audio"
        description="Pick the microphone AirWrite listens to and confirm Windows is feeding it sound."
        Icon={Mic2}
      />

      <Block>
        <Field
          label="Microphone"
          hint="Plugged in a USB mic? Hit refresh to rescan devices."
          trailing={
            <button
              onClick={refreshMics}
              className="text-[10.5px] text-zinc-500 hover:text-white inline-flex items-center gap-1 transition-colors"
            >
              <RefreshCw className="w-3 h-3" />
              Refresh
            </button>
          }
        >
          <Select
            value={settings.microphone}
            onChange={(v) => {
              setSettings({ ...settings, microphone: v });
              setResult(null);
            }}
            options={options}
          />
        </Field>
      </Block>

      <Block className="mt-8">
        <div className="flex items-center justify-between gap-3 mb-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-lg border border-white/[0.08] flex items-center justify-center">
              <Waves className="w-4 h-4 text-zinc-400" />
            </div>
            <div>
              <p className="text-[13px] font-medium text-white">Mic check</p>
              <p className="text-[11px] text-zinc-500">
                Records 1.5 seconds and reports the signal level.
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="secondary" size="sm" onClick={openPrivacy}>
              <ShieldAlert className="w-3.5 h-3.5" />
              Privacy
            </Button>
            <Button
              variant="primary"
              size="sm"
              disabled={testing}
              onClick={runTest}
            >
              <Activity className="w-3.5 h-3.5" />
              {testing ? "Listening" : "Test"}
            </Button>
          </div>
        </div>

        {err && <p className="text-[11.5px] text-red-400 mt-2">{err}</p>}

        {result && (
          <div className="space-y-3 mt-2">
            <div className="flex justify-between text-[10.5px] text-zinc-500 gap-2">
              <span className="truncate text-zinc-400" title={result.device}>
                {result.device}
              </span>
              <span className="shrink-0 font-mono">
                {result.sample_rate} Hz · {result.channels}ch · {result.format}
              </span>
            </div>
            <div className="h-2 bg-white/[0.04] rounded-full overflow-hidden border border-white/[0.06]">
              <div
                className={`h-full ${barColor(result.peak)} transition-[width] duration-300`}
                style={{ width: barWidth(result.peak) }}
              />
            </div>
            <div className="flex justify-between text-[11px] text-zinc-500">
              <span>
                peak{" "}
                <span className={`font-mono ${levelTone(result.peak)}`}>
                  {result.peak.toFixed(4)}
                </span>{" "}
                <span className="text-zinc-600">
                  ({result.peak_db.toFixed(1)} dB)
                </span>
              </span>
              <span className="font-mono text-zinc-600">
                {result.samples_collected.toLocaleString()} samples
              </span>
            </div>
            <p
              className={`pt-1 text-[11.5px] leading-relaxed ${levelTone(result.peak)}`}
            >
              {result.verdict}
            </p>
          </div>
        )}

        {!result && !err && (
          <div className="flex items-center gap-2 text-[11px] text-zinc-500 pt-1">
            <Pill tone="neutral">Idle</Pill>
            <span>Run a test to confirm Windows isn't blocking the mic.</span>
          </div>
        )}
      </Block>

      <Block className="mt-8">
        <div className="flex items-start justify-between gap-3 mb-4">
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-lg border border-white/[0.08] flex items-center justify-center">
              <Volume2 className="w-4 h-4 text-zinc-400" />
            </div>
            <div>
              <p className="text-[13px] font-medium text-white">
                Lower other audio while recording
              </p>
              <p className="text-[11px] text-zinc-500 max-w-[360px] leading-relaxed">
                When you start dictating, the system master volume is briefly
                reduced so music or video doesn't bleed into your mic.
                Restored the moment you stop.
              </p>
            </div>
          </div>
          <Toggle
            checked={settings.duckingEnabled}
            onChange={(v) =>
              setSettings({ ...settings, duckingEnabled: v })
            }
          />
        </div>

        <div
          className={`pl-12 transition-opacity ${
            settings.duckingEnabled ? "opacity-100" : "opacity-40 pointer-events-none"
          }`}
        >
          <div className="flex items-center justify-between text-[11px] text-zinc-500 mb-2">
            <span>Duck to</span>
            <span className="font-mono text-zinc-300">
              {settings.duckingLevel}%
            </span>
          </div>
          <input
            type="range"
            min={0}
            max={100}
            step={5}
            value={settings.duckingLevel}
            onChange={(e) =>
              setSettings({
                ...settings,
                duckingLevel: parseInt(e.target.value, 10),
              })
            }
            className="w-full accent-white"
            disabled={!settings.duckingEnabled}
          />
          <div className="flex justify-between text-[10px] text-zinc-600 mt-1.5 font-mono">
            <span>silent</span>
            <span>unchanged</span>
          </div>
        </div>
      </Block>
    </div>
  );
}

function Toggle({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={`relative shrink-0 w-9 h-5 rounded-full transition-colors border ${
        checked
          ? "bg-white border-white"
          : "bg-transparent border-white/20 hover:border-white/30"
      }`}
    >
      <span
        className={`absolute top-0.5 w-3.5 h-3.5 rounded-full transition-all ${
          checked
            ? "left-[18px] bg-black"
            : "left-0.5 bg-white/40"
        }`}
      />
    </button>
  );
}
