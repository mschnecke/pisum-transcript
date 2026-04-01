# Implementation Plan: Remove OpenAI Cloud Provider

> Generated from: `docs/PRD-remove-openai-provider.md`
> Date: 2026-04-01

## 1. Overview

This plan removes the OpenAI cloud transcription provider from the application to simplify the codebase. Gemini remains as the sole cloud provider alongside local Whisper. The existing provider abstraction (trait, pool, round-robin) is preserved for future extensibility.

The change touches 7 files across the Rust backend and Svelte frontend, plus one file deletion. A settings migration silently strips OpenAI entries from existing user configurations on load.

## 2. Architecture & Design

**Current state:**
```
ProviderType enum: Gemini | OpenAi
ProviderPool: [Box<dyn TranscriptionProvider>] — round-robin over Gemini + OpenAI instances
Frontend dropdown: "Gemini" | "OpenAI"
```

**Target state:**
```
ProviderType enum: Gemini
ProviderPool: [Box<dyn TranscriptionProvider>] — round-robin over Gemini instances (unchanged mechanism)
Frontend dropdown: "Gemini" (single option, dropdown preserved)
```

**Settings migration flow:**
```
Load JSON from disk
  → Deserialize with serde_json::Value (raw JSON)
  → Filter out provider entries where provider_type == "openai"
  → Deserialize remaining into AppSettings
  → Persist cleaned settings back to disk
```

This raw-JSON approach avoids serde deserialization failures when encountering the removed `"openai"` variant.

## 3. Phases & Milestones

### Phase 1: Backend — Remove OpenAI Code
**Goal:** Eliminate all OpenAI-specific Rust code while keeping the build green.
**Deliverable:** `cargo build` succeeds with no OpenAI references or warnings.

### Phase 2: Settings Migration
**Goal:** Existing user configs with OpenAI entries load without errors and are silently cleaned.
**Deliverable:** Loading a settings file containing `"openai"` providers filters them out and persists the cleaned version.

### Phase 3: Frontend Cleanup
**Goal:** The settings UI no longer references OpenAI.
**Deliverable:** `npm run check` passes, dropdown shows only "Gemini".

## 4. Files Overview

### Files to Delete
| File Path | Purpose |
|-----------|---------|
| `src-tauri/src/ai/openai.rs` | OpenAI provider implementation (385 lines) |

### Files to Modify
| File Path | What Changes |
|-----------|-------------|
| `src-tauri/src/ai/mod.rs` | Remove `pub mod openai;` declaration |
| `src-tauri/src/ai/pool.rs` | Remove OpenAI import and both match arms in `rebuild()` and `test_provider()` |
| `src-tauri/src/config/schema.rs` | Remove `OpenAi` variant from `ProviderType` enum |
| `src-tauri/src/lib.rs` | Remove OpenAI import and all OpenAI match arms in `test_provider_connection()`, `list_provider_models()`, `apply_settings()`, and `setup()` |
| `src-tauri/src/config/manager.rs` | Add pre-deserialization filter for `"openai"` provider entries |
| `src/lib/types.ts` | Remove `'openai'` from `providerType` union |
| `src/components/ProviderConfig.svelte` | Remove OpenAI dropdown option, type cast, and default model text |
| `docs/PRD-whisper-local-transcription.md` | Update references from "Gemini/OpenAI" to "Gemini" |

## 5. Task Breakdown

### Phase 1: Backend — Remove OpenAI Code

#### Task 1.1: Delete OpenAI Provider Module
- **Files to modify:**
  - `src-tauri/src/ai/openai.rs` — delete entirely
  - `src-tauri/src/ai/mod.rs` — remove line `pub mod openai;`
- **Implementation details:**
  - Delete `openai.rs`
  - In `mod.rs`, change from:
    ```rust
    pub mod gemini;
    pub mod openai;
    pub mod pool;
    pub mod provider;
    pub mod whisper;
    ```
    To:
    ```rust
    pub mod gemini;
    pub mod pool;
    pub mod provider;
    pub mod whisper;
    ```
