# TeachMyPC Final Regression and Native QA Report

Date: 2026-10-18

## Automated regression

| Test case | Expected result | Actual result | Status | Evidence |
|---|---|---|---|---|
| Rule activation and deactivation synchronization | Selected rule inspector, rule list, runtime status, and primary action agree after each transition. | Focused Playwright scenario passed through disabled → enabled → disabled. | Pass | `qa/walkthrough/rule-activation.spec.ts`; 1 test passed (2.1s). Browser IPC is a deterministic fixture. |
| Repeated activation | Rapid repeated interaction converges to enabled without inconsistent UI state. | Double-click activation converged to Running / Enabled with Disable rule available. | Pass | Same focused Playwright result. Fixture-only; not a native IPC check. |
| Failed activation | Error is shown and rule remains disabled with Enable rule available. | Fixture activation rejection surfaced the error and preserved Disabled state. | Pass | Same focused Playwright result. Fixture-only; not a native IPC check. |
| Existing browser walkthrough | Existing routes and UI scenarios continue to work. | Walkthrough passed. This run included the full existing visual walkthrough because activation coverage was embedded in its single test; a separate focused regression spec has since been added. | Pass | `qa/walkthrough/walkthrough.spec.ts`; 1 test passed (23.4s). Browser-only IPC fixtures. |
| Angular unit tests | Existing unit suite passes. | 21 tests across 5 files passed. | Pass | `npm test -- --watch=false`. |
| TypeScript checks | App and spec TypeScript projects type-check. | Both `tsc --noEmit` commands exited 0. | Pass | `npx tsc --noEmit -p tsconfig.app.json`; `npx tsc --noEmit -p tsconfig.spec.json`. |
| Production Angular build | Production frontend builds. | Build succeeded. `rule-creation.page.scss` exceeded its 4 kB warning budget by 2.10 kB (6.10 kB total); initial JS/CSS total was 356.43 kB. | Pass with warning | `npm run build`. Node 25.2.1 also emitted its odd-numbered-version advisory. |
| Rust backend unit tests | Native filesystem, rule, persistence, watcher, collision, and interpretation tests pass. | 50 passed, 0 failed, 1 ignored. Ignored test requires live Ollama/Gemma. One dead-code warning for `engine::execute`. | Pass with skip | `cargo test --manifest-path src-tauri/Cargo.toml`. |
| Tauri packaged build | QA `.app` bundle builds for native GUI testing. | Failed during Angular `beforeBuildCommand`: esbuild reported `all goroutines are asleep - deadlock`; no `.app` was produced. Normal `npm run build` succeeds separately. | Fail | `npm run tauri:build -- --config /tmp/teachmypc-qa-tauri.json`, attempted twice. |

## Isolated native test environment

| Test case | Expected result | Actual result | Status | Evidence |
|---|---|---|---|---|
| QA folder creation | `~/Desktop/TeachMyPC-QA/` contains Incoming, Archive, Copies, and CollisionTest. | All four directories were created. | Pass | Filesystem inspection/creation command completed with exit code 0. |
| Disposable fixtures | Only disposable `.txt` and `.pdf` fixtures are used for smoke testing. | `Incoming/sample.txt`, `Incoming/sample.pdf`, `CollisionTest/sample.txt`, and `CollisionTest/sample.pdf` were created. The PDFs are minimal valid disposable PDFs; all four files stay inside the QA directory. | Pass | Fixture creation and PDF generation commands. |

## Native macOS smoke tests

The native smoke sequence could not be completed because no interactable TeachMyPC window became available. The first `tauri dev` attempt could not bind port 4201, which was already occupied. A second attempt served Angular on 4202 and compiled/ran `target/debug/teachmypc`, but the app did not appear in the macOS app/window inventory exposed for GUI interaction. A packaged `.app` attempt then failed in esbuild as listed above. No browser mock is counted as native evidence.

