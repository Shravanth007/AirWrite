export type RecordingMode = "toggle" | "push_to_talk";

export interface Settings {
  microphone: string;
  groqApiKey: string;
  recordingMode: RecordingMode | string;
  hotkey: string;
  settingsHotkey: string;
  duckingEnabled: boolean;
  duckingLevel: number;
  aiCleanupEnabled: boolean;
  clipboardRestore: boolean;
  repasteHotkey: string;
  transcriptionLanguage: string;
}

export interface MicDevice {
  name: string;
  is_default: boolean;
}

export interface MicTestResult {
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

export type SectionId =
  | "api-key"
  | "audio"
  | "hotkey"
  | "recording"
  | "history"
  | "about";

export interface HistoryEntry {
  text: string;
  timestamp: number;
  duration_secs: number;
  word_count: number;
}
