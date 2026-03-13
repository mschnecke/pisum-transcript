# Pisum Langue

Hotkey-driven, system-tray-style dictation tool. Hold a global hotkey to record speech, release to transcribe via AI, and the result is pasted at the cursor position.

## Tech Stack

- **Backend**: Tauri 2 (Rust)
- **Frontend**: Svelte 5 (TypeScript), Vite 6, Tailwind CSS
- **AI Provider**: Google Gemini (configurable)

## Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.70+
- Platform-specific:
  - **Windows**: Visual Studio Build Tools
  - **macOS**: Xcode Command Line Tools, Opus (`brew install opus`)

## Development

```bash
npm install            # Install dependencies
npm run tauri:dev      # Start app in dev mode
npm run tauri:build    # Build application
```

## macOS Permissions

- **Accessibility**: Required for paste simulation. Grant in System Settings > Privacy & Security > Accessibility.
- **Microphone**: Required for audio recording. macOS will prompt on first use.
