# TeachMyPC v0.1.0 — Preview

First public preview of TeachMyPC, a local-first macOS desktop app for reviewing and running simple file organization rules.

## Highlights

- Describe file tasks in natural language and get a structured proposal from Gemma 3 4B through local Ollama.
- Review and edit proposed triggers and actions, select authorized folders, and save rules disabled by default.
- Preview planned file operations with Dry Run before enabling a rule.
- Watch selected folders for new files and apply supported rename, move, and copy actions.
- Review automation outcomes in Activity.

## Download and platform support

The v0.1.0 preview package is a macOS DMG for Apple Silicon (`arm64`). Intel Macs, Windows, and Linux are not supported by this release. The build was produced on macOS 26.5; older macOS versions have not been verified.

Install by opening the DMG and dragging TeachMyPC into Applications. This app has no Apple Developer ID signature and is not notarized, so Gatekeeper may block the first launch. See the README's installation steps for opening an unsigned preview app.

## Requirements

- Ollama installed and running locally.
- The `gemma3:4b` model installed (`ollama pull gemma3:4b`).

The app checks `http://127.0.0.1:11434`; Ollama and the model are not included in the DMG.

## Known limitations

- Only newly created regular files are watched. Watching is non-recursive and may miss files that appear and disappear between scans.
- Supported actions are rename, move, and copy. Cross-volume moves are rejected.
- Multi-action rules do not roll back completed operations if a later action fails.
- The Ollama endpoint and model are fixed. Rule interpretation may need clearer wording or manual edits.
- The release has no Apple Developer ID signature and is not notarized. Some managed Macs may prevent running it.
