# Implementation Plan: AI-Driven Dictation

> Generated from: `docs/PRD-transcription.md`
> Date: 2026-03-13

## 1. Overview

Pisum Langue is a cross-platform desktop utility (Windows and macOS) that runs as a system tray / menu bar application. Users hold a global hotkey to record audio from the default microphone (push-to-talk). On release, the recorded audio is compressed (Opus in OGG container), sent to a configurable AI provider (Gemini by default) with a transcription system prompt, and the transcription result is copied to the clipboard and pasted at the current cursor position. Errors are surfaced via OS-native toast notifications.

The project is greenfield — no code exists yet. The tech stack is **Tauri 2 (Rust backend) + Svelte 5 (TypeScript frontend)**, based on the proven architecture from the `github-global-hotkey` reference implementation. The Rust backend handles hotkey registration, audio recording/encoding, AI provider communication, clipboard management, and paste simulation. The Svelte 5 frontend provides a settings UI accessible from the system tray. Communication between frontend and backend uses Tauri's IPC command system with automatic JSON serialization via serde.

### Key Architectural Decisions

- **Tauri 2 over .NET/Avalonia**: Proven cross-platform approach from the reference repo, smaller binary size, native performance, built-in system tray support
- **Push-to-talk via hotkey hold/release**: Unlike the reference repo's toggle pattern, Pisum Langue uses hold-to-record with a maximum 10-minute duration
- **Gemini API**: Default provider (via API key, `gemini-2.5-flash-lite` model), behind a trait-based abstraction for swappability
- **Prompt presets (roles)**: Named presets with system prompts, selectable from the settings UI. Built-in defaults for common languages; user-created custom presets
- **Round-robin provider load balancing**: Distribute requests across configured providers, falling back on failure

## 2. Architecture & Design

### High-Level Component Diagram

```text
┌─────────────────────────────────────────────────────────────┐
│                   Svelte 5 Frontend (UI)                     │
│  ┌────────────┐  ┌────────────────┐  ┌───────────────────┐  │
│  │ Settings    │  │ Provider Config │  │ Hotkey Config     │  │
│  │ Page        │  │ Panel          │  │ Panel             │  │
│  └─────┬──────┘  └───────┬────────┘  └────────┬──────────┘  │
└────────┼─────────────────┼─────────────────────┼─────────────┘
         │       Tauri IPC (JSON/serde)          │
┌────────┼─────────────────┼─────────────────────┼─────────────┐
│        ▼                 ▼                     ▼             │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                    Tauri Commands (lib.rs)               │ │
│  └──┬──────────┬──────────┬──────────┬──────────┬──────────┘ │
│     │          │          │          │          │            │
│     ▼          ▼          ▼          ▼          ▼            │
│  ┌──────┐  ┌──────┐  ┌───────┐  ┌───────┐  ┌───────────┐   │
│  │hotkey│  │audio │  │  ai   │  │config │  │  output   │   │
│  │      │  │      │  │       │  │       │  │           │   │
│  │mgr   │  │record│  │gemini │  │schema │  │clipboard  │   │
│  │parse │  │encode│  │provid.│  │manage │  │paste sim  │   │
│  └──┬───┘  └──┬───┘  └───┬───┘  └───┬───┘  └─────┬─────┘   │
│     │         │          │          │             │          │
│     ▼         ▼          ▼          ▼             ▼          │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  tray.rs (system tray, notifications, recording UI)  │    │
│  └──────────────────────────────────────────────────────┘    │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  error.rs (centralized AppError enum via thiserror)  │    │
│  └──────────────────────────────────────────────────────┘    │
│                       Rust Backend (Tauri)                    │
└──────────────────────────────────────────────────────────────┘
```

### Data Flow: Push-to-Talk Recording Cycle

```text
1. User presses hotkey
   → global-hotkey event received in background thread
   → Start AudioRecorderHandle on dedicated thread (cpal)
   → Update tray icon to "recording" state
   → Start 10-minute max-duration timer

2. User releases hotkey (OR timer expires)
   → Send Stop command to recorder thread
   → Collect f32 samples, sample_rate, channels

3. Encode audio
   → Resample to Opus-compatible rate (rubato sinc interpolation)
   → Encode to Opus frames (audiopus, 24kbps VoIP)
   → Wrap in Ogg container (OpusHead + OpusTags + packets)
   → Fallback: encode to WAV if Opus fails

4. Transcribe
   → Get system prompt from active preset
   → Select provider via round-robin (skip failed providers)
   → POST base64-encoded audio + active preset's system prompt to Gemini API (generateContent)
   → Retry on 429/503 with exponential backoff (max 3 retries)

5. Output
   → Copy transcription text to clipboard (arboard)
   → Simulate Ctrl+V / Cmd+V paste (enigo)
   → Restore tray icon to idle state
   → On error: show OS-native toast notification
```

### Hotkey Hold/Release Detection

The reference repo uses a toggle pattern (press to start, press to stop). Pisum Langue requires **push-to-talk** (hold to record, release to stop). The `global-hotkey` crate fires events for both press and release. Implementation:

```rust
// In hotkey event handler
match event.state() {
    HotKeyState::Pressed => start_recording(),
    HotKeyState::Released => stop_recording_and_transcribe(),
}
```

### Provider Round-Robin with Fallback

```rust
pub struct ProviderPool {
    providers: Vec<Box<dyn TranscriptionProvider>>,
    current_index: AtomicUsize,
}

impl ProviderPool {
    pub async fn transcribe(&self, audio: &[u8], mime: &str, prompt: &str) -> Result<String, AppError> {
        let len = self.providers.len();
        let start = self.current_index.fetch_add(1, Ordering::Relaxed) % len;
        for i in 0..len {
            let idx = (start + i) % len;
            match self.providers[idx].transcribe(audio, mime, prompt).await {
                Ok(text) => return Ok(text),
                Err(e) => log::warn!("Provider {} failed: {}", idx, e),
            }
        }
        Err(AppError::Transcription("All providers failed".into()))
    }
}
```

## 3. Phases & Milestones

### Phase 1: Project Scaffolding, Logging & System Tray

**Goal:** Bootable Tauri 2 app running as a system tray/menu bar application with no main window. File-based logging initialized early so all subsequent phases have logging from the start.
**Deliverable:** App starts minimized to tray, shows right-click menu with "Settings" and "Quit" options. Settings opens a hidden window. Structured logs written to `~/.pisum-langue/logs/`.

### Phase 2: Global Hotkey & Audio Recording
**Goal:** Register a configurable global hotkey. Hold-to-record captures audio from the default microphone and encodes to Opus/OGG.
**Deliverable:** Press and hold hotkey → tray icon changes → release → Opus file produced (verifiable via debug log / temp file).

### Phase 3: AI Provider Abstraction & Gemini
**Goal:** Trait-based transcription provider with Gemini as the default implementation. Round-robin load balancing.
**Deliverable:** Audio recorded in Phase 2 is sent to Gemini and returns transcription text.

### Phase 4: Clipboard & Paste Output
**Goal:** Copy transcription to clipboard and simulate paste at cursor position.
**Deliverable:** End-to-end flow works: hold hotkey → speak → release → text appears at cursor.

### Phase 5: Settings UI
**Goal:** Svelte 5 settings UI accessible from system tray for configuring hotkey, audio format, AI provider credentials, and prompt presets. Built-in presets for common languages; user-created custom presets.
**Deliverable:** All configuration options from PRD §4.4 are functional and persisted. Preset management (create, edit, delete) works. Active preset selectable from settings UI.