- **Dependencies:** None
- **Acceptance criteria:** Module compiles without the openai module (downstream references will break — fixed in subsequent tasks)

#### Task 1.2: Remove OpenAi Variant from ProviderType Enum
- **Files to modify:**
  - `src-tauri/src/config/schema.rs` — remove `OpenAi` variant
- **Implementation details:**
  - Change the `ProviderType` enum from:
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "lowercase")]
    pub enum ProviderType {
        Gemini,
        #[serde(rename = "openai")]
        OpenAi,
    }
    ```
    To:
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "lowercase")]
    pub enum ProviderType {
        Gemini,
    }
    ```
- **Dependencies:** None
- **Acceptance criteria:** Enum compiles; all downstream match arms will need updating (next tasks)

#### Task 1.3: Update Provider Pool
- **Files to modify:**
  - `src-tauri/src/ai/pool.rs` — remove import and match arms
- **Implementation details:**
  - Remove import: `use super::openai::OpenAiProvider;`
  - In `rebuild()` method, remove the OpenAI match arm:
    ```rust
    "openai" | "OpenAi" => {
        let provider = OpenAiProvider::new(entry.api_key.clone(), entry.model.clone());
        self.providers.push(Box::new(provider));
    }
    ```
  - In `test_provider()` method, remove the OpenAI match arm:
    ```rust
    "openai" | "OpenAi" => {
        let provider = OpenAiProvider::new(entry.api_key.clone(), entry.model.clone());
        provider.test_connection().await
    }
    ```
- **Dependencies:** Task 1.1 (openai module deleted)
- **Acceptance criteria:** Pool compiles without OpenAI references; only creates Gemini provider instances

#### Task 1.4: Update lib.rs Commands and Initialization
- **Files to modify:**
  - `src-tauri/src/lib.rs` — remove OpenAI import and all match arms
- **Implementation details:**
  - Remove import: `use ai::openai::OpenAiProvider;`
  - In `test_provider_connection()`, remove the `ProviderType::OpenAi => "openai"` match arm
  - In `list_provider_models()`, remove the `"openai"` match arm that calls `OpenAiProvider::list_models()`
  - In `apply_settings()`, remove the `ProviderType::OpenAi => "openai"` match arm
  - In `setup()` function, remove the `ProviderType::OpenAi => "openai"` match arm
  - Since `ProviderType` now only has `Gemini`, each match can either remain as a single-arm match or be simplified. Keep the match statement structure for future extensibility.
- **Dependencies:** Task 1.2 (enum variant removed)
- **Acceptance criteria:** `cargo build` succeeds with no warnings about unused imports or unreachable patterns

### Phase 2: Settings Migration

#### Task 2.1: Add Pre-Deserialization Filter in Config Manager
- **Files to modify:**
  - `src-tauri/src/config/manager.rs` — add filter before typed deserialization
- **Implementation details:**
  - In the settings loading function, after reading the JSON string from disk but before deserializing into `AppSettings`:
    1. Parse the raw JSON string into `serde_json::Value`
    2. Access the `"providers"` array
    3. Filter out entries where `"providerType"` (or `"provider_type"`) equals `"openai"`
    4. Replace the providers array with the filtered version
    5. Deserialize the cleaned `Value` into `AppSettings`
    6. If any entries were removed, persist the cleaned settings back to disk
  - The migration must be completely silent — no logging, no user notification.
  - Pseudocode (insert in `load_settings()` between reading the file and the `serde_json::from_str` call):
    ```rust
    let mut raw: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| AppError::Config(format!("Failed to parse settings: {}", e)))?;

    // Silently filter out removed provider types (e.g., OpenAI)
    let needs_save = if let Some(providers) = raw.get_mut("providers").and_then(|v| v.as_array_mut()) {
        let original_len = providers.len();
        providers.retain(|p| {
            p.get("providerType")
                .and_then(|v| v.as_str())
                .map(|t| t != "openai")
                .unwrap_or(true)
        });
        providers.len() != original_len
    } else {
        false
    };

    let mut settings: AppSettings = serde_json::from_value(raw)
        .map_err(|e| AppError::Config(format!("Failed to parse settings: {}", e)))?;

    if needs_save {
        save_settings(&settings)?;
    }
    ```
  - **Confirmed:** The JSON key is `"providerType"` (camelCase) — verified from `ProviderConfig` struct's `#[serde(rename_all = "camelCase")]` attribute at `src-tauri/src/config/schema.rs:56`.
  - This replaces the existing `serde_json::from_str` call at line 42 of `manager.rs`. The raw JSON parse + filter + `from_value` pattern ensures `"openai"` entries are stripped before typed deserialization attempts to parse the `ProviderType` enum.
