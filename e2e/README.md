# E2E Tests

This directory contains end-to-end tests using Playwright.

## CI vs. Local Test Distribution

**GitHub Actions CI** runs only:
- `desktop-flow.spec.ts` — sanity check for the desktop app

**Local / Staging Environments** run all tests:
- `desktop-flow.spec.ts` — desktop app flow
- `brain-local-lm.spec.ts` — local Ollama integration
- `memory-flow.spec.ts` — memory/RAG features
- `mcp-mode.spec.ts` — MCP server integration
- `mobile-flow.spec.ts` — mobile app flow
- `animation-flow.spec.ts` — animation system

This separation keeps CI fast (filtered via `playwright.config.ts` grep pattern when `CI=true`) while maintaining comprehensive coverage in local development and dedicated staging/E2E environments.

## Running Tests Locally

Run all tests:
```bash
npm run test:e2e
```

Run specific test file:
```bash
npx playwright test e2e/brain-local-lm.spec.ts
```

Run tests matching a pattern:
```bash
npx playwright test --grep "desktop-flow"
```

## Real E2E (External APIs)

For comprehensive real-API testing:
```bash
npm run test:e2e:real
```

This uses the configuration in `Real-E2E/playwright.config.ts` and runs against production-like external LLM APIs.
