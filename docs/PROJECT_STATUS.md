# Phase 2 status

## Completed

- Angular standalone app with strict TypeScript, routing, SCSS, and a responsive dark shell.
- Dashboard and navigation for Dashboard, My Rules, Activity, and Settings.
- Tauri 2 configuration with typed app-info and rule-interpretation commands.
- Rule creation page with English/Arabic examples, loading/error/unsupported/clarification states, and a structured preview.
- Typed Angular/Tauri contract for local interpretation. Ollama is called only from Rust; proposals are validated before returning to the UI.
- Rust Ollama adapter uses non-streaming /api/chat, JSON Schema output, a 90-second timeout, bounded response reading, and gemma3:4b.
- Versioned few-shot interpreter prompt. Validation compares clear extensions and requested action types against the returned rule, validates date placeholders, and rejects patterns that duplicate preserved extensions.
- Validation can make one corrective model retry. Diagnostics log only a safe category, field path, and fixed failure reason.
- No file operations, persistence, watchers, activation, or shell execution are implemented.

## Reliability investigation

- The old schema accepted an empty extension list for every request. The UI correctly displayed that contract as “All regular files,” so a missing model filter looked like an intentional all-files rule.
- The old backend validated supported enum values and placeholders but never compared the proposal with the instruction. A valid move plus an unrequested copy therefore passed.
- Rename validation accepted a literal extension suffix even though the application preserves the original extension. The suffix could produce a duplicate extension when execution is added.
- The Arabic failure was returned as a generic safety-validation error, with no field or reason recorded. The original rejected model response was not captured, so its precise failing field cannot be recovered. The new field-aware diagnostics and Arabic regression test cover that gap.
- Preview mapping was consistent with the contract: an empty extension list means all regular files; move/copy labels already signal that folder selection is required. Backend consistency checks now prevent an omitted explicit filter from reaching that mapping.

## Verification

- npm run build — passed (Angular emitted a component-style budget warning for the rule creation screen).
- npx tsc --noEmit -p tsconfig.app.json — passed.
- npm test -- --watch=false — passed, 2 frontend tests.
- cargo fmt --manifest-path src-tauri/Cargo.toml — passed.
- cargo check --manifest-path src-tauri/Cargo.toml — passed.
- cargo test --manifest-path src-tauri/Cargo.toml — passed, 17 deterministic tests; one live-model test is ignored by default.
- cargo test --manifest-path src-tauri/Cargo.toml live_local_model_interpretation -- --ignored --nocapture — passed against installed local gemma3:4b for all three reproduced instructions.
- Tauri desktop startup was not separately verified.

## Setup and manual interpretation check

- Install Ollama from https://ollama.com/download, then run: ollama pull gemma3:4b. The app never downloads models automatically.
- Start Ollama, then run npm run tauri:dev from the project root.
- Open My Rules, enter an instruction such as “Move new PDFs to my Invoices folder,” and choose Generate preview. Verify the destination requires folder selection and the preview states that no files changed.
- Run deterministic backend tests with cargo test --manifest-path src-tauri/Cargo.toml; tests use a mocked Ollama adapter and do not need a running model.

## Known limitations

- Proposals are informational only. Phase 3 must add user-authorized folder selection before move/copy execution.
- Interpretation quality depends on the local model. Unsupported and ambiguous results are not executable rules.
- Activity, persistence, watchers, activation, and all file operations remain out of scope.
