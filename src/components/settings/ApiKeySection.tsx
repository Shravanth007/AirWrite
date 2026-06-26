import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  KeyRound,
  Eye,
  EyeOff,
  ShieldCheck,
  ExternalLink,
  Trash2,
} from "lucide-react";
import type { Settings } from "./types";
import {
  Block,
  Button,
  Field,
  PageHero,
  Pill,
  TextInput,
} from "./primitives";

interface Props {
  settings: Settings;
  setSettings: (s: Settings) => void;
  hasKey: boolean;
  onKeyChanged: () => void;
}

function maskedPreview(key: string): string {
  if (!key) return "";
  if (key.length <= 10) return "•".repeat(key.length);
  return `${key.slice(0, 4)}${"•".repeat(Math.min(key.length - 8, 16))}${key.slice(-4)}`;
}

export function ApiKeySection({ settings, setSettings, hasKey, onKeyChanged }: Props) {
  const [reveal, setReveal] = useState(false);
  const typed = settings.groqApiKey.trim();

  return (
    <div>
      <PageHero
        eyebrow="Credentials"
        title="API key"
        description="AirWrite uses a Groq API key to transcribe audio. The key stays on this machine — stored in Windows Credential Manager."
        Icon={KeyRound}
      />

      <Block>
        <div className="flex items-center gap-2.5 mb-5">
          <span
            className={`w-1.5 h-1.5 rounded-full ${
              hasKey ? "bg-white" : "bg-white/40"
            }`}
          />
          <span className="text-[13px] font-medium text-white">
            {hasKey ? "Key configured" : "No key set"}
          </span>
          {hasKey ? <Pill tone="ok">Active</Pill> : <Pill tone="warn">Required</Pill>}
          {hasKey && (
            <span className="font-mono text-[11px] text-zinc-500 ml-auto">
              {typed ? maskedPreview(typed) : "••••••••••••"}
            </span>
          )}
        </div>

        <Field
          label={hasKey ? "Update key" : "Paste your key"}
          trailing={
            settings.groqApiKey ? (
              <button
                onClick={() => setReveal((r) => !r)}
                className="text-[10.5px] text-zinc-500 hover:text-white inline-flex items-center gap-1 transition-colors"
                type="button"
              >
                {reveal ? (
                  <>
                    <EyeOff className="w-3 h-3" /> Hide
                  </>
                ) : (
                  <>
                    <Eye className="w-3 h-3" /> Reveal
                  </>
                )}
              </button>
            ) : null
          }
        >
          <TextInput
            type={reveal ? "text" : "password"}
            value={settings.groqApiKey}
            onChange={(v) => setSettings({ ...settings, groqApiKey: v })}
            placeholder="gsk_…"
            spellCheck={false}
            autoComplete="off"
          />
        </Field>

        {hasKey && (
          <div className="mt-4 flex items-center justify-between gap-2">
            <div className="inline-flex items-center gap-2 text-[11px] text-zinc-500">
              <ShieldCheck className="w-3.5 h-3.5 text-zinc-400" />
              Encrypted at rest by Windows.
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={async () => {
                try { await invoke("clear_api_key"); setSettings({ ...settings, groqApiKey: "" }); onKeyChanged(); }
                catch (e) { /* surface via existing error path if present, else ignore */ }
              }}
            >
              <Trash2 className="w-3 h-3" />
              Remove
            </Button>
          </div>
        )}
      </Block>

      <Block className="mt-8">
        <div className="flex items-start gap-4">
          <div className="w-9 h-9 shrink-0 rounded-lg border border-white/[0.08] flex items-center justify-center">
            <ExternalLink className="w-4 h-4 text-zinc-400" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h3 className="text-[13px] font-medium text-white">
                Don't have one yet?
              </h3>
              <Pill tone="neutral">Free</Pill>
            </div>
            <p className="text-[12px] text-zinc-500 leading-relaxed">
              Sign up at{" "}
              <span className="text-zinc-200 font-mono">console.groq.com</span>
              , create an API key, and paste it above. Groq's free tier covers
              casual dictation comfortably.
            </p>
          </div>
        </div>
      </Block>
    </div>
  );
}
