# Rule — Local-First Brain Configuration

> **Status:** Enforced · **Added:** 2026-04-29 · **Scope:** First-launch
> wizard, brain auto-configuration, `autoConfigureForDesktop()`, coding
> LLM recommendations.

## Principle

**TerranSoul must prefer a local LLM over any cloud provider** when
configuring the brain on first launch. "Your AI lives on your machine"
is the core product promise — the first-launch experience must reflect
it.

Cloud providers (Pollinations, Groq, etc.) are the **fallback**, not the
default.

## Decision Cascade

The first-launch wizard executes this cascade **in order**:

```
1. Detect Ollama → running?
   ├─ YES → Are models installed?
   │   ├─ YES → Pick the best installed model per §26 top-picks
   │   │        → setBrainMode({ mode: 'local_ollama', model })
   │   └─ NO  → Pick the §26 top-pick for the user's RAM tier
   │            → Pull it (ollama pull <model>)
   │            → setBrainMode({ mode: 'local_ollama', model })
   └─ NO  → Fallback to free cloud API (Pollinations)
            → setBrainMode({ mode: 'free_api', provider_id: 'pollinations' })
```

## Source of Truth

The model recommendation comes from **`brain-advanced-design.md` §26**
("Recommended Local LLM Catalogue"). The Rust model recommender
(`brain::model_recommender::recommend()`) reads the catalogue and
selects the top pick per RAM tier:

| RAM Tier | Top Pick |
|---|---|
| ≥ 32 GB | `gemma4:31b` |
| 16–32 GB | `gemma4:e4b` |
| 8–16 GB | `gemma4:e2b` |
| 4–8 GB | `gemma3:1b` |
| < 4 GB | `tinyllama` |

The same catalogue governs both the **brain** (chat LLM) and the
**coding LLM** (self-improve workflow).

## Rules

1. **First launch must attempt local Ollama first.** The wizard calls
   `brain.checkOllamaStatus()` + `brain.fetchRecommendations()` +
   `brain.fetchInstalledModels()` **before** deciding the brain mode.

2. **If Ollama is running and has models, use the best installed model.**
   "Best" = the model whose `model_tag` matches the §26 top-pick for
   the user's RAM tier, or — if that model is not installed — the
   largest installed model that fits in RAM.

3. **If Ollama is running but has no models, pull the §26 top-pick.**
   The wizard calls `brain.pullModel(topRecommendation.model_tag)` with
   a visible progress indicator, then activates it.

4. **If Ollama is unreachable, fall back to Pollinations.** This is the
   *only* path that uses a cloud provider. The summary screen must say
   "Brain connected (Pollinations AI — free cloud fallback)" so the
   user knows they are not running locally.

5. **The All Set screen must reflect the actual brain mode.** If local:
   "🧠 Brain connected (Local — `<model>`)". If cloud fallback:
   "🧠 Brain connected (Pollinations AI — free cloud fallback)".

6. **The coding LLM recommendations must use the same §26 catalogue.**
   `coding_llm_recommendations(total_ram_mb)` delegates to
   `brain::model_recommender::recommend()`. No hardcoded model names
   outside the catalogue.

7. **User override.** The Brain panel in Settings always lets the user
   switch to any mode (free cloud, paid cloud, local Ollama, LM Studio).
   This rule governs only the **automatic first-launch default**.

## Configuration

The local-first policy is stored in `SettingsStore` as
`prefer_local_brain: boolean` (default `true`). Users can toggle this
in **Settings → Brain → Preferred first-launch mode** to choose cloud
as their default for future resets / reinstalls.

## Enforcement

- Any PR that changes the first-launch wizard, `autoConfigureForDesktop`,
  or `brain.initialise()` must verify that the local-first cascade is
  preserved.
- Tests must cover: Ollama running + models → local; Ollama running + no
  models → pull + local; Ollama unreachable → cloud fallback.
- The `FirstLaunchWizard` summary items must reflect the actual mode, not
  a hardcoded string.