### Phase 6: Error Handling, Notifications & Auto-Start

**Goal:** OS-native toast notifications for all error conditions. Auto-start with OS. Recording duration limit.
**Deliverable:** Every failure in the pipeline surfaces a notification. App can auto-start on login.

## 4. Files Overview

### Files to Create

| File Path | Purpose |
|-----------|---------|
| `src-tauri/src/main.rs` | Entry point, calls `lib::run()` |
| `src-tauri/src/lib.rs` | Tauri command handlers, app setup, plugin registration |
| `src-tauri/src/error.rs` | Centralized `AppError` enum via `thiserror` |
| `src-tauri/src/tray.rs` | System tray setup, icon management, notifications, recording indicator |
| `src-tauri/src/hotkey/mod.rs` | Module exports |
| `src-tauri/src/hotkey/manager.rs` | Hotkey registration, event loop, push-to-talk state machine |
| `src-tauri/src/hotkey/parse.rs` | Hotkey string parsing (modifiers + key code) |
| `src-tauri/src/hotkey/conflict.rs` | Hotkey conflict detection (app + system hotkeys) |
| `src-tauri/src/audio/mod.rs` | Module exports |
| `src-tauri/src/audio/recorder.rs` | Audio capture via `cpal` on dedicated thread |
| `src-tauri/src/audio/encoder.rs` | Opus/OGG encoding with sinc resampling, WAV fallback |
| `src-tauri/src/ai/mod.rs` | Module exports |
| `src-tauri/src/ai/provider.rs` | `TranscriptionProvider` trait definition |
| `src-tauri/src/ai/gemini.rs` | Gemini API client implementation |
| `src-tauri/src/ai/pool.rs` | Round-robin provider pool with fallback |
| `src-tauri/src/config/mod.rs` | Module exports |
| `src-tauri/src/config/schema.rs` | Configuration data structures (serde) |
| `src-tauri/src/config/manager.rs` | Load/save single settings JSON file |
| `src-tauri/src/logging.rs` | File-based logging setup (`~/.pisum-langue/logs/`) |
| `src-tauri/src/config/presets.rs` | Built-in preset definitions, preset CRUD helpers |
| `src-tauri/src/output/mod.rs` | Module exports |
| `src-tauri/src/output/clipboard.rs` | Clipboard write via `arboard` |
| `src-tauri/src/output/paste.rs` | Paste simulation via `enigo` (Ctrl+V / Cmd+V) |
| `src-tauri/Cargo.toml` | Rust dependencies |
| `src-tauri/build.rs` | Tauri build script |
| `src-tauri/tauri.conf.json` | Tauri app configuration |
| `src-tauri/capabilities/default.json` | Tauri 2 capability-based permissions for plugins |
| `src-tauri/icons/` | App icons and tray icons (light/dark variants) |
| `src/App.svelte` | Root Svelte component (settings UI shell) |
| `src/main.ts` | Svelte app entry point |
| `src/app.css` | Global styles (Tailwind) |
| `src/lib/commands.ts` | Typed Tauri IPC command wrappers |
| `src/lib/types.ts` | TypeScript type definitions mirroring Rust schemas |
| `src/components/SettingsPage.svelte` | Main settings page layout |
| `src/components/HotkeyConfig.svelte` | Hotkey configuration panel (capture + display) |
| `src/components/HotkeyRecorder.svelte` | Hotkey capture widget (listens for keydown/keyup, shows modifiers in real-time) |
| `src/components/ProviderConfig.svelte` | AI provider credentials and selection |
| `src/components/AudioConfig.svelte` | Audio format and language settings |
| `src/components/PresetConfig.svelte` | Prompt preset management (list, create, edit, delete) |
| `src/stores/settings.ts` | Svelte reactive store for app settings |
| `package.json` | Node.js dependencies |
| `vite.config.ts` | Vite configuration |
| `svelte.config.js` | Svelte configuration |
| `tsconfig.json` | TypeScript configuration |
| `tailwind.config.js` | Tailwind CSS configuration |
| `postcss.config.js` | PostCSS configuration |

### Files to Modify

| File Path | What Changes |
|-----------|-------------|
| `CLAUDE.md` | Already updated to Tauri/Rust/Svelte tech stack |
| `.gitignore` | Add Tauri/Rust/Node build artifacts (`target/`, `node_modules/`, `dist/`) |

## 5. Task Breakdown

### Phase 1 Tasks: Project Scaffolding, Logging & System Tray

#### Task 1.1: Initialize Tauri 2 + Svelte 5 Project

- **Files to create/modify:**
  - `package.json` — Node dependencies: Svelte 5, Vite 6, Tailwind CSS 3, `@tauri-apps/api`, `@tauri-apps/plugin-dialog`
  - `vite.config.ts` — Vite + Svelte plugin configuration
  - `svelte.config.js` — Svelte 5 configuration
  - `tsconfig.json` — TypeScript strict mode, Svelte paths
  - `tailwind.config.js` — Tailwind content paths
  - `postcss.config.js` — PostCSS with Tailwind and Autoprefixer
  - `src/main.ts` — Svelte app mount point
  - `src/app.css` — Tailwind directives (`@tailwind base/components/utilities`)
  - `src/App.svelte` — Root component with minimal "Settings" placeholder
  - `.gitignore` — Add `target/`, `node_modules/`, `dist/`
- **Implementation details:**
  - Use `npm create tauri-app@latest` structure as reference but create manually for control
  - Svelte 5 with TypeScript, Vite 6, Tailwind CSS 3
- **Dependencies:** None
- **Acceptance criteria:** `npm run dev` starts Vite dev server; `npm run build` produces `dist/`

#### Task 1.2: Initialize Rust Backend (Cargo + Tauri)

- **Files to create/modify:**
  - `src-tauri/Cargo.toml` — Package definition and initial dependencies:
    ```toml
    [package]
    name = "pisum-langue"
    version = "0.1.0"
    edition = "2021"

    [lib]
    name = "pisum_langue_lib"
    crate-type = ["lib", "cdylib", "staticlib"]

    [build-dependencies]
    tauri-build = { version = "2", features = [] }

    [dependencies]
    tauri = { version = "2", features = ["tray-icon", "macos-private-api", "image-png"] }
    tauri-plugin-notification = "2"
    tauri-plugin-autostart = "2"
    serde = { version = "1", features = ["derive"] }
    serde_json = "1"
    thiserror = "2"
    once_cell = "1.20"
    tokio = { version = "1", features = ["rt"] }
    image = { version = "0.25", default-features = false, features = ["png"] }
    ```
  - `src-tauri/build.rs` — `tauri_build::build()`
  - `src-tauri/src/main.rs` — Entry point calling `pisum_langue_lib::run()`
  - `src-tauri/src/lib.rs` — Minimal `run()` function with Tauri builder
  - `src-tauri/src/error.rs` — `AppError` enum with `Config`, `Io`, `Json` variants
  - `src-tauri/tauri.conf.json` — App configuration:
    ```json
    {
      "productName": "PisumLangue",
      "identifier": "com.pisumlangue.app",
      "app": {
        "macOSPrivateApi": true,
        "windows": [{
          "title": "Pisum Langue - Settings",
          "width": 700, "height": 500,
          "visible": false, "center": true
        }]
      }
    }
    ```
  - `src-tauri/capabilities/default.json` — Tauri 2 capability-based permissions:
    ```json
    {
      "identifier": "default",
      "description": "Default capabilities for Pisum Langue",
      "windows": ["main"],
      "permissions": [
        "core:default",
        "notification:default",
        "notification:allow-notify",
        "notification:allow-request-permission",
        "autostart:default",
        "autostart:allow-enable",
        "autostart:allow-disable",
        "autostart:allow-is-enabled"
      ]
    }
    ```
