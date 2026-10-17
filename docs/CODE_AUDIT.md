# TeachMyPC Code Audit

**Audit date:** 2026-10-17  
**Scope:** Follow-up safety hotfix review, 2026-10-17. The two High findings below were addressed in Rust. Manual packaged-app acceptance testing was not performed.

## A. Executive summary

TeachMyPC has a clear, small vertical slice: standalone Angular pages call typed Rust Tauri commands; Rust validates rules and folder identifiers, builds deterministic plans, executes approved operations, and stores recent activity locally. The safety model is stronger than a typical prototype: saved rules are forced disabled, folder access originates in a native picker, sample paths are checked against an authorized root, plans reject collisions, and the interpreter output is schema and semantics validated. Rust engine and repository coverage is substantial for this scope.

The two High-severity findings are fixed. Multi-step rules can still leave partial file changes; these are now recorded explicitly and are never blindly retried. Other previously documented frontend and filesystem risks remain outside this hotfix.

**Overall:** Both High-severity Rust safety findings are fixed. macOS rename/move uses the OS exclusive-rename primitive (`renameatx_np` with `RENAME_EXCL`). This guarantee is limited to that primitive and does not imply general race freedom for all filesystem operations.

## B. Current architecture map

```text
Angular standalone app (src/app)
  app/routes + dashboard, rules, activity, settings pages
    RuleCreationPage -> RuleInterpreterService -> invoke("interpret_instruction")
    Pages -> RuleStoreService -> invoke(list/save/delete/enable/preview/activity/status)
    SettingsPage -> AppInfoService -> invoke(app and Ollama status)
                         |
                         v
Tauri 2 command layer (src-tauri/src/lib.rs)
  dialog picker -> RuleRepository (rules-v1.json, app data directory)
  interpreter -> blocking local HTTP request to Ollama at 127.0.0.1:11434
  engine watcher thread -> polls authorized source folders every 500 ms
    -> deterministic plan -> rename/move/copy -> bounded activity history
```

The frontend is organized by feature, with a small `core/models` and `core/services`, plus one shared component. State is held in injectable signal services and local page signals; no global state library or RxJS flows are used. Rust has three domain modules (`rules`, `engine`, `interpreter`) and a Tauri command/bootstrap module. There is no demonstrated need for a different folder structure.

## Baseline

- `package.json` declares Angular `^22.2.0`, Angular build/CLI `^22.2.3`, TypeScript `~6.0.2`, Tauri JS API `^2.12.4`, RxJS `~7.8.0`, Vitest `^5.0.0`; the lockfile resolves Angular core/compiler/compiler-cli/CLI to **22.2.3**, TypeScript to **6.0.3**, Tauri API to **2.12.4**, RxJS to **7.8.2**, and Vitest to **5.0.5**.
- Rust uses edition 2024, minimum Rust 1.90, Tauri 2.12.4, serde/serde_json, reqwest blocking + rustls, chrono, uuid, and serde_path_to_error (`src-tauri/Cargo.toml:1-29`). Both npm and Cargo lockfiles are present.
- Angular uses the application builder and Vitest unit-test builder (`angular.json:16-61`). TypeScript enables several useful diagnostics but **does not enable `strict` or Angular `strictTemplates`** (`tsconfig.json:4-21`; app/spec configs at `tsconfig.app.json:1-9` and `tsconfig.spec.json:1-8`).
- Tauri builds the Angular browser output, binds its dev server to localhost, bundles macOS app/DMG targets, and sets `app.security.csp` to `null` (`src-tauri/tauri.conf.json:5-37`). The default capability grants `core:default` to the main window (`src-tauri/capabilities/default.json:1-11`).
- The app has 5 Angular spec files. The Rust suite contains repository, interpreter, status, and engine tests, including watcher tests. No lint script or lint dependency is declared (`package.json:5-28`).

## C. Findings and recommended correction

### Must fix before release

#### High — Rename/move can overwrite a file created after collision planning

