# PRD: AI-Driven Transcription

## 1. Introduction/Overview

Pisum Langue is a cross-platform desktop utility (Windows and macOS) that lets users dictate text anywhere on their system. The user holds a global hotkey to record (push-to-talk) and releases it to stop. The app records the speech to a compressed audio file (Opus preferred), sends it with an prompt to AI provider, copies the resulting text to the clipboard, and pastes it at the current cursor position.

## 2. Goals

- Provide a system-wide hotkey that triggers speech recording from the default microphone
- Record speech to a compressed audio format (Opus preferred, WAV as fallback)
- Send the recorded audio with a prompt to a configurable AI provider (e.g., Gemini) for transcription
- Copy the transcription result to the system clipboard
- Automatically paste the result at the current cursor position
- Keep the AI provider abstracted so it can be replaced or upgraded independently
- Support both Windows and macOS

## 3. User Stories

1. As a user, I want to hold a hotkey and speak so that I can dictate text without switching applications.
2. As a user, I want the transcribed text to appear at my cursor position so that I can dictate directly into any text field or editor.
3. As a user, I want the transcribed text copied to my clipboard so that I can paste it manually if automatic pasting fails.
4. As a user, I want to configure which AI provider and model to use so that I can choose the best option for accuracy, speed, or cost.
5. As a user, I want to switch between prompt presets (e.g., "German transcription", "English meeting notes") so that I can quickly adapt the transcription to different contexts without editing the full prompt each time.

## 4. Functional Requirements

### Recording

1. The system must register a configurable global hotkey that works across all applications on Windows and macOS.
2. The system must capture audio from the default system microphone when the hotkey is activated.
3. The system must encode the captured audio to Opus in an OGG container (OGG_OPUS) for Gemini API compatibility. WAV is available as a fallback format.
4. The system must stop recording when the hotkey is released (push-to-talk mode).
5. The system must enforce a maximum recording duration of 10 minutes. Recording auto-stops when the limit is reached.
6. The system must provide audio/visual feedback (e.g., system tray icon change, small overlay) to indicate recording state.

### Transcription

1. The system must send the recorded audio file to an AI provider (e.g., Gemini) along with a system prompt that instructs the model to transcribe the audio. The language, vocabulary hints, and formatting instructions are all part of the prompt — there is no separate language parameter.
2. The system must support multiple named prompt presets (roles). Each preset has a name and a system prompt that controls transcription behavior (e.g., target language, vocabulary hints, formatting instructions, output style). The user selects the active preset from the system tray menu or settings UI.
3. The system must ship with sensible built-in presets (e.g., "Transcribe DE" for German transcription, "Transcribe EN" for English transcription). Users can create, edit, and delete custom presets. Built-in presets cannot be deleted but can be edited.
4. The AI provider must be abstracted behind a trait (`TranscriptionProvider`) so the implementation can be swapped without modifying consuming code.
5. The system must distribute transcription requests across configured providers in round-robin order to balance API rate limits and quotas. If a provider fails, the system must fall back to the next available provider.

### Output

1. The system must copy the transcription result to the system clipboard.
2. The system must simulate a paste action (Ctrl+V / Cmd+V) to insert the text at the current cursor position.

### Error Handling & Offline Behavior

1. The system must detect when the network is unavailable or the transcription API call fails.
2. The system must show an OS-native toast notification (Windows toast notification / macOS NSUserNotification) with a clear error message when transcription fails for any reason (network, auth, quota, etc.).
3. The system must not silently discard errors — every failure in the pipeline (recording, encoding, transcription, pasting) must surface a notification to the user.

### Configuration

1. The system must provide a settings UI (accessible from system tray) to configure: hotkey, audio format, AI provider credentials, and prompt presets.
2. The system must persist settings between sessions.
3. The system must start minimized to the system tray / menu bar.
4. The system must auto-start with the OS (Windows Startup / macOS Login Items) by default. The user can disable auto-start in settings.

## 5. Non-Goals (Out of Scope)

- Not included: A full windowed UI for reviewing, editing, or exporting transcriptions
- Not included: File upload — the only input is live microphone recording
- Not included: SRT, TXT, or DOCX export
- Not included: Translation — though a user could create a prompt preset that includes translation instructions, this is not a first-class feature
- Not included: Clipboard restoration — the transcription result overwrites the current clipboard content
- Not included: Streaming/real-time transcription during recording (audio is sent after recording stops)
- Not included: User accounts, authentication, or multi-user support
- Not included: Mobile platform support
- Not included: Auto-update mechanism or distribution/installer tooling

