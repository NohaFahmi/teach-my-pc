# TeachMyPC Native Launch Diagnosis

Date: 2026-10-18

## Environment and toolchain

| Tool | Version / finding |
|---|---|
| macOS | 26.6.2, Apple Silicon (arm64) |
| Node initially on PATH | 25.2.1; `ng version` reports this as unsupported by Angular 22.2.3 |
| Installed Node LTS | 24.15.0 via `~/.nvm/versions/node/v24.15.0/bin`; Angular 22.2.3 production build succeeds under it when run outside the sandbox |
| npm | 11.6.2 on PATH; 11.12.1 with Node 24.15.0 |
| Angular / Angular CLI / @angular/build | 22.2.3 |
| esbuild | 0.28.2 (installed transitively) |
| Tauri CLI / JS package | 2.12.3 |
| Rust / Cargo | 1.99.1 |
| Ollama | Local endpoint responds; `gemma3:4b` is available (two model records returned) |

`package.json` has no Node `engines` constraint. The normal scripts are `npm run start` (`ng serve`), `npm run build` (`ng build`), `npm run tauri:dev`, and `npm run tauri:build`.

## Configuration inspected

Checked-in `src-tauri/tauri.conf.json` sets:

- `frontendDist`: `../dist/teachmypc/browser` (production Angular output).
- `devUrl`: `http://localhost:4201`.
- `beforeDevCommand`: `npm run start -- --host 127.0.0.1 --port 4201`.
- `beforeBuildCommand`: `npm run build`.
- One 800×600 resizable `TeachMyPC` window, not fullscreen; CSP is `null`.
- Bundle targets are `app` and `dmg` by default.

The temporary `/tmp/teachmypc-qa-tauri.json` contained product name `TeachMyPC QA`, identifier `com.teachmypc.desktop.qa`, dev URL `http://localhost:4202`, and a dev command for 4202. It did not override `frontendDist` or `beforeBuildCommand`.

## Root cause and evidence

The packaging failure was not reproduced as an esbuild or Tauri hook bug once commands could run outside the restricted execution sandbox. Angular 22.2.3 declares Node 25.2.1 unsupported, but Node version alone was not the failure: Node 24.15.0 and Node 25.2.1 both exited 134 during `ng build` when run inside the sandbox, stopping after `Building…`. The same Node 24.15.0 standalone production build completed successfully outside the sandbox, and Tauri then produced the app bundle by consuming that output.

Relevant environment evidence:

- Angular `ng serve` could not bind from the sandbox (`listen EPERM` on 127.0.0.1:4202); the same server started after elevated execution.
- Tauri CLI could not reach the active frontend server from the sandbox and remained in “Waiting for your frontend dev server”. Its elevated launch proceeded to compile and run the Rust binary.
- macOS DiagnosticReports contains Node SIGABRT reports at 15:46 on 2026-10-18. The report shows `malloc` abort / “pointer being freed was not allocated” in a Node worker. This supports a native worker crash under the restricted run, but does not independently prove why the allocator aborted. The build succeeding outside the sandbox is the confirming environment comparison.
- Thus, the prior report that the problem occurred only inside `beforeBuildCommand` was an execution-context artifact: the Tauri hook and standalone build shared the same sandbox restriction. The temporary QA config was not required to bypass the problem.

Port observations: initial inspection found a Node listener on 127.0.0.1:4201. It later exited. A fresh Angular server was started on 127.0.0.1:4202 for native dev; it was reachable by the elevated Tauri launcher. Ollama listens on 127.0.0.1:11434 and returned the local tag list when queried in the elevated context.

## Commands executed and outcomes

Read-only inspection:

```text
node --version                         # v25.2.1
npm --version                          # 11.6.2
rustc --version                        # 1.99.1
cargo --version                        # 1.99.1
npx ng version                         # Angular/CLI 22.2.3; Node 25.2.1 unsupported
npx tauri --version                    # tauri-cli 2.12.3
npm ls esbuild @angular/build @angular/cli @tauri-apps/cli --depth=0
lsof -nP -iTCP -sTCP:LISTEN
```

Reproduction and resolution:

```text
PATH="$HOME/.nvm/versions/node/v24.15.0/bin:$PATH" npm run build
  # sandbox run aborted, exit 134, after “Building…”
env PATH=/Users/noha-mahmoud/.nvm/versions/node/v24.15.0/bin:/usr/bin:/bin:/usr/sbin:/sbin ./node_modules/.bin/ng build
  # sandbox run aborted, exit 134, after “Building…”
env PATH=/Users/noha-mahmoud/.nvm/versions/node/v24.15.0/bin:/usr/bin:/bin:/usr/sbin:/sbin npm run build
  # elevated run succeeded; Angular output at dist/teachmypc; initial total 356.43 kB
```

