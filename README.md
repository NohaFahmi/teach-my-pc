# TeachMyPC

TeachMyPC is a macOS-first desktop foundation for creating safe, local file automations. Phase 1 provides the Angular shell and Tauri 2 bridge; automation features are not implemented yet.

## Prerequisites

- macOS with Xcode Command Line Tools
- Node.js 24 LTS (24.15 or newer) and npm
- Rust stable with the macOS target installed

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

See [docs/PROJECT_STATUS.md](docs/PROJECT_STATUS.md) for the current Phase 1 verification record.