- **Dependencies:** Task 1.2 (enum no longer accepts `"openai"`, so raw filtering is necessary)
- **Acceptance criteria:** A settings file containing `{"providerType": "openai", ...}` entries loads successfully; OpenAI entries are stripped; cleaned file is persisted

### Phase 3: Frontend Cleanup

#### Task 3.1: Update TypeScript Types
- **Files to modify:**
  - `src/lib/types.ts` — update union type
- **Implementation details:**
  - Change `providerType: 'gemini' | 'openai'` to `providerType: 'gemini'`
- **Dependencies:** None
- **Acceptance criteria:** TypeScript compiles with no type errors referencing `'openai'`

#### Task 3.2: Update Provider Config Component
- **Files to modify:**
  - `src/components/ProviderConfig.svelte` — remove OpenAI option and default
- **Implementation details:**
  - Remove the OpenAI option from the provider type dropdown (line 127):
    ```svelte
    <!-- Remove this line: -->
    <option value="openai">OpenAI</option>
    ```
  - Update the type cast in the `onchange` handler (line 117):
    ```svelte
    <!-- From: -->
    const newType = e.currentTarget.value as 'gemini' | 'openai';
    <!-- To: -->
    const newType = e.currentTarget.value as 'gemini';
    ```
  - Update the default model display text (lines 197–199). Change from conditional OpenAI/Gemini text to just the Gemini default:
    ```svelte
    <!-- From: -->
    {provider.providerType === 'openai'
        ? 'Default (gpt-4o-mini-audio-preview)'
        : 'Default (gemini-2.5-flash-lite)'}
    <!-- To: -->
    Default (gemini-2.5-flash-lite)
    ```
- **Dependencies:** Task 3.1 (type change may trigger errors if `'openai'` is still referenced)
- **Acceptance criteria:** `npm run check` passes; dropdown shows only "Gemini"; default model text is unconditional

#### Task 3.3: Update Documentation References
- **Files to modify:**
  - `docs/PRD-whisper-local-transcription.md` — update OpenAI references
- **Implementation details:**
  - Line 5: Change "cloud AI providers (Gemini, OpenAI)" to "a cloud AI provider (Gemini)"
  - Line 35: Change "existing cloud provider pool (Gemini/OpenAI)" to "existing cloud provider pool (Gemini)"
- **Dependencies:** None
- **Acceptance criteria:** `rg -i "openai" docs/` returns only the PRD and IMPL for this feature

## 6. Data Model Changes

No database changes. The only data change is to the user settings file (`~/.pisum-transcript.json`), handled by the migration filter in Task 2.1.

## 7. API Changes

No external API changes. The Tauri IPC commands `test_provider_connection` and `list_provider_models` remain but no longer accept `"openai"` as a provider type. This is a breaking change only for the internal frontend-to-backend contract, which is updated simultaneously in Phase 3.

## 8. Dependencies & Risks