- **Implementation details:**
  - Tauri 2 uses a capability-based permission system. All plugin permissions must be declared in `src-tauri/capabilities/` for the frontend to invoke them via IPC
- **Dependencies:** Task 1.1
- **Acceptance criteria:** `cargo tauri dev` launches the app; `cargo build` succeeds. Capabilities file includes all required plugin permissions.

#### Task 1.3: System Tray with Menu

- **Files to create/modify:**
  - `src-tauri/src/tray.rs` — Tray setup and notification helper:
    ```rust
    use once_cell::sync::Lazy;
    use std::sync::RwLock;
    use tauri::{AppHandle, Manager};

    static APP_HANDLE: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

    pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> { ... }
    pub fn send_notification(title: &str, message: &str) { ... }
    pub fn set_recording_state(recording: bool) { ... }
    pub fn set_tray_tooltip(preset_name: &str) { ... }
    ```
  - `src-tauri/src/lib.rs` — Add tray setup in `.setup()` callback
  - `src-tauri/icons/` — Create tray icons (idle, recording) for light/dark themes
- **Implementation details:**
  - Right-click menu: "Settings" (shows hidden window), separator, "Quit"
  - No preset submenu in tray — preset switching is done through the settings UI only
  - "Settings" click: `app.get_webview_window("main").unwrap().show()`
  - Window close event (`CloseRequested`): hide the window back to tray instead of quitting the app
  - Tray icon changes color when recording (Phase 2 will activate this)
  - Tray tooltip displays the active preset name (e.g., "Pisum Langue — Transcribe DE"). Updated via `set_tray_tooltip()` on startup and whenever the active preset changes
  - Store `AppHandle` in global `APP_HANDLE` for notifications from any module
  - macOS: use `iconAsTemplate` for automatic theme adaptation
  - Windows: detect dark mode via registry (`AppsUseLightTheme`) and load appropriate icon
- **Dependencies:** Task 1.2
- **Acceptance criteria:** App starts minimized to tray. Right-click shows "Settings" and "Quit". "Settings" opens window. Closing the settings window hides it (back to tray). "Quit" exits. No main window on launch. Tray tooltip displays the active preset name.

#### Task 1.4: File-Based Logging

- **Files to create/modify:**
  - `src-tauri/src/logging.rs` — Logging setup:
    - Use `tracing` + `tracing-subscriber` + `tracing-appender` for structured file logging
    - Log directory: `~/.pisum-langue/logs/`
    - Rotating log files (daily rotation, keep 7 days)
    - Log levels: ERROR for user-facing failures, WARN for retries/fallbacks, INFO for pipeline events, DEBUG for development
  - `src-tauri/src/lib.rs` — Initialize logging in app setup **before** other modules (tray, hotkey, etc.)
  - `src-tauri/Cargo.toml` — Add `tracing = "0.1"`, `tracing-subscriber = "0.3"`, `tracing-appender = "0.2"`
- **Implementation details:**
  - Logging must be initialized as the first step in the `.setup()` callback so that all subsequent module initialization is logged
  - Console output in dev mode (`#[cfg(debug_assertions)]`), file-only in release
- **Dependencies:** Task 1.2
- **Acceptance criteria:** App writes structured logs to `~/.pisum-langue/logs/`. Logs rotate daily. Old logs cleaned up after 7 days. All Phase 2+ work benefits from logging.

### Phase 2 Tasks: Global Hotkey & Audio Recording

#### Task 2.1: Hotkey Registration & Push-to-Talk Event Loop

- **Files to create/modify:**
  - `src-tauri/src/hotkey/mod.rs` — Module exports
  - `src-tauri/src/hotkey/manager.rs` — Hotkey manager:
    ```rust
    use global_hotkey::{GlobalHotKeyManager, GlobalHotKeyEvent, HotKeyState};
    use once_cell::sync::Lazy;
    use std::cell::RefCell;
    use std::sync::Mutex;

    thread_local! {
        static MANAGER: RefCell<Option<GlobalHotKeyManager>> = const { RefCell::new(None) };
    }

    static REGISTRY: Lazy<Mutex<Option<(u32, global_hotkey::hotkey::HotKey)>>> =
        Lazy::new(|| Mutex::new(None));

    pub fn init() { /* create manager on main thread */ }
    pub fn register(binding: &HotkeyBinding) -> Result<(), AppError> { ... }
    pub fn unregister() -> Result<(), AppError> { ... }
    fn start_event_loop(app: AppHandle) { /* background thread */ }
    ```
  - `src-tauri/src/hotkey/parse.rs` — Parse hotkey binding to `global_hotkey::hotkey::HotKey`:
    - Modifier mapping: ctrl/control → Modifiers::CONTROL, alt → ALT, shift → SHIFT, meta/cmd/win/super → META
    - Key code mapping: A-Z, 0-9, F1-F12, special keys
  - `src-tauri/src/hotkey/conflict.rs` — Hotkey conflict detection:
    - Check against app's own registered hotkeys
    - Check against known system hotkeys (macOS: Cmd+Q, Cmd+W, Cmd+Tab, Cmd+Space, Cmd+Shift+3/4/5; Windows: Ctrl+Alt+Del, Alt+Tab, Alt+F4, Win+L/D/E/R/Tab, Ctrl+Shift+Esc)
  - `src-tauri/src/lib.rs` — Add Tauri commands: `register_hotkey`, `unregister_hotkey`, `get_current_hotkey`, `check_conflict`, `check_system_conflict`
  - `src-tauri/Cargo.toml` — Add `global-hotkey = "0.6"`
- **Implementation details:**
  - GlobalHotKeyManager is thread-local; registration must happen on main thread via `app.run_on_main_thread()`
  - Event loop runs in background thread, listens via `GlobalHotKeyEvent::receiver()`
  - Push-to-talk state machine:
    - `HotKeyState::Pressed` → call `handle_hotkey_press(app_handle)`
    - `HotKeyState::Released` → call `handle_hotkey_release(app_handle)`
  - Only one hotkey registered at a time (single configurable hotkey per PRD)
- **Dependencies:** Task 1.3
- **Acceptance criteria:** Hotkey registers on startup. Press event logged. Release event logged. Hotkey works across all applications. Conflict detection warns about system hotkey clashes.

#### Task 2.2: Audio Recording with cpal

- **Files to create/modify:**
  - `src-tauri/src/audio/mod.rs` — Module exports
  - `src-tauri/src/audio/recorder.rs` — Audio recorder (adapt from reference repo):
    ```rust
    pub struct AudioRecorderHandle {
        command_tx: Sender<RecorderCommand>,
        samples: Arc<Mutex<Vec<f32>>>,
        is_recording: Arc<AtomicBool>,
        sample_rate: u32,
        channels: u16,
        thread_handle: Option<JoinHandle<()>>,
    }

    impl AudioRecorderHandle {
        pub fn start() -> Result<Self, AppError> { ... }
        pub fn stop(mut self) -> Result<(Vec<f32>, u32, u16), AppError> { ... }
        pub fn is_recording(&self) -> bool { ... }
    }
    ```
  - `src-tauri/src/error.rs` — Add `Audio(String)` variant
  - `src-tauri/Cargo.toml` — Add `cpal = "0.15"`
