import { useState } from "react";
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
  Button,
  Card,
  Field,
  PageHero,
  Pill,
  TextInput,
} from "./primitives";

interface Props {
  settings: Settings;
  setSettings: (s: Settings) => void;
}

function maskedPreview(key: string): string {
  if (!key) return "";
  if (key.length <= 10) return "•".repeat(key.length);
  return `${key.slice(0, 4)}${"•".repeat(Math.min(key.length - 8, 16))}${key.slice(-4)}`;
}

export function ApiKeySection({ settings, setSettings }: Props) {
  const [reveal, setReveal] = useState(false);
  const trimmed = settings.groqApiKey.trim();
  const hasKey = trimmed.length > 0;

  return (
    <div className="space-y-7">
      <PageHero
        eyebrow="Credentials"
        title="API key"
        description="AirWrite needs a Groq API key to transcribe what you say. The key stays on this machine — stored in Windows Credential Manager."
        Icon={KeyRound}
      />

      <Card glow={!hasKey} className="p-5">
        <div className="flex items-center gap-3 mb-5">
          <div
            className={`w-2 h-2 rounded-full ${
              hasKey
                ? "bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.6)]"
                : "bg-amber-400 shadow-[0_0_8px_rgba(251,191,36,0.6)]"
            }`}
          />
          <span className="text-[13px] font-medium text-zinc-100">
            {hasKey ? "Key configured" : "No key set"}
          </span>
          {hasKey && (
            <span className="font-mono text-[11px] text-zinc-500 ml-auto">
              {maskedPreview(trimmed)}
            </span>
          )}
        </div>

        <Field
          label={hasKey ? "Update key" : "Paste your key"}
          trailing={
            settings.groqApiKey ? (
              <button
                onClick={() => setReveal((r) => !r)}
                className="text-[10.5px] text-zinc-500 hover:text-zinc-200 inline-flex items-center gap-1"
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
              <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
              Encrypted at rest by Windows.
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setSettings({ ...settings, groqApiKey: "" })}
            >
              <Trash2 className="w-3 h-3" />
              Remove
            </Button>
          </div>
        )}
      </Card>

      <Card className="p-5">
        <div className="flex items-start gap-4">
          <div className="w-9 h-9 shrink-0 rounded-lg bg-[var(--color-surface)] border border-[var(--color-line)] flex items-center justify-center">
            <ExternalLink className="w-4 h-4 text-zinc-400" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h3 className="text-[13px] font-medium text-zinc-100">
                Don't have one yet?
              </h3>
              <Pill tone="brand">Free</Pill>
            </div>
            <p className="text-[11.5px] text-zinc-500 leading-relaxed">
              Sign up at{" "}
              <span className="text-brand-300 font-mono">console.groq.com</span>
              , create an API key, and paste it above. Groq's free tier covers
              casual dictation comfortably.
            </p>
          </div>
        </div>
      </Card>
    </div>
  );
}
