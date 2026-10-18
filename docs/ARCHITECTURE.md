# TeachMyPC Architecture

TeachMyPC is a local-first macOS desktop app. Angular presents the rule workflow, Tauri exposes desktop capabilities, and Rust owns persistence and filesystem access.

## Components

- **Angular frontend (`src/app`)** — standalone pages for Workbench, rule creation, Activity, and Settings. Signal-based services hold frontend state and call typed Tauri commands.
- **Tauri command layer (`src-tauri/src/lib.rs`)** — connects frontend requests to the Rust interpreter, rule repository, file operation engine, and native dialog plugin.
- **Rule interpreter (`src-tauri/src/interpreter.rs`)** — sends instructions to the fixed local Ollama endpoint (`http://127.0.0.1:11434`) using `gemma3:4b`. It validates structured model output and returns supported, clarification, or unsupported results.
- **Rule repository (`src-tauri/src/rules.rs`)** — persists rules, authorized folder identifiers, and bounded Activity history in the app's local data store. Rules are saved disabled.
- **Execution engine (`src-tauri/src/engine.rs`)** — polls authorized source folders for new regular files, checks rule matches, plans supported actions, and records execution outcomes.

## Rule and file flow

1. The user submits an instruction. The frontend calls the interpreter command; it receives a structured proposal from local Ollama.
2. The user reviews or edits the proposal and chooses source and destination folders through the native picker. AI-provided folder names do not grant filesystem access.
3. Rust validates the rule and stores it disabled. Enabling is a separate, confirmed operation that revalidates the rule and folders.
4. For an enabled rule, the Rust engine polls for new matching files. Its deterministic planner checks authorized roots, resolves the ordered rename, move, or copy actions, and rejects collisions before execution.
5. Execution results are persisted and shown in Activity. Dry Run uses the planner for a selected sample file without executing operations or recording activity.

The model only interprets language into rule data. It does not choose authorized paths, execute commands, or perform filesystem operations.

## Important boundaries

- Filesystem access stays in Rust; the frontend uses identifiers for folders selected and authorized by the user.
- Only file-created triggers and rename, move, and copy actions are supported.
- The watcher polls every 500 ms and does not scan recursively. See the [README limitations](../README.md#current-limitations) and [demo guide](DEMO_GUIDE.md) for operational caveats.