- **No OpenAI-specific Cargo dependencies** exist — both providers share `reqwest`, `serde`, and `base64`. No dependency cleanup needed. (PRD open question resolved.)
- **Settings migration must be completely silent** — no logging, no notification. (PRD open question resolved.)
- **Risk: serde deserialization of legacy settings** — If the raw-JSON filtering approach is not implemented correctly, users with existing OpenAI configs will get a deserialization error on app startup. Mitigation: Task 2.1 must be tested with a settings file containing OpenAI entries.
- **Risk: exhaustive match warnings** — After removing the `OpenAi` variant, matches on `ProviderType` with only `Gemini` may trigger "irrefutable pattern" warnings. Mitigation: Keep match statements (compiler will guide cleanup).
- **Confirmed:** The JSON settings file uses `"providerType"` (camelCase) as the key name — verified from `ProviderConfig` struct's `#[serde(rename_all = "camelCase")]` attribute.

## 9. Testing Strategy

Since this project has no automated test suite, verification is manual:

- **Build verification:** `cargo build` completes without warnings; `npm run check` passes; `npm run tauri:build` succeeds.
- **Settings migration test:**
  1. Create a `~/.pisum-transcript.json` with an OpenAI provider entry
  2. Launch the app
  3. Verify the OpenAI entry is removed from the file
  4. Verify the app starts without errors
- **Settings migration — OpenAI-only config test:**
  1. Create settings with ONLY an OpenAI provider (no Gemini)
  2. Launch the app
  3. Verify the providers list is empty and the app doesn't crash
- **UI verification:**
  1. Open settings
  2. Verify provider type dropdown shows only "Gemini"
  3. Add a new provider — verify it defaults to Gemini
  4. Verify model default text shows "Default (gemini-2.5-flash-lite)"
- **Grep verification:** `rg -i "openai" src-tauri/src/ src/` returns no matches (confirming full removal)

## 10. Requirement Traceability

### Functional Requirements

| PRD Ref | Requirement Summary | Task(s) | Notes |
|---------|-------------------|---------|-------|
| 4.1 #1 | Delete `openai.rs` | 1.1 | |
| 4.1 #2 | Remove `pub mod openai` from ai module | 1.1 | |
| 4.1 #3 | Remove `OpenAi` variant from `ProviderType` enum | 1.2 | |
| 4.1 #4 | Remove OpenAI match arm from pool `rebuild()` | 1.3 | |
| 4.1 #5 | Remove OpenAI match arm from `list_provider_models` | 1.4 | |
| 4.1 #6 | Remove OpenAI-specific imports | 1.1, 1.3, 1.4 | Covered across all backend tasks |
| 4.2 #7 | Filter out OpenAI entries on settings load | 2.1 | Raw JSON filtering approach |
| 4.2 #8 | Persist cleaned settings to disk | 2.1 | Conditional save when entries removed |
| 4.3 #9 | Remove OpenAI from dropdown, keep dropdown visible | 3.2 | |
| 4.3 #10 | Remove OpenAI default model from frontend | 3.2 | |
| 4.3 #11 | Update `providerType` union type | 3.1 | |
| 4.4 #12 | Remove OpenAI comments and dead code | 1.1–1.4, 3.1–3.3 | Includes docs cleanup in 3.3 |
| 4.4 #13 | Cargo builds without warnings | 1.4 | Final verification step |

### User Stories

| PRD Ref | User Story Summary | Implementing Tasks | Fully Covered? |
|---------|-------------------|-------------------|----------------|
| US-1 | Developer: fewer providers to maintain | 1.1–1.4 | Yes |
| US-2 | User: settings silently cleaned on upgrade | 2.1 | Yes |
| US-3 | User: UI only shows supported providers | 3.1–3.2 | Yes |

### Success Metrics

| Metric | How the Plan Addresses It |
|--------|--------------------------|
| `npm run tauri:build` succeeds | Task 1.4 final verification; Phase 3 frontend cleanup |
| `npm run check` passes | Task 3.1, 3.2 |
| Existing OpenAI settings silently cleaned | Task 2.1 |
| Provider pool works with Gemini only | Task 1.3 |
| UI shows only "Gemini" | Task 3.2 |