## 6. Design Considerations

- The app runs as a system tray (Windows) / menu bar (macOS) application with no main window
- A small floating indicator or tray icon color change shows when recording is active
- Settings are accessed via right-click on the tray/menu bar icon
- The tray context menu shows available prompt presets for quick switching (radio-style selection with a checkmark on the active preset)
- The interaction should feel instantaneous — minimal latency between stopping recording and text appearing at the cursor
- Errors (network failure, invalid API key, no microphone, etc.) are surfaced as OS-native toast notifications (Windows toast / macOS NSUserNotification) so the user always gets feedback even when no app window is visible

## 7. Technical Considerations

- **Cross-platform:** Use Tauri 2 (Rust backend) with Svelte 5 (TypeScript frontend), Vite 6, and Tailwind CSS. Platform-specific behavior is isolated via conditional compilation (`#[cfg(...)]`) in Rust modules.
- **Global Hotkey:** Use the `global-hotkey` crate for cross-platform system-wide hotkey registration. The `GlobalHotKeyManager` runs on the main thread (thread-local), with an event loop in a background thread listening for press/release events.
- **Audio Recording:** Use the `cpal` crate (Cross-Platform Audio Library) to capture from the default microphone on a dedicated thread. Supports f32, i16, and u16 sample formats with normalization to f32.
- **Audio Encoding:** Encode to Opus in an OGG container (OGG_OPUS format) using `audiopus` for Opus encoding, `rubato` for high-quality sinc resampling to Opus-compatible sample rates, and the `ogg` crate for Ogg container wrapping. WAV via `hound` as fallback.
- **Clipboard & Paste Simulation:** Use the `arboard` crate for cross-platform clipboard access. Simulate Ctrl+V / Cmd+V via the `enigo` crate for cross-platform keystroke simulation. The transcription overwrites the current clipboard content (no restore).
- **Notifications:** Use `tauri-plugin-notification` for OS-native toast notifications on both Windows and macOS.
- **AI Provider Abstraction:** Define a `TranscriptionProvider` Rust trait. Implementations are instantiated and managed via a `ProviderPool` that handles round-robin distribution and fallback.
- **AI Provider:** Gemini (Google Generative Language API) is the default provider, accessed via API key. Default model: `gemini-2.5-flash-lite`. Audio is sent as base64-encoded inline data. No service account JSON files.
- **System Tray:** Use Tauri's `TrayIconBuilder` with dynamic icon theming (light/dark detection). macOS uses `iconAsTemplate` for automatic theme adaptation. Windows detects dark mode via registry.
- **Configuration:** JSON config files in the user's home directory. Dual-file system: fixed-location settings file and configurable-location config file, serialized via `serde`.
- **Auto-Start:** Use `tauri-plugin-autostart` for OS startup integration — Windows Startup registry and macOS LaunchAgent. Configurable in settings.

## 8. Success Metrics

- End-to-end latency (hotkey release → text pasted) under 3 seconds for short utterances (< 15 seconds of speech)
- Transcription works reliably across common applications (browsers, editors, chat apps, Office)
- Swapping the AI provider requires only adding a new trait implementation and registering it in the provider pool
- The app runs unobtrusively in the system tray with minimal resource usage when idle

## 9. Open Questions

- [x] Which cross-platform framework should be used? -> Tauri 2 (Rust backend) + Svelte 5 (TypeScript frontend)
- [x] Should the hotkey default to push-to-talk (hold to record) or toggle (press to start/stop)? -> push-to-talk only
- [x] What is the maximum recording duration to support? -> 10 min
- [x] Which AI provider should be the default implementation (OpenAI Whisper, Azure, Google Gemini)? -> Google Gemini (`gemini-2.5-flash-lite`)
- [x] Should the app support multiple prompt presets (e.g., "medical terminology", "casual conversation")? -> Yes, with built-in defaults and user-defined custom presets. Active preset selectable from tray menu.
- [x] How should clipboard restoration work — always restore, or make it configurable? -> No clipboard restoration; transcription overwrites clipboard
