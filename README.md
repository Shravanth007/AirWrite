# AirWrite

AirWrite is a Windows desktop dictation app that lets you speak from anywhere on your system and paste the transcription directly into the focused app.

It runs in the background, listens for a global hotkey, records from your selected microphone, sends the audio to Groq Whisper for transcription, and pastes the result back with `Ctrl+V`.

## Platform

AirWrite currently supports Windows only.

## What It Does

- Global hotkey dictation from any app
- Toggle and push-to-talk recording modes
- Groq API key storage through Windows Credential Manager
- Microphone picker with a built-in mic test
- Configurable hotkeys for recording, opening settings, and re-pasting the latest transcript
- Optional audio ducking while recording
- Optional AI cleanup pass for punctuation and grammar polishing
- History view for recent successful dictations
- Re-paste support for the most recent transcript

## Screenshots

### API Key

![API key](assets/screenshots/api-key.png)

### Audio

![Audio settings](assets/screenshots/audio.png)

### Hotkeys

![Hotkey settings](assets/screenshots/hotkeys.png)

### Recording

![Recording settings](assets/screenshots/recording.png)

### History

![History view](assets/screenshots/history.png)

## How It Works

1. Add your Groq API key in Settings.
2. Choose the microphone AirWrite should use.
3. Press the recording hotkey from anywhere in Windows.
4. Speak.
5. Stop or release the hotkey.
6. AirWrite transcribes the audio and pastes the text into the focused window.

## Setup

### Prerequisites

- Windows
- Node.js 18+
- Rust
- A Groq API key

### Run In Development

```powershell
npm install
npm run tauri dev
```

### Build

```powershell
npm run tauri build
```

Tauri build outputs Windows installers such as:

- `src-tauri/target/release/bundle/nsis/AirWrite_0.1.0_x64-setup.exe`
- `src-tauri/target/release/bundle/msi/AirWrite_0.1.0_x64_en-US.msi`

## Privacy

- Your Groq API key is stored on Windows through Credential Manager.
- AirWrite records locally, then sends the captured audio to Groq for transcription.
- Recent transcription history is stored locally on your machine.

## Tech Stack

- Tauri 2
- Rust
- React 18
- Vite
- Tailwind CSS 4
- Groq Whisper

## Status

AirWrite is an early Windows-only desktop app and is still evolving.