- **Implementation details:**
  - Dedicated recording thread (cpal stream is not `Send`)
  - Support f32, i16, u16 input formats with normalization to f32 [-1.0, 1.0]
  - `mpsc::channel` for stop command communication
  - `Arc<AtomicBool>` for recording state flag
  - Store active recorder in `static ACTIVE_RECORDER: Lazy<Mutex<Option<AudioRecorderHandle>>>`
- **Dependencies:** Task 2.1
- **Acceptance criteria:** Audio captured from default microphone. Samples accessible after stop. No audio glitches.

#### Task 2.3: Audio Encoding (Opus/OGG + WAV Fallback)

- **Files to create/modify:**
  - `src-tauri/src/audio/encoder.rs` — Encoding functions (adapt from reference repo):
    ```rust
    pub fn encode_to_opus(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>, AppError>
    pub fn encode_to_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>, AppError>
    pub fn opus_mime_type() -> &'static str { "audio/ogg" }
    pub fn wav_mime_type() -> &'static str { "audio/wav" }
    ```
  - `src-tauri/Cargo.toml` — Add:
    ```toml
    audiopus = "0.2"
    rubato = "0.16"
    ogg = "0.9"
    hound = "3.5"
    ```
- **Implementation details:**
  - The encoder respects the user's `audio_format` setting from config. If set to Opus, encode to Opus; if set to WAV, encode to WAV directly (no Opus attempt)
  - Runtime fallback: if the selected format's encoding fails (e.g., Opus library unavailable), fall back to the other format and log a warning via `tracing::warn!`
  - Resampling pipeline: detect if sample rate is Opus-compatible (8k/12k/16k/24k/48k), resample via `rubato::SincFixedIn` if not
  - Sinc parameters: `sinc_len: 256`, `f_cutoff: 0.95`, `oversampling_factor: 256`, `WindowFunction::BlackmanHarris2`
  - Opus encoding: `Application::Voip`, 24kbps bitrate, 20ms frames
  - Ogg wrapping: OpusHead header, OpusTags header (vendor: "pisum-langue"), audio packets with 48kHz granule positions
  - WAV encoding: 16-bit PCM via `hound`
- **Dependencies:** Task 2.2
- **Acceptance criteria:** Recorded audio encodes to valid Ogg/Opus. File plays correctly in external player. WAV fallback produces valid WAV.

#### Task 2.4: Integrate Recording with Hotkey (Push-to-Talk Orchestration)

- **Files to create/modify:**
  - `src-tauri/src/hotkey/manager.rs` — Wire press/release to recording:
    ```rust
    fn handle_hotkey_press(app: &AppHandle) {
        // Start recording
        // Update tray icon to recording state
        // Start max-duration timer (10 min)
    }

    fn handle_hotkey_release(app: &AppHandle) {
        // Stop recording
        // Encode audio
        // (Phase 3: send to AI)
        // (Phase 4: clipboard + paste)
        // Restore tray icon
    }
    ```
  - `src-tauri/src/tray.rs` — Implement `set_recording_state()` to swap tray icon
- **Implementation details:**
  - Max recording duration: 10 minutes (600,000 ms). Spawn a timer thread on press; if it fires before release, auto-stop recording.
  - Guard against double-press: if already recording, ignore subsequent press events
  - Guard against concurrent transcription: if a transcription API call is already in-flight from a previous recording, ignore the new press event and show a "transcription in progress" tray notification. Use an `Arc<AtomicBool>` flag (`is_transcribing`) to track this state
  - On release with no active recording (edge case), do nothing gracefully
  - Empty recording (press and immediately release, < 0.5s): skip transcription silently, restore tray icon, no notification
  - macOS microphone permission: `cpal` will trigger the system permission prompt on first use. If denied, `default_input_device()` returns `None` → show error notification guiding user to System Settings > Privacy & Security > Microphone
- **Dependencies:** Tasks 2.1, 2.2, 2.3
- **Acceptance criteria:** Hold hotkey → tray shows recording → release → Opus data produced. Recording auto-stops at 10 minutes. Tray icon updates correctly. Quick press-release (< 0.5s) is silently ignored.

### Phase 3 Tasks: AI Provider Abstraction & Gemini

#### Task 3.1: Transcription Provider Trait

- **Files to create/modify:**
  - `src-tauri/src/ai/mod.rs` — Module exports
  - `src-tauri/src/ai/provider.rs` — Provider trait:
    ```rust
    use crate::error::AppError;

    pub struct TranscriptionResult {
        pub text: String,
    }

    pub trait TranscriptionProvider: Send + Sync {
        fn transcribe(
            &self,
            audio_data: &[u8],
            mime_type: &str,
            system_prompt: &str,
        ) -> impl std::future::Future<Output = Result<TranscriptionResult, AppError>> + Send;

        fn test_connection(
            &self,
        ) -> impl std::future::Future<Output = Result<bool, AppError>> + Send;

        fn provider_name(&self) -> &str;
    }
    ```
  - `src-tauri/src/error.rs` — Add `Transcription(String)` variant
- **Dependencies:** None (trait is standalone)
- **Acceptance criteria:** Trait compiles. Can be implemented by any provider.

#### Task 3.2: Gemini Provider Implementation

- **Files to create/modify:**
  - `src-tauri/src/ai/gemini.rs` — Gemini provider (adapt from reference repo):
    ```rust
    const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
    const DEFAULT_MODEL: &str = "gemini-2.5-flash-lite";
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_MS: u64 = 1000;

    pub struct GeminiProvider {
        client: Client,
        api_key: String,
        model: String,
    }

    impl GeminiProvider {
        pub fn new(api_key: String, model: Option<String>) -> Self { ... }
    }

    impl TranscriptionProvider for GeminiProvider {
        async fn transcribe(&self, audio_data: &[u8], mime_type: &str,
                           system_prompt: &str)
            -> Result<TranscriptionResult, AppError> { ... }
        async fn test_connection(&self) -> Result<bool, AppError> { ... }
        fn provider_name(&self) -> &str { "Gemini" }
    }
    ```
  - `src-tauri/Cargo.toml` — Add:
    ```toml
    reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
    base64 = "0.22"
    ```
- **Implementation details:**
  - Gemini API: `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={API_KEY}`
  - Request body:
    ```json
    {
      "system_instruction": { "parts": [{ "text": "<system_prompt from active preset>" }] },
      "contents": [{ "parts": [
        { "inline_data": { "mime_type": "audio/ogg", "data": "<base64 audio>" } }
      ]}],
      "generationConfig": { "temperature": 0.1, "maxOutputTokens": 8192 }
    }
    ```
  - Retry logic: max 3 retries, exponential backoff (1s base), retry on HTTP 429/503 or response containing "overloaded"/"rate limit"
  - Parse response: extract `candidates[0].content.parts[0].text`
  - Auth: API key as query parameter
  - Low temperature (0.1) for deterministic transcription output
- **Dependencies:** Task 3.1
- **Acceptance criteria:** Sending recorded Opus audio returns correct transcription. Retry works on transient errors. Invalid API key returns clear error.

#### Task 3.3: Provider Pool (Round-Robin + Fallback)

