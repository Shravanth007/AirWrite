# AirWrite

A small Windows desktop app that turns your voice into pasted text anywhere on
your computer. Press a hotkey, talk, release — whatever you said gets
transcribed by Groq's Whisper Turbo model and pasted into the focused window
via simulated `Ctrl+V`.

Built with **Tauri 2 · Rust · React 18 · Vite · Tailwind 4**.

---

## What it does

- Global hotkey starts and stops dictation from any app — no need to bring
  AirWrite to the foreground.
- Two modes:
  - **Toggle** — press once to start, press again to stop.
  - **Push to talk** — hold while speaking, release to transcribe.
- Floating "notch" overlay near the top of your screen shows recording /
  transcribing / done / error state, with a 5-bar audio waveform while
  recording.
- Click-to-rebind hotkeys for both recording and opening Settings.
- Built-in **mic test** that records 1.5 s and reports peak / RMS / sample
  format so you can confirm Windows is letting AirWrite hear the mic.
- One-click button into Windows microphone privacy settings.

## How your data flows

- Audio is captured locally via CPAL, downsampled to 16 kHz mono, and uploaded
  to `api.groq.com` only at the moment you finish a recording.
- The audio file is held in a `tempfile::NamedTempFile` and deleted as soon
  as the round-trip completes (or the process crashes).
- Your Groq API key lives in **Windows Credential Manager** under
  `com.airwrite.app` — never written to `config.json`, never sent anywhere
  except as a `Bearer` token to Groq.
- App config (mic, hotkeys, recording mode) lives in
  `%LOCALAPPDATA%\com.airwrite.app\config.json`. No telemetry, no
  analytics, no third-party tracking.
- Pasting works by setting the system clipboard to the transcribed text and
  synthesising `Ctrl+V`. Whatever was on your clipboard before a dictation
  is **overwritten** — AirWrite does not save and restore the previous
  contents. Copy anything important before dictating.

## Quick start (dev)

You need Rust, Node 18+, and a [Groq API key](https://console.groq.com).

```powershell
git clone https://github.com/Shravanth007/AirWrite.git
cd AirWrite
npm install
npm run tauri dev
```

On first launch, AirWrite opens the Settings window automatically and
prompts you for the API key. Paste it, save, and you're done — the default
hotkey is `Ctrl+Shift+Space` for recording, `Ctrl+Alt+S` for opening
Settings.

## Logs & diagnostics

Logs are written to the terminal you launched `npm run tauri dev` from. The
default filter is `airwrite=info, airwrite_lib=info, warn`, which is enough
for normal use. Set `RUST_LOG` to dial it up or down:

```powershell
$env:RUST_LOG="airwrite=debug,airwrite_lib=debug"
npm run tauri dev
```

### What a healthy recording looks like

A successful press-and-talk cycle prints four log lines:

```text
[INFO  airwrite_lib::audio]    Captured 8.72s of audio: peak=0.0444 (-27.1 dBFS), rms=0.0058
[INFO  airwrite_lib::recorder] Speed: groq=0.67s rtf=0.08x · audio=8.72s · upload=273KB · paste=0.05s · total=0.79s
[INFO  airwrite_lib::recorder] Transcription: "After every successful recording your..."
[INFO  airwrite]               Hotkey: ptt: stopped & transcribed
```

#### `Captured ... of audio`

Emitted by `audio::stop_and_save`. Tells you how long the recording was, the
peak amplitude, and RMS. The **silence guard** kicks in at peak `< 0.005`
(≈ -46 dBFS) and refuses to send empty audio to Groq — that's why this line
exists, so you can see *why* a recording was rejected.

| Reading | What it means |
|---|---|
| `peak=0.0000` | Mic produced exact zero. Almost always Windows mic-privacy blocking the app. |
| `peak < 0.005` | Effectively silent — Whisper would hallucinate "Thank you" on this. |
| `peak ≥ 0.05`  | Healthy speech-level signal. |

#### `Speed: groq=... rtf=... · audio=... · upload=... · paste=... · total=...`

Per-recording latency breakdown:

| Field | Meaning |
|---|---|
| `groq` | Wall time of the API call (network + model). |
| `rtf` | **Real-time factor** = `groq / audio`. Lower is better. Groq's `whisper-large-v3-turbo` typically lands at **0.05 – 0.20** (5–20× faster than real-time). |
| `audio` | Length of the recording you just made. |
| `upload` | Size of the 16 kHz WAV sent over the wire (~32 KB per second of audio). |
| `paste` | Time to set the clipboard + synthesise `Ctrl+V`. |
| `total` | End-to-end pipeline time from "stop recording" to "text pasted". |

If you see `rtf > 1.0`, something is bottlenecking — typically slow
network or Groq itself being temporarily overloaded.

#### `Transcription: "..."`

The exact text Groq returned (post-cleanup but pre-paste). Useful when paste
lands in the wrong window — you can still see what was transcribed.

#### `Hotkey: ...`

The toggle/PTT state machine's outcome:

- `toggle: started` / `toggle: stopped & transcribed`
- `ptt: started` / `ptt: stopped & transcribed`
- `ptt: noop` — release without a matching press, or press during transcribe
- `toggle: ignored release` — releasing the toggle hotkey is a no-op

### Errors

Network and HTTP failures are surfaced both in the floating overlay and as a
red banner at the top of the Settings window. Sample translations:

| What happened | What you'll see |
|---|---|
| WiFi off | "Can't reach api.groq.com. You may be offline or behind a firewall." |
| Wrong API key | "Your Groq API key was rejected. Open Settings → API key to update it." |
| Rate limit (429) | "Too many requests. Wait a few seconds and try again." |
| Groq is down | "Groq is having issues right now. Try again in a moment." |
| Hotkey can't bind | "Recording hotkey X couldn't be bound — another app may already use it…" |

The raw `reqwest` error is always logged at `warn` level so you can dig into
the technical detail when the user-facing message isn't enough.

## Project layout

```text
src/                    React UI (Vite, Tailwind 4, Lucide icons)
  components/
    Overlay.tsx         floating notch with state pills + waveform
    SettingsPanel.tsx   sidebar app-shell that hosts each section
    settings/           ApiKey · Audio · Hotkey · Recording · About sections,
                        ErrorBanner, Sidebar, primitives (Button, Pill, etc.)

src-tauri/              Rust backend
  src/
    main.rs             tray, overlay window, hotkey registration & dispatch
    recorder.rs         state machine: Ready → Recording → Transcribing
    audio.rs            CPAL capture, format-aware (F32/I16/U16/I32),
                        silence guard, mic test
    transcribe_groq.rs  multipart upload, error classification
    paste.rs            clipboard set + simulated Ctrl+V via enigo
    settings.rs         config.json + Windows Credential Manager glue
    cleanup.rs          text normalisation (capitalisation, trailing punctuation)
```

## Status

`v0.1` — single user, single machine, dev builds only. No installer signing,
no auto-updater, no telemetry. The UI is monochrome black-and-white by
design.
