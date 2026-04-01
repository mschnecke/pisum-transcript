# PRD: Remove OpenAI Cloud Provider

## 1. Introduction/Overview

The application currently supports two cloud transcription providers: Gemini and OpenAI. This feature removes the OpenAI provider entirely to simplify the codebase. Gemini and local Whisper provide sufficient transcription coverage, making the OpenAI integration unnecessary maintenance overhead.

## 2. Goals

- Remove all OpenAI-specific code from the Rust backend (client, API calls, model filtering)
- Remove OpenAI as a selectable option in the frontend settings UI
- Silently clean up any existing OpenAI provider entries from user configuration on load
- Preserve the existing provider abstraction (trait, pool, enum pattern) for future extensibility

## 3. User Stories

- As a developer, I want fewer provider implementations to maintain so that the codebase stays lean and focused.
- As a user with an existing OpenAI configuration, I want my settings to be silently cleaned up on upgrade so that I don't see broken or unusable provider entries.
- As a user configuring a new cloud provider, I want the UI to only show providers that are actually supported so that I'm not confused by non-functional options.

## 4. Functional Requirements

### 4.1 Backend — Remove OpenAI Provider Implementation

1. Delete `src-tauri/src/ai/openai.rs` entirely.
2. Remove the `pub mod openai;` declaration from the `ai` module file.
3. Remove the `OpenAi` variant from the `ProviderType` enum in `src-tauri/src/config/schema.rs`.
4. Remove the OpenAI match arm from the provider pool factory logic in `src-tauri/src/ai/pool.rs` (the `rebuild()` method that creates provider instances based on `provider_type`).
5. Remove the OpenAI match arm from the `list_provider_models` command in `src-tauri/src/lib.rs`.
6. Remove any OpenAI-specific imports throughout the backend.

### 4.2 Backend — Settings Migration

7. When loading settings from disk (`config/manager.rs`), filter out any `ProviderConfig` entries where `provider_type` is `"openai"`. This ensures existing user configurations are silently cleaned up.
8. After filtering, persist the cleaned settings back to disk so the cleanup is permanent.

### 4.3 Frontend — Update Provider UI

9. In `src/components/ProviderConfig.svelte`, remove `"openai"` from the provider type dropdown options. The dropdown should only show `"Gemini"` but remain as a dropdown element (preserving UI structure for future providers).
10. Remove the OpenAI default model (`"gpt-4o-mini-audio-preview"`) from any model defaults or fallback logic in the frontend.
11. In `src/lib/types.ts`, update the `ProviderConfig.providerType` type from `'gemini' | 'openai'` to `'gemini'`.

### 4.4 Cleanup

12. Remove any OpenAI-related comments, documentation references, or dead code paths across the codebase.
13. Verify that Cargo builds without warnings related to unused imports or dead code after removal.

## 5. Non-Goals (Out of Scope)

- Not included: Adding a replacement cloud provider (e.g., Anthropic, Deepgram).
- Not included: Simplifying or removing the provider abstraction layer (trait, pool, round-robin). The abstraction stays as-is for future extensibility.
- Not included: Changes to the local Whisper transcription engine.
- Not included: User-facing notifications or migration dialogs about the removal. Cleanup is silent.

## 6. Design Considerations

- The provider type dropdown in the settings UI should remain visible (with only "Gemini" as an option) to preserve the UI layout and make it easy to add new providers in the future.
- No other UI changes are needed since the provider configuration form (API key, model selection, enable/disable) is shared across provider types.

## 7. Technical Considerations

- **Provider pool rebuild**: After filtering out OpenAI entries from settings, the provider pool must be rebuilt. Ensure the pool handles the case where all providers have been removed (e.g., user only had OpenAI configured).
- **Serde deserialization**: When loading settings JSON that contains `"openai"` as a `provider_type`, the deserialization must not fail. Two approaches:
  - Filter at the JSON/serde level using a custom deserializer or `#[serde(other)]` on the enum.
  - Deserialize with a lenient approach (e.g., keep `ProviderType` temporarily accepting `"openai"` during load, then filter in the manager).
  - The simpler approach: deserialize the raw JSON, filter entries with `provider_type == "openai"`, then parse the remaining entries into the typed config. Choose whichever approach is cleanest.
- **Cargo dependencies**: Check if removing OpenAI removes any crate dependencies that are no longer needed (e.g., if OpenAI used specific HTTP client features or serialization helpers not used by Gemini).
- **Affected files summary**:
  - `src-tauri/src/ai/openai.rs` — delete
  - `src-tauri/src/ai/mod.rs` — remove module declaration
  - `src-tauri/src/ai/pool.rs` — remove OpenAI factory arm
  - `src-tauri/src/config/schema.rs` — remove `OpenAi` enum variant
  - `src-tauri/src/config/manager.rs` — add migration filter
  - `src-tauri/src/lib.rs` — remove OpenAI match arm in `list_provider_models`
  - `src/lib/types.ts` — update union type
  - `src/components/ProviderConfig.svelte` — remove OpenAI from dropdown and defaults

## 8. Success Metrics

- `npm run tauri:build` completes without errors or warnings related to OpenAI.
- `npm run check` passes with no type errors.
- Existing user settings files containing OpenAI providers are silently cleaned on first load after upgrade.
- The provider pool functions correctly with only Gemini providers configured.
- The settings UI shows only "Gemini" in the provider type dropdown.

## 9. Open Questions

- [x] Are there any OpenAI-specific Cargo dependencies that can be removed to reduce binary size? -> No, all dependencies are still needed.
- [x] Should the settings migration (filtering OpenAI entries) log a debug/info message for troubleshooting, or be completely silent? -> completly silent
