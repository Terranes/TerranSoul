# LM Studio Full-Parity Design

Date: 2026-04-23
Status: Draft for review

## Goal

Upgrade TerranSoul's LM Studio integration from a manual `model + base_url` text entry into a full local-model workflow comparable to Ollama. Users should be able to search, download, load, and activate any LM Studio model from inside the desktop app instead of being limited to a default typed model.

## User Intent

The user wants LM Studio to behave "like Ollama":

- Pick any model, not just a hard-coded default
- Download models from inside TerranSoul
- Load and switch models from inside TerranSoul
- Avoid frontend changes unless they are necessary

## Current State

The current LM Studio integration exists as a persisted brain mode:

- `local_lm_studio` is stored in the frontend and backend brain-mode config
- Chat routing already supports LM Studio through its OpenAI-compatible server
- The UI only exposes two free-text inputs: `model` and `base_url`
- There is no backend support for searching, downloading, listing, or loading LM Studio models

Relevant files:

- `src/views/MarketplaceView.vue`
- `src/views/BrainSetupView.vue`
- `src/stores/brain.ts`
- `src/stores/brain.test.ts`
- `src-tauri/src/commands/brain.rs`
- `src-tauri/src/brain/brain_config.rs`

## Constraints

- Preserve the existing visual layout unless a small additive UI change is required
- Keep existing LM Studio chat routing intact
- Do not regress the current Ollama flow
- Desktop is the primary target; browser fallback may continue to support only local state

## Chosen Approach

Use a CLI-first LM Studio integration.

TerranSoul will use the installed `lms` CLI for model management:

- `lms get` for search/download
- `lms ls` for downloaded models on disk
- `lms ps` for models currently loaded in memory
- `lms load` for loading and activating a model

TerranSoul will continue using LM Studio's OpenAI-compatible server for chat completions after the model is loaded.

This approach is preferred because:

- It provides true download/load behavior rather than only listing already-served models
- It matches the user's "like Ollama" expectation
- It avoids rewriting the inference path
- It keeps UI changes localized to the existing LM Studio panel

## Alternatives Considered

### 1. Server-only integration

Use only the OpenAI-compatible `/v1/models` endpoint and never invoke `lms`.

Rejected because it cannot provide full download parity and only sees models the server already exposes.

### 2. Hybrid server-first with CLI fallback

Prefer server APIs and use the CLI only when necessary.

Rejected for the first version because it adds complexity without reducing required CLI support for download/load operations.

### 3. Rich cached LM Studio manager

Introduce a larger abstraction with background sync, cache reconciliation, and install-state normalization.

Deferred because it is more complexity than needed for the first parity pass.

## Architecture

### Backend

Add a new LM Studio backend module parallel to the existing Ollama integration. This module will:

- locate and invoke `lms`
- normalize CLI output into typed Rust structs
- return friendly errors when LM Studio or its daemon is unavailable
- expose Tauri commands for the frontend

Planned backend commands:

- `check_lm_studio_status`
- `search_lm_studio_models`
- `get_lm_studio_downloaded_models`
- `get_lm_studio_loaded_models`
- `download_lm_studio_model`
- `load_lm_studio_model`

These commands will be registered in `src-tauri/src/lib.rs` and implemented in `src-tauri/src/commands/brain.rs` or a dedicated LM Studio command module if that keeps the file clearer.

### Frontend Store

Extend `useBrainStore()` with LM Studio state analogous to the Ollama flow:

- LM Studio availability/status
- search results
- downloaded models
- loaded models
- in-flight action state
- last error for LM Studio operations

Planned store methods:

- `checkLmStudioStatus()`
- `searchLmStudioModels(query: string)`
- `fetchLmStudioDownloadedModels()`
- `fetchLmStudioLoadedModels()`
- `downloadLmStudioModel(modelKey: string)`
- `loadLmStudioModel(modelKey: string, identifier?: string)`

Activation flow:

1. load the selected LM Studio model through the backend
2. persist `set_brain_mode({ mode: 'local_lm_studio', model, base_url })`
3. update store state and confirmation UI

The app should never persist LM Studio mode before the load step succeeds.

### Frontend Views

Keep the current LM Studio sections in:

- `src/views/MarketplaceView.vue`
- `src/views/BrainSetupView.vue`

Replace the current free-text-only behavior with a minimally expanded workflow:

- searchable model input or search bar
- list of matching downloadable models
- list of downloaded models
- list or indicator of currently loaded models
- `Download` and `Load & Activate` actions

The existing `base_url` field remains because chat still needs a target server URL.

## Data Model

The existing `BrainMode::LocalLmStudio { model, base_url }` shape can remain unchanged for the first pass.

The new backend commands will likely introduce additional transport types:

- `LmStudioStatus`
- `LmStudioSearchResult`
- `LmStudioDownloadedModel`
- `LmStudioLoadedModel`

These should live close to the backend integration and be mirrored in `src/types/index.ts`.

## CLI Behavior

### Search and download

The first implementation will support search/download through a direct model-key workflow built around `lms get`.

Behavior:

- the UI provides a searchable text input for the model key
- the user can type any LM Studio model key, such as a hub name or supported direct reference
- TerranSoul passes that value to `lms get`
- downloaded models from `lms ls` and loaded models from `lms ps` are shown as selectable local options

This means the first pass explicitly supports:

- choosing any model by entering its key
- downloading that model from inside TerranSoul
- selecting already-downloaded models from a local list
- loading and activating the selected model

This first pass does not require TerranSoul to scrape or infer a rich remote search-result list from human-oriented CLI output. If LM Studio later exposes a stable structured search surface, richer remote suggestions can be added without changing the core workflow.

### List downloaded models

Use `lms ls` to populate models already available on disk.

### List loaded models

Use `lms ps` to show which models are currently loaded.

### Load and activate

Use `lms load <model-key>` to load the chosen model. After success:

- update the current LM Studio lists
- persist `local_lm_studio` brain mode with the loaded model id
- show activation confirmation in the UI

## Error Handling

LM Studio failures must not block other provider modes.

Expected failure cases:

- `lms` is not installed or not on `PATH`
- the LM Studio daemon is not running
- the local server is not reachable at the chosen `base_url`
- CLI output is malformed or changes shape
- download/load operations time out or fail

Rules:

- failed download does not change the active brain mode
- failed load does not persist `local_lm_studio`
- failed refresh of model lists leaves previous UI state intact when reasonable
- errors are surfaced inline in the LM Studio section with human-readable messages

## Base URL Handling

TerranSoul already chats with LM Studio through an OpenAI-compatible endpoint. The integration should normalize user input so that:

- `http://127.0.0.1:1234`
- `http://127.0.0.1:1234/`
- `http://127.0.0.1:1234/v1`

can all be stored or resolved consistently for chat usage.

The exact normalization rule should be chosen during implementation, documented in tests, and applied in one place only.

## Testing Strategy

### Rust tests

Add unit tests for:

- CLI invocation wrappers
- output parsing
- error mapping for missing CLI, timeout, and bad output
- load/download command success and failure handling

Where possible, isolate parsing from real process execution so tests remain fast and deterministic.

### Store tests

Extend `src/stores/brain.test.ts` to verify:

- LM Studio commands update store state correctly
- load success persists `local_lm_studio`
- load failure does not persist `local_lm_studio`
- downloaded and loaded model lists are refreshed after actions

### View tests

Extend `src/views/MarketplaceView.test.ts` to verify:

- the LM Studio tab shows non-default model options
- download can be triggered from the UI
- load and activate works for a user-selected model
- the confirmation reflects the chosen model rather than a default placeholder

Add or extend setup-wizard coverage if the LM Studio onboarding view is changed materially.

## Non-Goals

- Replacing the LM Studio chat transport
- Reworking the overall LLM settings layout
- Adding browser-only support for true LM Studio CLI workflows
- Introducing background download progress streaming in the first pass

## Implementation Notes

- Prefer additive changes to the existing LM Studio UI instead of new screens
- Mirror the Ollama code structure where it improves consistency
- Keep frontend changes minimal but sufficient for search/download/load
- Avoid coupling LM Studio state to Ollama-specific types

## Open Decision Resolved

The user explicitly selected full parity:

- search/download models
- list local models
- list loaded models
- load and switch models

This design therefore targets full desktop parity rather than a partial server-only integration.

## Expected Outcome

After implementation, a desktop user should be able to:

1. open the existing LM Studio section
2. pick or type any LM Studio model
3. download it from inside TerranSoul
4. load it into LM Studio
5. activate it as the current brain
6. chat through the same LM Studio-backed brain mode without extra manual setup