- **Files to create/modify:**
  - `src-tauri/src/ai/pool.rs` — Provider pool:
    ```rust
    pub struct ProviderPool {
        providers: Vec<Box<dyn TranscriptionProvider>>,
        current_index: AtomicUsize,
    }

    impl ProviderPool {
        pub fn new(providers: Vec<Box<dyn TranscriptionProvider>>) -> Self { ... }
        pub async fn transcribe(&self, audio: &[u8], mime: &str,
                               system_prompt: &str)
            -> Result<TranscriptionResult, AppError> { ... }
        pub fn rebuild(&mut self, configs: &[ProviderConfig]) { ... }
    }
    ```
  - Store as `static PROVIDER_POOL: Lazy<RwLock<ProviderPool>>`
- **Implementation details:**
  - Round-robin: `AtomicUsize` counter, mod by provider count
  - Fallback: on failure, try next provider in sequence until all exhausted
  - `rebuild()`: called when settings change, constructs new provider instances from config. Uses a write lock, which will block until any in-flight `transcribe()` read lock is released. This is acceptable because settings changes are infrequent and user-initiated
  - Initialized on first launch during app setup (in `lib.rs` `.setup()` callback) from the loaded settings. If no providers are configured (e.g., first launch before API key entry), the pool is empty and `transcribe()` returns an error prompting the user to configure a provider
- **Dependencies:** Task 3.2
- **Acceptance criteria:** Requests cycle across providers. Failed provider is skipped. All-fail returns aggregated error. Empty pool returns clear "no providers configured" error.

#### Task 3.4: Wire Transcription into Recording Pipeline

- **Files to create/modify:**
  - `src-tauri/src/hotkey/manager.rs` — In `handle_hotkey_release()`, after encoding:
    1. Load config, find active preset by `active_preset_id`, get its `system_prompt`
    2. Call `PROVIDER_POOL.read().transcribe(audio, mime, &active_preset.system_prompt).await`
    3. Pass result to output module (Phase 4)
  - `src-tauri/src/lib.rs` — Add `test_provider_connection` Tauri command
- **Dependencies:** Tasks 2.4, 3.3
- **Acceptance criteria:** End-to-end: hold hotkey → speak → release → transcription text returned from API.

### Phase 4 Tasks: Clipboard & Paste Output

#### Task 4.1: Clipboard Write

- **Files to create/modify:**
  - `src-tauri/src/output/mod.rs` — Module exports
  - `src-tauri/src/output/clipboard.rs`:
    ```rust
    use arboard::Clipboard;
    use crate::error::AppError;

    pub fn set_clipboard_text(text: &str) -> Result<(), AppError> {
        let mut clipboard = Clipboard::new()
            .map_err(|e| AppError::Output(format!("Failed to access clipboard: {}", e)))?;
        clipboard.set_text(text.to_string())
            .map_err(|e| AppError::Output(format!("Failed to set clipboard: {}", e)))?;
        Ok(())
    }
    ```
  - `src-tauri/src/error.rs` — Add `Output(String)` variant
  - `src-tauri/Cargo.toml` — Add `arboard = "3"`
- **Dependencies:** None
- **Acceptance criteria:** Text set via `set_clipboard_text` is retrievable from system clipboard.

#### Task 4.2: Paste Simulation

- **Files to create/modify:**
  - `src-tauri/src/output/paste.rs`:
    ```rust
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use crate::error::AppError;

    pub fn simulate_paste() -> Result<(), AppError> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| AppError::Output(format!("Failed to create input simulator: {}", e)))?;

        #[cfg(target_os = "macos")]
        let modifier = Key::Meta;
        #[cfg(not(target_os = "macos"))]
        let modifier = Key::Control;

        enigo.key(modifier, Direction::Press).map_err(|e| AppError::Output(e.to_string()))?;
        enigo.key(Key::Unicode('v'), Direction::Click).map_err(|e| AppError::Output(e.to_string()))?;
        enigo.key(modifier, Direction::Release).map_err(|e| AppError::Output(e.to_string()))?;
        Ok(())
    }
    ```
  - `src-tauri/Cargo.toml` — Add `enigo = "0.3"`
- **Dependencies:** Task 4.1
- **Acceptance criteria:** After `set_clipboard_text("test")` + `simulate_paste()`, "test" appears at the active cursor position in any application.

#### Task 4.3: Wire Output into Pipeline

- **Files to create/modify:**
  - `src-tauri/src/hotkey/manager.rs` — In `handle_hotkey_release()`, after transcription:
    ```rust
    output::clipboard::set_clipboard_text(&result.text)?;
    output::paste::simulate_paste()?;
    ```
- **Dependencies:** Tasks 3.4, 4.1, 4.2
- **Acceptance criteria:** Full end-to-end: hold hotkey → speak → release → text pasted at cursor. Clipboard contains the transcription.

### Phase 5 Tasks: Settings UI

#### Task 5.1: Configuration Schema & Manager

