# TeachMyPC

TeachMyPC is a macOS-first desktop app for creating local file automation rules. It interprets natural-language instructions with local Gemma 3 through Ollama, then lets you review, save, and explicitly enable rules. The Rust backend monitors authorized folders and supports rename, move, and copy actions. Review the limitations in [docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md) before enabling rules.

## Prerequisites

- macOS with Xcode Command Line Tools
- Node.js 24 LTS (24.15 or newer) and npm
- Rust stable with the macOS target installed
- Ollama with the gemma3:4b model

## Install and run

```sh
npm ci
npm run tauri:dev
```

For browser-only Angular development, run `npm start`. To create a production desktop bundle, run `npm run tauri:build`.

## Checks

```sh
npm run build
npm run test -- --watch=false
cargo check --manifest-path src-tauri/Cargo.toml
```

For the demo flow, disposable-folder setup, expected results, and troubleshooting, see [docs/DEMO_GUIDE.md](docs/DEMO_GUIDE.md).

Before starting the app, install Ollama and pull the model with `ollama pull gemma3:4b`. In My Rules, generate a proposal, select authorized source and destination folders with the native picker, review the actions, then save. Saved rules start disabled. Use Enable to confirm and activate a rule; Disable stops it. Dry run asks for a sample file path and previews the plan without changing files. Visit Activity to view recent persisted successes, skips, and failures. Malformed storage is preserved and reported instead of replaced.

The dashboard uses persisted rule and activity data; Settings shows the local Ollama health status. Rust unit tests use a mocked Ollama client. See [docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md) for verification and known limitations.
