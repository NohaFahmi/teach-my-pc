# TeachMyPC

TeachMyPC is a macOS-first desktop app for creating safe, local file automation proposals. Phase 2 interprets natural-language instructions with local Gemma 3 through Ollama and shows a reviewable preview. It does not execute rules or change files.

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

Before starting the app, install Ollama and pull the model with ollama pull gemma3:4b.

Rust unit tests use a mocked Ollama client. See [docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md) for the opt-in real-model test, verification, and known limitations.