`plan` retains its collision check for previews. Execution now uses `renameatx_np(RENAME_EXCL)` on macOS, which atomically fails if the destination exists, including if it appears after planning. Linux builds use `renameat2(RENAME_NOREPLACE)` for local checks. The same-device check remains; no copy-and-delete fallback is used. Collision results are persisted as `skipped`. Tests cover occupied and late-created destinations plus normal rename/move. This is the guarantee of the exclusive-rename primitive; it is not a claim of race freedom for path validation, folder replacement, or every network/filesystem implementation.

**Correction:** Implemented: macOS uses `renameatx_np(RENAME_EXCL)` and reports `EEXIST` as a safe skipped outcome. Regression tests run on macOS and cover existing and late-created destinations.

#### High — Watcher discards execution outcomes and marks failures processed

The watcher now consumes explicit dispositions. Success and terminal skip are processed once. Retryable failures are retried at most twice after the initial attempt only when no action completed; partial failures are never retried. Partial and terminal failures are recorded in Activity and exposed as per-rule runtime `Error` status. Activity storage failures are also surfaced and treated as terminal to avoid duplicate execution after possible file changes.

**Correction:** Implemented as described above. Retry classification is limited to known transient OS errors, unavailable source, or inaccessible source/destination. Other failures are terminal for that observed file identity to prevent blind duplicate actions.

### Safe incremental improvement

#### Medium — Multi-action failure can leave partial changes

Actions execute sequentially and stop on the first error (`src-tauri/src/engine.rs:239-285`). Earlier renames, moves, or copies are not rolled back. Activity records completed actions and a resulting path (`engine.rs:288-319`), which is useful evidence, but a failed rule can still have changed the file location or contents. A rename followed by a failed move is one concrete case.

**Correction:** Activity records `partial_failure`, completed action labels, resulting path, and the error. The watcher does not retry it. Reliable rollback is not implemented; an I/O error after earlier actions can still leave partial changes.

#### Medium — JSON store mutations are not committed transactionally in memory

Repository methods mutate the locked `StoreFile` and then call `write` (`src-tauri/src/rules.rs:159-183`, `185-217`, `240-252`). If serialization or temporary-file write/rename fails, the in-memory data has already changed although the command returns an error. Subsequent reads in this process may show a mutation that was not persisted; for `save`, the caller may retry after a reported failure. The temp-and-rename approach protects the old file from most interrupted writes, but does not keep memory and disk consistent on returned errors.

**Correction:** Stage changes in a cloned candidate store, write that candidate, and publish it to the mutex only after the write succeeds. Add injected write-failure tests if the repository is made injectable; do not compromise the current malformed-file preservation behavior.

#### Medium — Reloaded folder authorization follows a replaced path

Folders are canonicalized when selected (`src-tauri/src/rules.rs:110-129`) and stored as path strings. Every use canonicalizes the stored path again (`rules.rs:219-235`). If the original directory is moved away and another directory or symlink is installed at that path, the app follows the new target under the existing folder ID. Symlink samples themselves are rejected, but this does not pin the selected directory identity.

**Correction:** Decide and document whether authorization follows a path or a directory identity. If the intended grant is to the selected directory, detect replacement and require the user to select the new target. Add tests for a replaced/symlinked authorized root. Assess macOS sandbox/security-scoped bookmark needs against the actual distribution model before adding complexity.

#### Medium — Activity polling can overlap and stale responses can win

`ActivityPagePage` starts a 1.5-second interval (`src/app/features/activity/activity-page.page.ts:15-20`), while each call performs an unsequenced IPC request and sets shared loading/data signals (`src/app/core/services/rule-store.service.ts:22`). If a request lasts longer than the interval, requests overlap and an older response can overwrite a newer result; `activityLoading` can also become false while another request is pending. The global `aria-live="polite"` wraps the page outlet (`src/app/app.html:37-39`), so frequent activity changes can cause broad, repeated announcements.