- **Files to create/modify:**
  - `src-tauri/src/config/mod.rs` — Module exports
  - `src-tauri/src/config/schema.rs` — Configuration data structures:
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AppSettings {
        pub start_with_system: bool,
        pub show_tray_notifications: bool,
        pub hotkey: HotkeyBinding,
        pub audio_format: AudioFormat,          // Opus or Wav
        pub presets: Vec<Preset>,               // Named prompt presets (roles)
        pub active_preset_id: String,           // ID of the currently active preset
        pub providers: Vec<ProviderConfig>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Preset {
        pub id: String,                         // Unique identifier (e.g., "de-transcribe")
        pub name: String,                       // Display name (e.g., "Transcribe DE")
        pub system_prompt: String,              // Full prompt sent to AI provider
        pub is_builtin: bool,                   // Built-in presets can be edited but not deleted
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ProviderConfig {
        pub id: String,
        pub provider_type: ProviderType,    // Gemini (extensible)
        pub api_key: String,
        pub model: Option<String>,
        pub enabled: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HotkeyBinding {
        pub modifiers: Vec<String>,
        pub key: String,
    }

    pub enum AudioFormat { Opus, Wav }
    pub enum ProviderType { Gemini }
    ```
  - `src-tauri/src/config/manager.rs` — File I/O:
    ```rust
    const SETTINGS_FILE: &str = ".pisum-langue.json";   // ~/

    pub fn init() -> Result<(), AppError> { /* create file with defaults if missing */ }
    pub fn load_settings() -> Result<AppSettings, AppError> { ... }
    pub fn save_settings(settings: &AppSettings) -> Result<(), AppError> { ... }
    ```
  - `src-tauri/Cargo.toml` — Add `dirs = "5"`, `uuid = { version = "1", features = ["v4"] }`
- **Implementation details:**
  - Single settings file: `~/.pisum-langue.json` contains all configuration (settings, hotkey, presets, providers)
  - No config migration logic — use `#[serde(default)]` on all fields so missing fields get defaults when schema changes
  - API keys are stored in plaintext in the settings file (accepted tradeoff)
  - Auto-create file with defaults on first run
  - Default hotkey: Ctrl+Shift+Space (Windows) / Cmd+Shift+Space (macOS)
  - Default audio format: Opus
  - Built-in presets loaded from `presets.rs` on first run and merged on subsequent loads (see Task 5.1b)
  - Default active preset: `"de-transcribe"`
  - Active preset fallback: if `active_preset_id` references a nonexistent preset (e.g., deleted custom preset), fall back to the first built-in preset (`"de-transcribe"`) and persist the corrected setting
  - First-run detection: if no settings file exists on startup, this is a first launch. After creating defaults, signal the app to open the settings window automatically and show a notification: "Welcome to Pisum Langue! Please configure an AI provider to get started."
- **Dependencies:** None (can be built in parallel with earlier phases)
- **Acceptance criteria:** Config loads on startup. Defaults created if missing. Save/load roundtrips correctly. Invalid `active_preset_id` falls back to first built-in preset. First launch opens settings window automatically.

#### Task 5.1b: Built-in Presets

- **Files to create/modify:**
  - `src-tauri/src/config/presets.rs` — Built-in preset definitions:
    ```rust
    use crate::config::schema::Preset;

    pub fn get_builtin_presets() -> Vec<Preset> {
        vec![
            Preset {
                id: "de-transcribe".to_string(),
                name: "Transcribe DE".to_string(),
                system_prompt: "Transcribe the following German audio accurately. \
                    Output only the transcription without any additional commentary.".to_string(),
                is_builtin: true,
            },
            Preset {
                id: "en-transcribe".to_string(),
                name: "Transcribe EN".to_string(),
                system_prompt: "Transcribe the following English audio accurately. \
                    Output only the transcription without any additional commentary.".to_string(),
                is_builtin: true,
            },
        ]
    }
    ```
  - `src-tauri/src/config/manager.rs` — On load, merge built-in presets: ensure all built-in presets exist in config (add missing ones, preserve user edits to existing ones)
- **Implementation details:**
  - Built-in presets have `is_builtin: true` — they can be edited (system_prompt changed) but not deleted
  - On config load, call `get_builtin_presets()` and insert any missing built-in presets
  - User-created custom presets have `is_builtin: false` and can be fully managed (create, edit, delete)
  - Preset IDs are kebab-case strings; custom presets get UUID v4 IDs
- **Dependencies:** Task 5.1
- **Acceptance criteria:** First launch creates config with built-in presets. Adding a new built-in preset in code appears on next launch. User edits to built-in preset prompts are preserved.

#### Task 5.2: Tauri Commands for Settings

- **Files to create/modify:**
  - `src-tauri/src/lib.rs` — Add Tauri commands:
    ```rust
    #[tauri::command]
    async fn load_settings() -> Result<AppSettings, String> { ... }

    #[tauri::command]
    async fn save_settings(settings: AppSettings) -> Result<(), String> { ... }

    #[tauri::command]
    async fn test_provider(provider: ProviderConfig) -> Result<bool, String> { ... }

    #[tauri::command]
    async fn get_presets() -> Result<Vec<Preset>, String> { ... }

    #[tauri::command]
    async fn set_active_preset(preset_id: String) -> Result<(), String> { ... }

    #[tauri::command]
    async fn save_preset(preset: Preset) -> Result<(), String> { ... }

    #[tauri::command]
    async fn delete_preset(preset_id: String) -> Result<(), String> { ... }
    ```
- **Dependencies:** Task 5.1, 5.1b
- **Acceptance criteria:** Frontend can call each command and receive typed responses. Preset commands correctly CRUD presets. Deleting a built-in preset returns an error.

#### Task 5.3: TypeScript Types & Command Wrappers

- **Files to create/modify:**
  - `src/lib/types.ts` — Mirror Rust config schema:
    ```typescript
    export interface AppSettings {
      startWithSystem: boolean;
      showTrayNotifications: boolean;
      hotkey: HotkeyBinding;
      audioFormat: 'opus' | 'wav';
      presets: Preset[];
      activePresetId: string;
      providers: ProviderConfig[];
    }
    export interface Preset {
      id: string;
      name: string;
      systemPrompt: string;
      isBuiltin: boolean;
    }
    export interface ProviderConfig { ... }
    export interface HotkeyBinding { modifiers: string[]; key: string; }
    ```
  - `src/lib/commands.ts` — Typed invoke wrappers:
    ```typescript
    import { invoke } from '@tauri-apps/api/core';
    export async function loadSettings(): Promise<AppSettings> { return invoke('load_settings'); }
    export async function saveSettings(settings: AppSettings): Promise<void> { ... }
    export async function testProvider(provider: ProviderConfig): Promise<boolean> { ... }
    export async function getPresets(): Promise<Preset[]> { return invoke('get_presets'); }
    export async function setActivePreset(presetId: string): Promise<void> { ... }
    export async function savePreset(preset: Preset): Promise<void> { ... }
    export async function deletePreset(presetId: string): Promise<void> { ... }
    export async function checkConflict(binding: HotkeyBinding): Promise<boolean> { ... }
    export async function checkSystemConflict(binding: HotkeyBinding): Promise<boolean> { ... }
    ```
  - `src/stores/settings.ts` — Svelte writable store:
    ```typescript
    import { writable } from 'svelte/store';
    export const settings = writable<AppSettings | null>(null);
    export async function initSettings() { ... }
    ```
- **Dependencies:** Task 5.2
- **Acceptance criteria:** Types match Rust schemas. Commands compile and communicate correctly.

#### Task 5.4: Settings UI Components

- **Files to create/modify:**
  - `src/App.svelte` — Load config on mount, render SettingsPage
  - `src/components/SettingsPage.svelte` — Main layout with sections: Hotkey, Audio, Provider, Presets, General
  - `src/components/HotkeyConfig.svelte` — Hotkey display + "Record New Hotkey" button that opens inline HotkeyRecorder
  - `src/components/HotkeyRecorder.svelte` — Hotkey capture widget (based on reference repo pattern):
    - Enters recording mode on click, captures `onkeydown`/`onkeyup` events
    - Tracks modifiers (Ctrl, Alt, Shift, Meta) in real-time, displays them as user presses
    - Requires at least one modifier + a non-modifier key to complete capture
    - Maps key names (single char → uppercase, `Arrow*` → strip prefix)
    - Calls `checkConflict` and `checkSystemConflict` to warn about clashes before saving
    - Shows "Press a key combination..." prompt while recording
  - `src/components/AudioConfig.svelte` — Audio format toggle (Opus/WAV)
  - `src/components/ProviderConfig.svelte` — Provider list: add/remove/edit providers, API key input, model selection, "Test Connection" button
  - `src/components/PresetConfig.svelte` — Preset management:
    - List of all presets (built-in and custom) with active indicator
    - Click to select active preset
    - "Add Preset" button to create custom presets (name + system prompt textarea)
    - Edit button on each preset (opens inline edit with name + system prompt textarea)
    - Delete button on custom presets (built-in presets show delete as disabled)
    - Built-in presets show an indicator badge
- **Implementation details:**
  - Each component binds to the Svelte store, auto-saves on change (debounced 500ms)
  - Tailwind CSS for styling, minimal and functional
  - "Test Connection" calls `testProvider` and shows success/failure feedback
  - PresetConfig uses `getPresets`, `savePreset`, `deletePreset`, `setActivePreset` commands
- **Dependencies:** Task 5.3
- **Acceptance criteria:** All PRD §4.4 configuration options are present and functional. Changes persist across app restarts.

### Phase 6 Tasks: Error Handling, Notifications & Auto-Start

#### Task 6.1: Comprehensive Error Notifications

- **Files to create/modify:**
  - `src-tauri/src/tray.rs` — Ensure `send_notification()` works cross-platform
  - `src-tauri/src/hotkey/manager.rs` — Wrap entire recording/transcription pipeline in error handler:
    ```rust
    fn handle_hotkey_release(app: &AppHandle) {
        let result = std::panic::catch_unwind(|| {
            // ... full pipeline ...
        });
        match result {
            Ok(Ok(())) => { /* success, optionally notify */ },
            Ok(Err(e)) => tray::send_notification("Transcription Error", &e.to_string()),
            Err(_) => tray::send_notification("Unexpected Error", "An unexpected error occurred"),
        }
    }
    ```
- **Implementation details:**
  - Every error in the pipeline (recording start failure, no microphone, encoding error, network failure, API auth error, quota exceeded, clipboard failure, paste failure) must trigger a notification
  - Notification format: title = error category, body = actionable message
  - Error categories: "Recording Error", "Encoding Error", "Transcription Error", "Network Error", "Output Error"
- **Dependencies:** Tasks 4.3, 1.3
- **Acceptance criteria:** Disconnect microphone → notification. Invalid API key → notification. No network → notification. Every error path has a notification.

#### Task 6.2: Auto-Start with OS

- **Files to create/modify:**
  - `src-tauri/src/lib.rs` — Register `tauri-plugin-autostart`:
    ```rust
    use tauri_plugin_autostart::MacosLauncher;

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent, None
        ))
        // ...
    ```
  - `src-tauri/src/lib.rs` — Add Tauri commands:
    ```rust
    #[tauri::command]
    async fn set_autostart(enabled: bool, app: AppHandle) -> Result<(), String> {
        let manager = app.autolaunch();
        if enabled { manager.enable()? } else { manager.disable()? }
        Ok(())
    }
    ```
  - `src/components/SettingsPage.svelte` — Add auto-start toggle in General section
- **Implementation details:**
  - Windows: Registry-based startup via `tauri-plugin-autostart`
  - macOS: LaunchAgent via `tauri-plugin-autostart` with `MacosLauncher::LaunchAgent`
  - Default: enabled (per PRD). User can disable in settings.
- **Dependencies:** Task 5.4
- **Acceptance criteria:** Toggle auto-start in settings. Restart OS → app starts in tray (when enabled). Disable → app does not start.

#### Task 6.3: Maximum Recording Duration Timer

- **Files to create/modify:**
  - `src-tauri/src/hotkey/manager.rs` — Add duration enforcement:
    ```rust
    const MAX_RECORDING_DURATION: Duration = Duration::from_secs(600); // 10 minutes

    fn handle_hotkey_press(app: &AppHandle) {
        // ... start recording ...
        let app_clone = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(MAX_RECORDING_DURATION);
            if is_still_recording() {
                handle_hotkey_release(&app_clone); // Auto-stop
            }
        });
    }
    ```
- **Dependencies:** Task 2.4
- **Acceptance criteria:** Recording auto-stops after 10 minutes. Transcription proceeds normally after auto-stop.

#### Task 6.4: macOS Post-Install Permission Notification

- **Files to create/modify:**
  - `packages/macos/postinstall` — Shell script for macOS installer:
    - Display OS notification guiding user to grant Accessibility permissions in System Settings > Privacy & Security > Accessibility
    - Uses `osascript -e 'display notification ...'`
  - `src-tauri/tauri.conf.json` — Reference postinstall script in macOS bundle config
- **Dependencies:** Task 1.2
- **Acceptance criteria:** After macOS installation, user sees notification about Accessibility permissions.

## 6. Data Model Changes

No database is used. All state is persisted in a single JSON file:

- `~/.pisum-langue.json` — All settings: hotkey, audio format, presets (built-in + custom), active preset ID, provider credentials, auto-start, notifications
- `~/.pisum-langue/logs/` — Rotating log files

## 7. API Changes

No HTTP API is exposed. The application communicates with external APIs:

### Gemini API (outbound)

- **Endpoint:** `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={API_KEY}`
- **Default model:** `gemini-2.5-flash-lite`
- **Auth:** API key as query parameter
- **Request:**
  ```json
  {
    "system_instruction": { "parts": [{ "text": "<system_prompt from active preset>" }] },
    "contents": [{ "parts": [
      { "inline_data": { "mime_type": "audio/ogg", "data": "<base64>" } }
    ]}],
    "generationConfig": { "temperature": 0.1, "maxOutputTokens": 8192 }
  }
  ```
- **Response:** `{ "candidates": [{ "content": { "parts": [{ "text": "..." }] } }] }`

### Tauri IPC Commands (internal)

| Command | Direction | Purpose |
|---------|-----------|---------|
| `load_settings` | Frontend → Backend | Load all settings |
| `save_settings` | Frontend → Backend | Persist all settings |
| `register_hotkey` | Frontend → Backend | Register new hotkey |
| `unregister_hotkey` | Frontend → Backend | Remove current hotkey |
| `check_conflict` | Frontend → Backend | Check hotkey conflict with app |
| `check_system_conflict` | Frontend → Backend | Check hotkey conflict with OS |
| `test_provider` | Frontend → Backend | Test API key validity |
| `get_presets` | Frontend → Backend | Get all presets (built-in + custom) |
| `set_active_preset` | Frontend → Backend | Set active preset by ID |
| `save_preset` | Frontend → Backend | Create or update a preset |
| `delete_preset` | Frontend → Backend | Delete a custom preset (rejects built-in) |
| `set_autostart` | Frontend → Backend | Toggle OS auto-start |

## 8. Dependencies & Risks

### External Dependencies

| Dependency | Version | Purpose | Risk |
|------------|---------|---------|------|
| `tauri` | 2.x | App framework | Mature, actively maintained |
| `global-hotkey` | 0.6+ | System-wide hotkey registration | Verify latest version before implementation; API may differ from 0.6 |
| `cpal` | 0.15 | Audio I/O | Cross-platform, widely used |
| `audiopus` | 0.2+ | Opus encoding | Verify latest version before implementation; wraps libopus; requires C toolchain |
| `rubato` | 0.16 | Audio resampling | Pure Rust, no system deps |
| `ogg` | 0.9 | Ogg container | Stable |
| `hound` | 3.5 | WAV encoding | Stable fallback |
| `reqwest` | 0.12 | HTTP client | Uses rustls (no OpenSSL dep) |
| `arboard` | 3 | Clipboard access | Cross-platform |
| `enigo` | 0.3+ | Input simulation | Verify latest version before implementation; requires accessibility on macOS |
| `tracing` | 0.1 | Structured logging | Mature, widely used |
| `tracing-subscriber` | 0.3 | Log output formatting | Companion to tracing |
| `tracing-appender` | 0.2 | File-based log output with rotation | Companion to tracing |
| `svelte` | 5.x | UI framework | Latest major version |
| `tailwindcss` | 3.x | CSS framework | Stable |
| Gemini API | v1beta | AI transcription via generateContent | Rate limits, requires billing |

### Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| macOS Accessibility permission required for `enigo` | Paste simulation fails silently | Post-install notification guides user to System Settings > Privacy & Security > Accessibility |
| macOS Microphone permission required for `cpal` | Recording fails | macOS prompts on first use; if denied, show error notification guiding user to System Settings > Privacy & Security > Microphone |
| `audiopus` requires C compiler for libopus | Build fails on clean machines | Document build prerequisites (Visual Studio Build Tools on Windows, Xcode CLI Tools + `brew install opus` on macOS); WAV fallback ensures app works even if Opus build fails |
| Gemini API rate limits | Transcription fails under heavy use | Round-robin multiple API keys; exponential backoff; clear error notification |
| Hotkey conflicts with other apps | Hotkey registration fails | Conflict detection against app and system hotkeys; suggest alternative hotkey |
| Large audio files (10 min recording) | API timeout | Opus compression keeps file size manageable (~1.8 MB for 10 min at 24kbps) |

### Assumptions

- User has a working microphone connected
- User has network access for API calls
- User has a Google AI Studio API key with Gemini API access

### Pre-Implementation Checklist

- **Verify crate versions:** Before starting Phase 2, check `crates.io` for the latest versions of `global-hotkey`, `audiopus`, and `enigo`. These crates have had breaking API changes between minor versions. Pin exact versions in `Cargo.toml` after verification.
- **Minimum Rust version:** Target Rust 1.80+ to use `std::sync::LazyLock` from the standard library instead of `once_cell::sync::Lazy`. If targeting Rust < 1.80, keep the `once_cell` dependency.

## 9. Testing Strategy

### Unit Tests (Rust)
- **Audio encoder:** Verify Opus output is valid Ogg/Opus. Verify WAV fallback produces valid WAV. Test resampling from various input rates.
- **Config manager:** Load/save roundtrip. Default creation. Invalid JSON handling. Missing fields get defaults via `#[serde(default)]`. Built-in preset merge on load (missing built-in presets added, user edits preserved).
- **Preset manager:** Create custom preset. Edit preset. Delete custom preset. Reject deletion of built-in preset. Get active preset by ID. Fallback when active preset ID is invalid.
- **Hotkey parser:** Valid hotkey strings parse correctly. Invalid strings return errors.
- **Provider pool:** Round-robin index advances. Fallback skips failed providers. All-fail returns error. Empty pool returns "no providers configured" error.
- **Hotkey conflict:** Detect app-level conflicts. Detect system hotkey conflicts on Windows and macOS.

### Integration Tests (Rust)
- **Gemini provider:** Send known audio file, verify transcription (requires API key; skip in CI without key).
- **Recording + encoding pipeline:** Record silence for 1 second, verify Opus output is non-empty and valid.

### Manual E2E Test Scenarios
- Hold hotkey → speak "hello world" → release → verify text appears at cursor
- Hold hotkey → speak for > 10 minutes → verify auto-stop and transcription
- Disconnect network → hold hotkey → speak → release → verify error notification
- Use invalid API key → verify error notification on transcription attempt
- Change hotkey in settings → verify old hotkey stops working, new one activates
- Test in multiple apps: browser text field, VS Code editor, Notepad, chat applications
- Switch preset in settings UI → dictate → verify transcription uses new preset's prompt
- Create custom preset in settings → verify it appears in preset list
- Delete custom preset → verify it disappears from preset list
- Edit built-in preset prompt → verify edit persists across restart
- Close settings window → verify it hides to tray (does not quit app)
- First launch (delete config file) → verify settings window opens automatically with welcome notification
- Verify tray tooltip shows active preset name (e.g., "Pisum Langue — Transcribe DE")
- Switch active preset → verify tray tooltip updates
- Press hotkey while transcription is in progress → verify "transcription in progress" notification, no crash
- Delete active custom preset → verify fallback to first built-in preset

### Edge Cases
- No microphone connected → error notification on hotkey press
- Empty recording (press and immediately release, < 0.5s) → skip silently, no notification
- Multiple rapid press/release cycles → no crash or resource leak
- Very long utterance (10 min) → encoding and API call succeed
- Non-ASCII transcription results → clipboard and paste handle Unicode correctly

## 10. Requirement Traceability

### Functional Requirements

| PRD Ref | Requirement Summary | Task(s) | Notes |
|---------|-------------------|---------|-------|
| §4.1 #1 | Configurable global hotkey across all apps (Win/macOS) | 2.1, 5.4 | `global-hotkey` crate handles OS-level registration |
| §4.1 #2 | Capture audio from default microphone when hotkey active | 2.2, 2.4 | `cpal` on dedicated thread |
| §4.1 #3 | Encode audio using selected format (Opus default, WAV fallback) | 2.3, 5.1 | User selects format in settings; runtime fallback if encoding fails |
| §4.1 #4 | Stop recording on hotkey release (push-to-talk) | 2.1, 2.4 | Press/release events from `global-hotkey` |
| §4.1 #5 | Max recording duration 10 minutes | 6.3 | Timer thread auto-stops recording |
| §4.1 #6 | Audio/visual feedback for recording state | 1.3, 2.4 | Tray icon changes during recording; tray tooltip shows active preset |
| §4.2 #1 | Send audio + active preset's system prompt to AI provider | 3.2, 3.4 | Gemini with system prompt from active preset |
| §4.2 #2 | Multiple named prompt presets with fallback on invalid active preset | 5.1, 5.1b, 5.4 | `Preset` struct, PresetConfig UI; falls back to first built-in preset |
| §4.2 #3 | Built-in presets for common languages; users can create/edit/delete custom presets | 5.1b, 5.4 | `get_builtin_presets()`, built-in presets editable but not deletable |
| §4.2 #4 | AI provider behind interface (swappable) | 3.1 | `TranscriptionProvider` trait |
| §4.2 #5 | Round-robin distribution with fallback | 3.3 | `ProviderPool` with atomic index |
| §4.3 #1 | Copy transcription to clipboard | 4.1 | `arboard` crate |
| §4.3 #2 | Simulate paste (Ctrl+V / Cmd+V) | 4.2 | `enigo` crate |
| §4.4 #1 | Settings UI from system tray | 1.3, 5.4 | Tray menu → hidden Settings window |
| §4.4 #2 | Persist settings between sessions | 5.1 | Single JSON file in user home directory |
| §4.4 #3 | Start minimized to system tray | 1.3 | Window `visible: false` in tauri.conf.json |
| §4.4 #4 | Auto-start with OS (configurable) | 6.2 | `tauri-plugin-autostart` |
| §4.4 #5 | First-run opens settings, guides provider setup | 5.1, 1.3 | Config manager detects first launch; shows welcome notification |
| §4.5 #1 | Detect network unavailable / API failure | 6.1 | reqwest error handling + retry logic |
| §4.5 #2 | OS-native toast notification on error | 6.1 | `tauri-plugin-notification` |
| §4.5 #3 | No silent error discarding | 6.1 | Every pipeline stage wrapped in error handler |

### User Stories

| PRD Ref | User Story Summary | Implementing Tasks | Fully Covered? |
|---------|-------------------|-------------------|----------------|
| US-1 | Hold hotkey to dictate without switching apps | 2.1, 2.2, 2.4 | Yes |
| US-2 | Transcribed text appears at cursor position | 4.1, 4.2, 4.3 | Yes |
| US-3 | Transcribed text copied to clipboard as fallback | 4.1 | Yes |
| US-4 | Configure AI provider and model | 5.1, 5.4 | Yes |
| US-5 | Switch between prompt presets for different contexts | 5.1b, 5.4 | Yes |

### Success Metrics

| Metric | How the Plan Addresses It |
|--------|--------------------------|
| End-to-end latency < 3s for short utterances | Opus compression minimizes upload size; Gemini flash-lite is fast for short audio; direct clipboard+paste with no intermediate steps |
| Works across common applications | `enigo` for input simulation is application-agnostic; tested in E2E scenarios across browsers, editors, chat apps |
| Swapping AI provider = new class + DI change | `TranscriptionProvider` trait; new provider = implement trait + add to `ProviderPool` |
| Minimal resource usage when idle | Tauri app is lightweight; no background threads when not recording; system tray only |