Dev/native launch:

```text
env PATH=/Users/noha-mahmoud/.nvm/versions/node/v24.15.0/bin:/usr/bin:/bin:/usr/sbin:/sbin npm run start -- --host 127.0.0.1 --port 4202
  # sandbox: EPERM binding localhost; elevated: Angular dev server ready at 127.0.0.1:4202
node -e 'require("fs").writeFileSync("/tmp/teachmypc-native-dev-override.json", JSON.stringify({build:{devUrl:"http://127.0.0.1:4202",beforeDevCommand:""}}))'
npx tauri dev --config /tmp/teachmypc-qa-tauri.json --config /tmp/teachmypc-native-dev-override.json
  # sandbox stayed waiting for server; interrupted
npx tauri dev --config /tmp/teachmypc-qa-tauri.json --config /tmp/teachmypc-native-dev-override.json
  # elevated: compiled and ran target/debug/teachmypc
```

Tauri dev completed compilation and launched the process, but TeachMyPC did not appear in the computer-use app inventory. A later attempt to bind the app through the GUI automation API returned `Computer Use permissions are not granted`. This is an access limitation of the current GUI automation session; it does not establish that the app window crashed or that the Tauri window configuration is defective.

Packaging used a temporary override only for this invocation. It leaves the checked-in production config and frontend output path intact, disables the duplicate frontend build, and requests an app bundle:

```text
node -e 'require("fs").writeFileSync("/tmp/teachmypc-package-override.json", JSON.stringify({build:{beforeBuildCommand:""},bundle:{targets:["app"]}}))'
npx tauri build --config /tmp/teachmypc-package-override.json --bundles app
  # succeeded; 14.19 MiB app bundle
```

Bundle metadata inspection confirmed identifier `com.teachmypc.desktop`, executable `teachmypc`, and minimum macOS 10.13. `open /Users/noha-mahmoud/Personal/teach-my-pc/src-tauri/target/release/bundle/macos/TeachMyPC.app` returned exit 0. Interactive window visibility could not be verified because GUI automation permissions are unavailable.

## Outcome

- **Build:** standalone production frontend succeeded under Node 24.15.0 outside the sandbox.
- **Packaged app:** succeeded at [`TeachMyPC.app`](/Users/noha-mahmoud/Personal/teach-my-pc/src-tauri/target/release/bundle/macos/TeachMyPC.app), 14.19 MiB.
- **Dev launch:** Rust compilation and `target/debug/teachmypc` launch succeeded with the QA dev URL override.
- **Native window:** app was opened from the bundle with `open`, but the window could not be observed or interacted with in this session. No crash was confirmed.
- **Ollama:** local endpoint returned `gemma3:4b`; no request was made through TeachMyPC’s real Tauri IPC.
- **Actual app functionality tested natively:** none. No rule was created, enabled, or disabled, and no filesystem operation, Activity record, or restart persistence check was performed. These remain **Blocked**, not passed.

## Files changed

Only this diagnosis document was added. No Angular, Rust, Tauri configuration, automation behavior, dependency, or application data was changed. Temporary override JSON files were written under `/tmp` and are not part of the repository.

## Remaining blockers and manual actions

GUI automation reports that Computer Use permissions are not granted. To finish native verification on this Mac:

1. Allow the Codex/GUI automation app access to the macOS Accessibility permission in System Settings, or perform these checks directly in the visible TeachMyPC window.
2. Launch the packaged app at `/Users/noha-mahmoud/Personal/teach-my-pc/src-tauri/target/release/bundle/macos/TeachMyPC.app` (it was already opened once with `open`). If it is not visible, open Finder, navigate to that path, and double-click `TeachMyPC.app`; check the Dock and app switcher before treating a missing window as a crash.
3. Use only `~/Desktop/TeachMyPC-QA/` and its disposable Incoming, Archive, Copies, and CollisionTest folders. Confirm Settings connects to local Ollama and lists `gemma3:4b`.
4. Save a disposable rule and verify it starts disabled; enable and disable it; exercise one real rename/copy/move with a newly created disposable file; confirm Activity records the result; restart and confirm persistence. Do not point rules at personal or unrelated folders.

If the packaged app opens but no window appears, capture the app’s macOS Console/crash entry and report whether its process remains running. The current evidence is insufficient to attribute the missing GUI inventory entry to the app itself.