**Correction:** Prefer an event from the Rust engine when activity changes, or use a serialized refresh with a materially slower interval. Guard response ordering and loading state. Scope live regions to concise status messages rather than the entire routed page.

#### Medium — Dry-run failure can display a stale plan beside a new error

`dryRun` clears `dryRunError` but does not clear `dryRunResult` before opening the picker or requesting another preview (`src/app/features/rules/rule-creation.page.ts:77-89`). If a prior preview succeeded and the next selection or preview fails, the panel can show the previous sample's plan alongside the new failure. The existing tests cover a successful preview and an initial picker failure, but not this sequence (`rule-creation.page.spec.ts:125-145`).

**Correction:** Clear the prior result when starting a new preview, or model dry-run state as a discriminated union keyed by the selected rule/sample. Add the success-then-failure regression test.

#### Medium — Compiler strictness leaves avoidable gaps at the IPC/UI boundary

`tsconfig.json` enables `strictInjectionParameters` but omits TypeScript `strict` and Angular `strictTemplates` (`tsconfig.json:4-21`). The Tauri calls use generic return annotations such as `invoke<SavedRule[]>` (`src/app/core/services/rule-store.service.ts:16-25`), which provide compile-time shape assumptions but do not validate runtime payloads. Angular's template compiler therefore has fewer checks on bindings such as the action selector's `$event` (`rule-creation.page.html:25`).

**Correction:** Enable TypeScript strict mode and Angular strict template checking in a dedicated, incremental change; resolve concrete diagnostics. Keep Rust serde as the authoritative runtime boundary and avoid adding runtime schema machinery without evidence of mismatched payloads.

#### Low — One shared component is unused; rules page exceeds style warning budget

`src/app/shared/components/empty-feature-page.ts:1-10` and its HTML/SCSS have no references elsewhere in `src`. The production build succeeds, but warns that `rule-creation.page.scss` is 7.65 kB against a 4 kB warning budget (configured in `angular.json:29-38`).

**Correction:** Remove the unused component in routine cleanup if it remains unused. Split or trim rules-page styles only if that improves ownership/readability; do not raise the budget just to suppress the warning.

#### Low — Accessibility labeling is incomplete

The action-type `<select>` has no associated label (`src/app/features/rules/rule-creation.page.html:25`), while most other controls have labels or explicit accessible names. The page has a global live region as noted above. Existing templates include useful landmarks, headings, button names, status roles, and hidden decorative icons.

**Correction:** Give each action selector an accessible name tied to its action number and run an accessibility audit against keyboard and screen-reader flows. Add `datetime` to activity `<time>` elements if those remain semantic time elements (`activity-page.page.html:19-20`).

## D. Modern Angular assessment

- **Standalone:** Already used across routed pages; Angular 22 defaults components to standalone, so the absence of explicit `standalone: true` is not a defect. Keep the feature page boundaries.
- **Signals/computed:** Services and rule builder already use signals and a few derived values use `computed` (`rule-creation.page.ts:13-30`). Dashboard counts and `latestActivities()` are recomputed through template method calls (`dashboard-page.page.ts:21-27`); convert only if profiling or repeated derivation becomes material. Avoid effects for values that can remain computed.
- **`input()` / `output()` / `model()` and `linkedSignal`:** The unused empty page already uses required signal inputs (`empty-feature-page.ts:1-10`). No current routed page needs child-component model synchronization; `linkedSignal` would not simplify the present workflow.
- **OnPush:** No page declares `ChangeDetectionStrategy.OnPush`. Signals and standalone components are in place; OnPush is a safe incremental option after strict template checks, especially if the rules page remains one large component. Measure and verify behavior rather than applying it as a blanket rewrite.
- **Control flow:** Modern `@if`, `@for`, `@switch`, and `@let` syntax is already used. Tracking is present; the activity list tracks `$index`, which is acceptable for append/replace snapshots but a stable record identifier would be preferable if records gain one.
- **`@defer`:** No expensive below-the-fold feature or heavy optional dependency currently justifies it. The initial bundle is 355.16 kB raw (91.31 kB estimated transfer), so defer only if a real bundle or interaction need appears.
- **`inject()` / lifecycle:** `inject()` is used on most pages/services. The interval in the rule page is explicitly cleared with `DestroyRef`; Activity does likewise, so there is no demonstrated timer leak. `takeUntilDestroyed` is unnecessary without RxJS subscriptions.
- **Forms:** The builder currently uses template-driven `ngModel`; there is no multi-control form model or async validation complexity that demonstrates a need for typed reactive forms. Revisit if form validation or dirty-state behavior expands.
- **Async state:** The interpreter uses a useful discriminated union. The store keeps independent loading/error signals and has the stale dry-run case above. Keep IPC calls imperative. Angular 22 supports resource APIs, but `resource`/`rxResource` would add little for explicitly user-triggered Tauri commands and could obscure command side effects; no migration is recommended.