| Test case | Expected result | Actual result | Status | Evidence |
|---|---|---|---|---|
| Save a new rule disabled by default | New rule persists with enabled=false. | Requires visible app interaction; not run. | Blocked | No accessible native app window. Backend unit tests cover save-always-disabled behavior, but that is not a GUI smoke test. |
| Enable rule and start watcher | Rule becomes enabled and watcher reports Running. | Not run in native UI. | Blocked | Focused Playwright used mocked IPC; not evidence of a real watcher. |
| Create file and verify rename | New authorized-folder file is renamed as configured. | Not run against the QA folders. | Blocked | Rust engine tests cover rename behavior with temporary directories. |
| Verify move | File moves to the selected destination. | Not run against the QA folders. | Blocked | Rust engine tests cover ordered rename/move behavior with temporary directories. |
| Verify copy preserves source | Destination copy appears and source remains. | Not run against the QA folders. | Blocked | Rust engine tests cover copy behavior with temporary directories. |
| Dry Run makes no changes | Preview reports actions without modifying files or adding activity. | Not run in native UI. | Blocked | Rust dry-run unit tests pass; browser walkthrough uses fixtures only. |
| Filename collision handling | Existing destination is preserved; operation is reported as skipped/error per documented behavior. | Not run against the QA folders. | Blocked | Rust collision tests pass, including preservation of existing destination and skipped activity. |
| Activity success/skipped/failed reporting | Activity records match each real operation outcome. | Not run in native UI. | Blocked | Rust persistence/recording unit tests pass; no real activity history was generated in the QA directory. |
| Restart persistence | Rules and activity remain after app restart. | Not run in native UI. | Blocked | Rust repository restart tests pass; no native app restart was performed. |
| Unauthorized folder protection | Rule cannot operate outside explicitly authorized folders. | Not run in native UI. | Blocked | Rust path-validation tests pass; native folder authorization was not exercised. |
| Watcher after restart and status/error reporting | Enabled rules resume monitoring and report errors accurately. | Not run in native UI. | Blocked | Rust watcher/restart tests pass; no live watcher status was observed through Tauri IPC. |

## Remaining bugs and submission blockers

- No application behavior bug was confirmed in this verification.
- Native macOS acceptance remains unknown because the GUI smoke tests were not executed.
- Tauri packaging is currently blocked by an esbuild deadlock in the configured `beforeBuildCommand`, despite the standalone production Angular build succeeding.
- The installed `gemma3:4b` Ollama model is present, but no rule was interpreted or created in the native app.
- The valid `.pdf` fixtures are prepared, but PDF-specific app handling has not been exercised.

## Manual native smoke procedure

After resolving the Tauri packaging blocker and opening the isolated QA app, use only `~/Desktop/TeachMyPC-QA/`:

1. In the folder picker, authorize `Incoming` and the destinations needed for each rule (`Archive`, `Copies`, or `CollisionTest`). Create a rename rule for new `.txt` files, save it, and verify it appears Disabled. Repeat with a move rule from Incoming to Archive and a copy rule from Incoming to Copies.
2. Enable each rule. Confirm its list row says Running, the inspector says Enabled/Running, and the primary action changes to Disable rule. Disable and re-enable once to check status synchronization.
3. With the rename rule active, create a new disposable file such as `rename-check.txt` in Incoming. Confirm the dated rename appears there and a success record names the resulting path.
4. With the move rule active, create `move-check.txt` in Incoming. Confirm it leaves Incoming and appears in Archive. With the copy rule active, create `copy-check.txt`; confirm the copy appears in Copies and the source remains in Incoming.
5. Select each rule and run Dry Run against an appropriate disposable fixture. Compare folder contents before and after; they must be unchanged and Dry Run should not create an Activity record.
6. For collision behavior, configure a copy to CollisionTest and use the existing `sample.txt` destination. Create a new Incoming file with that same name. Confirm the existing CollisionTest file is unchanged, the source remains, and Activity reports the documented skipped/collision result.
7. Confirm Activity shows the real success and skipped cases. To exercise failed status, use only a disposable destination under the QA folder and make it unavailable after configuring the rule, then confirm the operation records an actionable failure. If that failure cannot be induced without changing permissions or risking unrelated paths, leave it Not tested.
8. Restart the isolated QA app. Confirm saved rules and Activity remain, enabled rules resume monitoring, and runtime status/error details are visible. Add a fresh test file to verify the restarted watcher processes it once.
9. Create a separate disposable source folder outside `~/Desktop/TeachMyPC-QA/`, place one disposable `.txt` file there, and verify the rule builder cannot select/access it until it is explicitly authorized. Do not point any rule at personal or unrelated folders.

Record screenshots or exact filenames/Activity statuses in this report after completing the GUI run. Do not mark any blocked native case as passed based on the browser fixture or Rust unit tests.

## Next highest-priority action

Resolve the Tauri packaging/esbuild deadlock, launch the isolated QA `.app`, then complete the manual native cases above against `~/Desktop/TeachMyPC-QA/`. Do not treat the passing browser fixture or Rust unit suite as a substitute for that acceptance run.
