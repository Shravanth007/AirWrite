import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check } from "lucide-react";

import { Sidebar } from "./settings/Sidebar";
import { ApiKeySection } from "./settings/ApiKeySection";
import { AudioSection } from "./settings/AudioSection";
import { HotkeySection } from "./settings/HotkeySection";
import { RecordingSection } from "./settings/RecordingSection";
import { AboutSection } from "./settings/AboutSection";
import { ErrorBanner } from "./settings/ErrorBanner";
import { Button } from "./settings/primitives";
import type { Settings, MicDevice, SectionId } from "./settings/types";

function normalize(s: Settings): Settings {
  return {
    ...s,
    groqApiKey: s.groqApiKey.trim(),
    hotkey: s.hotkey.trim(),
    settingsHotkey: s.settingsHotkey.trim(),
  };
}

const SEARCH_INDEX: Record<SectionId, string[]> = {
  "api-key": ["api", "key", "groq", "credential", "token", "secret"],
  audio: ["audio", "mic", "microphone", "input", "device", "level"],
  hotkey: ["hotkey", "shortcut", "key", "binding", "trigger", "combination"],
  recording: ["recording", "mode", "toggle", "push to talk", "ptt", "behavior"],
  about: ["about", "info", "version", "credits", "model", "whisper"],
};

function matchSearch(query: string): SectionId | null {
  const q = query.trim().toLowerCase();
  if (!q) return null;
  for (const id of Object.keys(SEARCH_INDEX) as SectionId[]) {
    if (SEARCH_INDEX[id].some((kw) => kw.startsWith(q) || kw.includes(q))) {
      return id;
    }
  }
  return null;
}

export function SettingsPanel() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [snapshot, setSnapshot] = useState<Settings | null>(null);
  const [mics, setMics] = useState<MicDevice[]>([]);
  const [section, setSection] = useState<SectionId>("api-key");
  const [query, setQuery] = useState("");
  const [loadError, setLoadError] = useState("");
  const [saving, setSaving] = useState(false);
  const [savedAt, setSavedAt] = useState<number | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    Promise.all([
      invoke<Settings>("get_settings"),
      invoke<MicDevice[]>("list_microphones"),
    ])
      .then(([s, m]) => {
        if (cancelled) return;
        setSettings(s);
        setSnapshot(s);
        setMics(m);
        if (s.groqApiKey.trim().length > 0) setSection("audio");
      })
      .catch((e) => {
        if (cancelled) return;
        setLoadError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Live-jump to the section whose keywords match what the user typed.
  // ("/" focusing the search field is handled inside Sidebar.)
  useEffect(() => {
    const id = matchSearch(query);
    if (id) setSection(id);
  }, [query]);

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
    setSaving(true);
    setError("");
    const trimmed = normalize(settings);
    try {
      await invoke("save_settings", { settings: trimmed });
      setSettings(trimmed);
      setSnapshot(trimmed);
      setSavedAt(Date.now());
      setTimeout(() => setSavedAt(null), 2200);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  const dirty = useMemo(() => {
    if (!settings || !snapshot) return false;
    return (
      JSON.stringify(normalize(settings)) !==
      JSON.stringify(normalize(snapshot))
    );
  }, [settings, snapshot]);

  if (loadError) {
    return (
      <div className="h-screen w-screen bg-black text-zinc-300 flex flex-col items-center justify-center gap-2 text-sm p-6 text-center">
        <p className="text-red-400">Could not load settings.</p>
        <p className="text-zinc-500 text-xs">{loadError}</p>
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="h-screen w-screen bg-black flex items-center justify-center text-zinc-500 text-sm">
        <span className="animate-pulse tracking-widest text-[11px] uppercase">
          Loading
        </span>
      </div>
    );
  }

  return (
    <div className="h-screen w-screen flex bg-black text-zinc-200 overflow-hidden">
      <Sidebar
        current={section}
        onSelect={(id) => {
          setSection(id);
          setQuery("");
        }}
        settings={settings}
        query={query}
        setQuery={setQuery}
      />

      <main className="flex-1 flex flex-col min-w-0 bg-black overflow-hidden">
        <ErrorBanner />
        <div className="flex-1 overflow-y-auto">
          <div className="max-w-[640px] mx-auto px-12 pt-14 pb-10">
            {section === "api-key" && (
              <ApiKeySection settings={settings} setSettings={setSettings} />
            )}
            {section === "audio" && (
              <AudioSection
                settings={settings}
                setSettings={setSettings}
                mics={mics}
                refreshMics={refreshMics}
              />
            )}
            {section === "hotkey" && (
              <HotkeySection settings={settings} setSettings={setSettings} />
            )}
            {section === "recording" && (
              <RecordingSection
                settings={settings}
                setSettings={setSettings}
              />
            )}
            {section === "about" && <AboutSection />}
            <div className="h-16" />
          </div>
        </div>

        <SaveBar
          saving={saving}
          savedAt={savedAt}
          error={error}
          dirty={dirty}
          onSave={save}
        />
      </main>
    </div>
  );
}

function SaveBar({
  saving,
  savedAt,
  error,
  dirty,
  onSave,
}: {
  saving: boolean;
  savedAt: number | null;
  error: string;
  dirty: boolean;
  onSave: () => void;
}) {
  return (
    <div className="border-t border-white/[0.06] bg-black/80 backdrop-blur-md px-8 py-3.5 flex items-center justify-between gap-3">
      <div className="text-[11.5px] min-w-0 flex-1">
        {error ? (
          <span className="text-red-400 truncate block" title={error}>
            {error}
          </span>
        ) : savedAt ? (
          <span className="text-emerald-400 inline-flex items-center gap-1.5">
            <Check className="w-3.5 h-3.5" strokeWidth={2.5} />
            Saved
          </span>
        ) : dirty ? (
          <span className="text-amber-400/90 inline-flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
            Unsaved changes
          </span>
        ) : (
          <span className="text-zinc-600">All changes saved.</span>
        )}
      </div>
      <Button
        variant="primary"
        size="md"
        onClick={onSave}
        disabled={saving || !dirty}
      >
        {saving ? "Saving" : "Save"}
      </Button>
    </div>
  );
}