## E. Tauri and Rust safety findings

### Strengths observed

- Rust owns filesystem access; frontend sends folder IDs for rule actions, not AI-generated paths (`src-tauri/src/lib.rs:95-117`; saved `Action` uses `destination_folder_id` in `rules.rs:29-40`).
- Folder selection canonicalizes and checks directories; rules validate action counts, extensions, placeholders, authorized IDs, and source/destination containment (`rules.rs:110-129`, `300-360`).
- Dry-run samples must be regular files below the selected source root; symlink samples are rejected (`lib.rs:147-190`, `engine.rs:167-186`).
- AI output has unknown-field rejection, bounded response reads, one corrective retry, field-safe validation errors, semantic checks, and fixed local Ollama endpoint (`interpreter.rs:9-83`, `163-225`, `243-267`, `542-640`). No shell execution path was found.
- Activity is bounded to 500 persisted records and returns the latest 100 (`rules.rs:240-259`). Store writes use a same-directory temporary file and rename, and malformed/unknown-version data is preserved by opening storage as unavailable (`rules.rs:72-109`, `261-276`).

### Additional reviewed areas

- **IPC contracts:** Rust command inputs/outputs are explicit structs/enums or primitive fields. Frontend `invoke<T>` is statically typed but unchecked at runtime. No current contract mismatch was verified.
- **Path handling:** Sample path validation and source/destination containment are present. Rename validation rejects separators, control characters, dot names, and preserves extensions (`engine.rs:67-89`). Fix the rename replacement race and assess authorized-root replacement as above.
- **Copy behavior:** Copy writes a random temporary sibling, checks again, then uses `hard_link` to avoid replacing an existing destination (`engine.rs:330-339`). If temporary cleanup fails after hard-link creation, the command reports failure while the destination copy exists; activity captures the failed outcome. Add a fault-focused test if this path is hardened.
- **Watcher lifecycle/concurrency:** A detached thread polls every 500 ms forever (`engine.rs:485-499`). There is no shutdown signal or join handle, appropriate only while the process lifetime is the watcher lifetime. Rules saved through the repository are forced disabled, so edits stop their current watcher state until explicitly enabled (`rules.rs:159-183`). The frontend status refresh interval is 1 second (`rule-creation.page.ts:31-35`), polling twice as fast as backend scanning; a native event or longer refresh interval would be simpler.
- **Watcher edge cases:** The watcher scans only direct children (`engine.rs:394-400`), seeds existing matching files so they are ignored, requires size stability for two further polls, and uses `(device, inode)` in-memory identity tracking (`engine.rs:420-479`). Tests cover seed/restore and renamed-output behavior. It does not observe same-size writes after the stability window and missed/failed execution behavior is not covered.
- **Store integrity:** The mutex serializes process-local repository access. Writes are not fsynced and in-memory mutation precedes disk commit, as described above. No multi-process locking is present; current desktop architecture appears single-instance but that assumption is not enforced in this code.
- **AI isolation/logging:** Prompt and user instruction are sent only to local `127.0.0.1:11434`; validation logs contain category, field, and static reason, not instruction or raw model output (`interpreter.rs:265-267`). Activity necessarily stores local filenames and paths for the user's history; no external telemetry or cloud service is declared.
- **Tauri content security:** `csp: null` disables an explicit app CSP (`tauri.conf.json:29-34`). With custom filesystem commands exposed to the app webview, defense-in-depth is reduced if script injection or an unsafe webview content path is introduced. No current remote UI origin was found. Add a restrictive CSP compatible with the Angular bundle and local Ollama only as needed; verify Tauri IPC and dialog behavior in the packaged app.

