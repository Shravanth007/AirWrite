import { useEffect, useState } from "react";
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

export function SettingsPanel() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [mics, setMics] = useState<MicDevice[]>([]);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings).catch(console.error);
    invoke<MicDevice[]>("list_microphones").then(setMics).catch(console.error);
  }, []);

  async function save() {
    if (!settings) return;
    try {
      await invoke("save_settings", { settings });
      setSaved(true);
      setError("");
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(String(e));
    }
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
          />
        </div>

        <div>
          <label className="block text-xs text-zinc-400 mb-1.5">
            Microphone
          </label>
          <select
            value={settings.microphone}
            onChange={(e) =>
              setSettings({ ...settings, microphone: e.target.value })
            }
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
        </div>

        <div>
          <label className="block text-xs text-zinc-400 mb-1.5">Hotkey</label>
          <div className="bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-sm text-zinc-400">
            {settings.hotkey}
          </div>
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
          Press <span className="text-zinc-300">Ctrl+Shift+Space</span> anywhere
          to start/stop recording.
        </p>
      </div>
    </div>
  );
}
