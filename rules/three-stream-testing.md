# Three-Stream Synchronization Testing

> Rule added: 2026-05-09

## What are the Three Streams?

TerranSoul has three concurrent output streams during an LLM response:

| # | Stream     | Rust event      | JS consumer                | Purpose                    |
|---|-----------|-----------------|---------------------------|----------------------------|
| 1 | **Text**      | `llm-chunk`     | `streaming.handleChunk()` | Chat text in UI            |
| 2 | **Animation** | `llm-animation` | `streaming.handleAnimation()` | Face emotion + body motion |
| 3 | **Voice**     | `tts.feedChunk()` | `useTtsPlayback` composable | Audio + lip sync           |

## State Machine Lifecycle

Every response follows this state machine. Tests MUST validate transitions:

```
idle → thinking → streaming/talking → [TTS speaking] → final emotion → idle
```

Key invariants:

1. **`isStreaming`**: `false→true` only on first `text` chunk; `true→false` only on `done:true`
2. **`currentEmotion`**: persists from `handleAnimation()` until `reset()` — never cleared by `done:true`
3. **`isSpeaking`**: `true` after TTS synthesis starts; `false` only after ALL audio finishes
4. **Body state**: stays `'talk'` during streaming AND TTS; only returns to `'idle'` when `isSpeaking` becomes `false`
5. **Face emotion**: updated in real-time via `llm-animation` events (does NOT override body state)
6. **Final emotion**: applied by the `isSpeaking` watcher when TTS finishes, using `streaming.currentEmotion`

## Testing Methodology (No LLM Required)

### Unit tests validate the three-stream contract WITHOUT a running LLM

Test file: `src/composables/useThreeStreamSync.test.ts`

The tests simulate the exact event sequence Rust emits:

```typescript
// Simulate what Rust's StreamTagParser emits:
streaming.handleAnimation({ emotion: 'happy', intensity: 0.8 });
streaming.handleChunk({ text: 'Hello! ', done: false });
tts.feedChunk('Hello! ');
streaming.handleChunk({ text: '', done: true });
tts.flush();
```

### What to test (checklist for new streaming features):

- [ ] Text accumulates correctly across all chunks
- [ ] Animation events update `currentEmotion` without resetting text
- [ ] `done:true` stops streaming but preserves emotion for TTS watcher
- [ ] TTS synthesizes complete sentences, not partial tokens
- [ ] TTS `isSpeaking` outlives `isStreaming` (voice finishes after text)
- [ ] Emotion is available to the `isSpeaking` watcher after stream ends
- [ ] `reset()` clears ALL state for the next turn
- [ ] Multiple turns don't leak state
- [ ] Empty stream doesn't crash
- [ ] TTS failure doesn't block streaming
- [ ] `stop()` cancels pending TTS and allows clean restart

### How to run

```bash
# Run only three-stream sync tests:
npx vitest run src/composables/useThreeStreamSync.test.ts

# Run all streaming-related tests:
npx vitest run --reporter=verbose src/stores/streaming.test.ts src/composables/useTtsPlayback.test.ts src/composables/useThreeStreamSync.test.ts
```

## Anti-patterns (DO NOT)

1. **DO NOT** use `setTimeout(() => setAvatarState('idle'), N)` during streaming
   — it fires blindly and overrides TTS talking state.

2. **DO NOT** apply final emotion in the `isStreaming` watcher if TTS is still
   speaking — the `isSpeaking` watcher handles the final transition.

3. **DO NOT** call `tts.flush()` in the `isStreaming` watcher — the `llm-chunk
   done:true` handler already flushes. Double-flushing speaks the last
   sentence twice.

4. **DO NOT** set body state to `'idle'` anywhere except the `isSpeaking`
   watcher (when `speaking` becomes `false`).

5. **DO NOT** test streaming sync with a running LLM in CI. Use the mock
   event sequence. LLM-dependent tests go in `e2e/` with proper service
   guards.

## View-specific watcher responsibilities

### ChatView / PetOverlayView

| Watcher | Fires when | Responsibility |
|---------|-----------|---------------|
| `isThinking` | User sends message | Set body to `'thinking'` |
| `isStreaming` (true) | First text chunk | Set body to `'talking'` |
| `isStreaming` (false) | `done:true` | Apply final emotion ONLY IF `!tts.isSpeaking` |
| `currentEmotion` | Animation event | Update face blend shapes only (no body change) |
| `isSpeaking` (true) | TTS starts audio | Set body to `'talk'`, keep emotion |
| `isSpeaking` (false) | TTS finishes all audio | Set body to `'idle'`, apply final emotion from `streaming.currentEmotion` |
| `llm-animation` handler | Structured JSON from Rust | `setAvatarState(emotion)` — only changes face, body stays `'talk'` |