## F. Tests and maintainability

- Frontend tests cover root rendering, dashboard summaries, activity loading/refresh, rule builder interactions, and settings states. They are component-level and use service spies; service IPC rejection handling, concurrent requests, runtime status staleness, and dry-run success-then-failure are gaps. UI tests do not exercise a real Tauri webview or native folder dialog.
- Rust hotfix verification has **51 discovered tests: 50 passed, 0 failed, 1 ignored**. Coverage includes interpreter validation/retry, malformed storage, folder/action validation, dry-run non-mutation/collisions, rename/move/copy, activity persistence, disabled rules, watcher seeding/restart, bounded retry handling, and partial-failure terminal handling. A live Ollama test is intentionally ignored unless local Ollama and `gemma3:4b` are available.
- Remaining engine cases outside this hotfix: file replacement between scans; root replacement/symlink; copy cleanup failure; and write failure consistency between memory and JSON. The watcher policy tests cover initial attempt plus two retries for injected transient outcomes and one attempt for partial outcomes; OS-specific failure injection in a packaged watcher run remains a manual check.
- `npm run build` succeeds; it emits the style-budget warning above. `npm test -- --watch=false` passes all **16 tests in 5 files**. `cargo test --locked` passes 42 tests with 1 ignored. `cargo clippy --locked --all-targets -- -D warnings` was attempted but could not run because `cargo-clippy` is not installed for the selected stable toolchain. No lint command exists in `package.json`.
- The build runner reports Node.js **25.2.1** and warns that odd-numbered Node versions are not LTS. README asks for Node 24 LTS (`README.md:5-10`), so the local verification environment differs from the documented runtime. This does not invalidate the successful build, but CI/release checks should use the documented LTS version.
- No dependency upgrade or hygiene action is justified by this audit. Dependencies are reasonably narrow for the app's current needs. Files are short except the rules page template/styles and Rust engine, which contain the central workflow and should be split only when a concrete responsibility or testing seam is needed.

## G. Prioritized roadmap

### Must fix before release

1. Revisit watcher identity behavior if filesystem replacement between scans is in scope.
2. Run packaged macOS GUI acceptance for rename/move collisions and watcher status/activity.

### Safe incremental improvement

3. Clarify completed actions and current resulting location in Activity; partial completion is now recorded, but filesystem rollback is not implemented.
4. Make repository mutation transactional across memory and JSON persistence.
5. Prevent stale activity and dry-run responses; reduce polling and narrow live announcements.
6. Turn on strict TypeScript and Angular template checks incrementally, fixing concrete diagnostics.
7. Confirm whether folder grants bind to path or directory identity and handle path replacement accordingly.
8. Add missing selector labels and run keyboard/screen-reader checks; address the single SCSS budget warning when a focused style cleanup is convenient.

### Optional future enhancement

9. Consider native filesystem events if polling cost or latency is demonstrated to matter; retain stability and duplicate protection.
10. Consider `OnPush` and more derived computed signals after strictness and measurement. Keep imperative Tauri commands out of resource APIs unless a future read-only request actually benefits from them.
11. Add a packaged-app integration check for command permissions, CSP, and native dialogs, in addition to existing Rust unit and Angular component tests.

No target folder structure is proposed: the existing feature/core/shared split is proportionate to the current product.
