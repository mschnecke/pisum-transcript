# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Pisum Langue** is an AI-driven transcription utility. The user holds a global hotkey to record speech, releases it to stop, and the transcribed text is pasted at the cursor position. The project uses **Tauri 2 (Rust backend) + Svelte 5 (TypeScript frontend)**, Vite 6, and Tailwind CSS.

## Repository State

This project is in early development (foundation phase). The `1-foundation` branch is the active development branch. The main branch serves as the stable base.

## Build & Development Commands

```bash
npm install            # Install Node.js dependencies
npm run dev            # Start Vite dev server
npm run tauri:dev      # Start full Tauri app in dev mode
npm run build          # Build frontend only
npm run tauri:build    # Build complete application
```

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.70+
- Platform-specific dependencies:
  - **Windows**: Visual Studio Build Tools
  - **macOS**: Xcode Command Line Tools, Opus (`brew install opus`)

## Architecture

- **Rust backend** (`src-tauri/`): Hotkey registration, audio recording/encoding, AI provider communication, clipboard & paste simulation, system tray
- **Svelte 5 frontend** (`src/`): Settings UI accessible from system tray
- **IPC**: Tauri command system with JSON/serde serialization

## Reference Implementation

The `W:\github-global-hotkey` repo is the reference for cross-platform patterns (hotkey management, audio recording, Tauri setup).
