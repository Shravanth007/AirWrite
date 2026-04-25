import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Settings {
  microphone: string;
  groqApiKey: string;
  recordingMode: string;
  hotkey: string;
}

interface MicDevice {
  name: string;
  is_default: boolean;
}

interface MicTestResult {
  device: string;
  sample_rate: number;
  channels: number;
  format: string;
  duration_ms: number;
  samples_collected: number;
  peak: number;
  peak_db: number;
  rms: number;
  verdict: string;
}

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

function levelTone(peak: number): string {
  if (peak < 0.001) return "text-red-400";
  if (peak < 0.005) return "text-orange-400";
  if (peak < 0.05) return "text-yellow-400";
  return "text-green-400";
}

function levelBarWidth(peak: number): string {
  // Map peak to a visual 0–100% with mild compression so quiet signal is still visible.
  const normalized = Math.min(1, Math.sqrt(peak));
  return `${Math.max(2, normalized * 100).toFixed(0)}%`;
}

export function SettingsPanel() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [mics, setMics] = useState<MicDevice[]>([]);
  const [loadError, setLoadError] = useState("");
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState("");
  const [capturing, setCapturing] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<MicTestResult | null>(null);
  const [testError, setTestError] = useState("");
  const captureRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let cancelled = false;
    Promise.all([
      invoke<Settings>("get_settings"),
      invoke<MicDevice[]>("list_microphones"),
    ])
      .then(([s, m]) => {
        if (cancelled) return;
        setSettings(s);
        setMics(m);
      })
      .catch((e) => {
        if (cancelled) return;
        setLoadError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, []);

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
      setSettings((prev) => (prev ? { ...prev, hotkey: accel } : prev));
      setCapturing(false);
    };
    window.addEventListener("keydown", handler, { capture: true });
    captureRef.current?.focus();
    return () => window.removeEventListener("keydown", handler, { capture: true });
  }, [capturing]);

  async function refreshMics() {
    try {
      const m = await invoke<MicDevice[]>("list_microphones");
      setMics(m);
    } catch (e) {
      setError(String(e));
    }
  }

  async function save() {
    if (!settings) return;
    const trimmed: Settings = {
      ...settings,
      groqApiKey: settings.groqApiKey.trim(),
    };
    setSettings(trimmed);
    setError("");
    try {
      await invoke("save_settings", { settings: trimmed });
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(String(e));
    }
  }

  async function runMicTest() {
    if (!settings) return;
    setTesting(true);
    setTestError("");
    setTestResult(null);
    try {
      const r = await invoke<MicTestResult>("test_microphone", {
        mic: settings.microphone,
      });
      setTestResult(r);
    } catch (e) {
      setTestError(String(e));
    } finally {
      setTesting(false);
    }
  }

  async function openPrivacy() {
    try {
      await invoke("open_mic_privacy_settings");
    } catch (e) {
      setError(String(e));
    }
  }

  if (loadError) {
    return (
      <div className="flex flex-col items-center justify-center h-screen bg-zinc-900 text-zinc-300 text-sm gap-2 p-6 text-center">
        <p className="text-red-400">Could not load settings.</p>
        <p className="text-zinc-500 text-xs">{loadError}</p>
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="flex items-center justify-center h-screen bg-zinc-900 text-zinc-400 text-sm">
        Loading…
      </div>
    );
  }

  return (
    <div className="p-6 bg-zinc-900 text-white min-h-screen text-sm">
      <h1 className="text-base font-semibold mb-6">AirWrite Settings</h1>

      <div className="space-y-5">
        <div>
          <label className="block text-xs text-zinc-400 mb-1.5">
            Groq API Key
          </label>
          <input
            type="password"
            value={settings.groqApiKey}
            onChange={(e) =>
              setSettings({ ...settings, groqApiKey: e.target.value })
            }
            className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500 transition-colors"
            placeholder="gsk_..."
            spellCheck={false}
            autoComplete="off"
          />
          <p className="text-[10px] text-zinc-500 mt-1">
            Stored in Windows Credential Manager, not in plain text.
          </p>
        </div>

        <div>
          <div className="flex items-center justify-between mb-1.5">
            <label className="block text-xs text-zinc-400">Microphone</label>
            <button
              onClick={refreshMics}
              className="text-[10px] text-zinc-500 hover:text-zinc-300"
              type="button"
            >
              Refresh list
            </button>
          </div>
          <select
            value={settings.microphone}
            onChange={(e) => {
              setSettings({ ...settings, microphone: e.target.value });
              setTestResult(null);
            }}
            className="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500 transition-colors"
          >
            <option value="default">Default</option>
            {mics.map((m) => (
              <option key={m.name} value={m.name}>
                {m.name}
                {m.is_default ? " (default)" : ""}
              </option>
            ))}
          </select>

          <div className="mt-2 flex items-center gap-2">
            <button
              onClick={runMicTest}
              disabled={testing}
              className="bg-zinc-800 hover:bg-zinc-700 disabled:opacity-60 border border-zinc-700 px-3 py-1.5 rounded-lg text-xs"
              type="button"
            >
              {testing ? "Listening 1.5s…" : "Test microphone"}
            </button>
            <button
              onClick={openPrivacy}
              className="bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 px-3 py-1.5 rounded-lg text-xs"
              type="button"
            >
              Open Windows mic privacy
            </button>
          </div>

          {testError && (
            <p className="mt-2 text-[11px] text-red-400">{testError}</p>
          )}

          {testResult && (
            <div className="mt-3 bg-zinc-950/60 border border-zinc-800 rounded-lg p-3 space-y-2 text-[11px]">
              <div className="flex justify-between text-zinc-400">
                <span className="truncate" title={testResult.device}>
                  {testResult.device}
                </span>
                <span>
                  {testResult.sample_rate} Hz · {testResult.channels} ch ·{" "}
                  {testResult.format}
                </span>
              </div>
              <div className="h-2 bg-zinc-800 rounded overflow-hidden">
                <div
                  className={`h-full transition-all ${
                    testResult.peak < 0.001
                      ? "bg-red-500"
                      : testResult.peak < 0.05
                      ? "bg-yellow-400"
                      : "bg-green-500"
                  }`}
                  style={{ width: levelBarWidth(testResult.peak) }}
                />
              </div>
              <div className="flex justify-between text-zinc-500">
                <span>
                  peak{" "}
                  <span className={levelTone(testResult.peak)}>
                    {testResult.peak.toFixed(4)}
                  </span>{" "}
                  ({testResult.peak_db.toFixed(1)} dB)
                </span>
                <span>{testResult.samples_collected} samples</span>
              </div>
              <p className={`pt-1 ${levelTone(testResult.peak)}`}>
                {testResult.verdict}
              </p>
            </div>
          )}
        </div>

        <div>
          <label className="block text-xs text-zinc-400 mb-1.5">Hotkey</label>
          <div
            ref={captureRef}
            tabIndex={0}
            role="button"
            onClick={() => setCapturing(true)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") setCapturing(true);
            }}
            className={`bg-zinc-800 border rounded-lg px-3 py-2 text-sm cursor-pointer focus:outline-none transition-colors ${
              capturing
                ? "border-blue-500 text-zinc-300"
                : "border-zinc-700 text-zinc-200 hover:border-zinc-600"
            }`}
          >
            {capturing
              ? "Press a key combination… (Esc to cancel)"
              : settings.hotkey}
          </div>
          <p className="text-[10px] text-zinc-500 mt-1">
            Click and press a combination. Must include Ctrl, Alt, or Shift.
          </p>
        </div>
      </div>

      <div className="mt-6 flex items-center gap-3">
        <button
          onClick={save}
          className="bg-blue-600 hover:bg-blue-500 active:bg-blue-700 px-4 py-2 rounded-lg text-sm font-medium transition-colors"
        >
          {saved ? "Saved!" : "Save Settings"}
        </button>
        {error && <span className="text-red-400 text-xs">{error}</span>}
      </div>

      <div className="mt-8 pt-6 border-t border-zinc-800">
        <p className="text-xs text-zinc-500 leading-relaxed">
          Get a free Groq API key at{" "}
          <span className="text-zinc-300">console.groq.com</span>
          <br />
          Press <span className="text-zinc-300">{settings.hotkey}</span> anywhere
          to start/stop recording.
        </p>
      </div>
    </div>
  );
}
