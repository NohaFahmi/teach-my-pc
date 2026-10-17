# TeachMyPC Demo Guide

## Prerequisites

- macOS with Xcode Command Line Tools, Node.js 24+, npm, and Rust stable.
- Ollama running locally with `gemma3:4b` installed (`ollama pull gemma3:4b`). Settings shows the endpoint, model, and connection check.

## Start the demo

1. Start Ollama (`ollama serve` if it is not already running).
2. From the repository, run `npm ci` once, then `npm run tauri:dev`.
3. Confirm Settings reports Ollama connected.

## Safe disposable folders

Create a temporary demo directory outside any personal folders, for example `~/TeachMyPC-Demo`, with `Incoming` and `Organized` subfolders. Put only disposable sample files in `Incoming`. Select these exact folders in the native folder picker. Remove them yourself after the demonstration; the app does not delete files.

## Reliable examples

English: `Move new PDF files to the Organized folder.`

Arabic: `انقل ملفات PDF الجديدة إلى مجلد Organized.`

If the local model interprets a phrase differently, edit the proposed rule before saving. Use simple file type and action wording for a predictable demo.

## 90-second demonstration

1. **0–10s:** Show the dashboard metrics and explain that rules start disabled.
2. **10–25s:** Create a rule with the English example and generate its interpretation.
3. **25–40s:** Review the title, extension, action, and choose `Incoming` as source and `Organized` as destination.
4. **40–50s:** Save disabled, then show the rule in My Rules.
5. **50–65s:** Run Dry Run against a disposable PDF and show that it previews without changing files.
6. **65–80s:** Enable explicitly, add a new disposable PDF to `Incoming`, then show the persisted Activity record.
7. **80–90s:** Disable the rule and point out the local Ollama status in Settings.

## Expected results

The saved rule is disabled until explicitly enabled. Dry Run displays the planned move and does not modify files. Once enabled, a matching new regular file is moved to the authorized destination and the outcome appears in Activity. Dashboard counts come from saved rules, runtime statuses, and persisted activity; it shows no sample or fabricated records.

## Troubleshooting

- **Ollama unavailable:** Start Ollama and revisit Settings. Confirm `http://127.0.0.1:11434` is reachable locally.
- **Model missing:** Run `ollama pull gemma3:4b`, then retry interpretation.
- **Instruction unsupported or unclear:** Use one supported action (rename, move, or copy) and state a file extension and destination clearly. The app does not run arbitrary commands.
- **No activity:** Check that the rule is enabled, its source folder is the selected `Incoming` folder, the new file matches its extension, and it remains present while the watcher scans.
- **Collision or cross-volume move:** Choose a destination without a same-named file; moves across filesystem devices are rejected.

## Known limitations

- macOS desktop app; local Ollama endpoint and `gemma3:4b` are currently fixed.
- The file-created watcher polls rather than using native filesystem notifications and may miss files created and removed between scans.
- Supported actions are rename, move, and copy. Rules do not delete files or execute shell commands.
- Dry Run uses a native sample-file picker and previews the deterministic plan.
- The local model may need simple wording; always review the proposal and authorize folders yourself.
