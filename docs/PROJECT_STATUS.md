# Phase 4 status

## Implemented

- Saved rules default to disabled; users can explicitly enable or disable them. Enabling revalidates the rule and its authorized directories. Editing continues to save a disabled rule.
- A Rust background directory poller snapshots existing files, checks non-recursively for new regular files, filters extensions case-insensitively, and waits for stable file size before processing. It ignores directories and symlinks.
- A shared deterministic planner validates the source against the authorized directory, resolves ordered rename/move/copy destinations, preserves the source extension, and rejects collisions. Execution stops at the first error. Copy uses a temporary file and no-overwrite hard-link finalization.
- Moves are rejected across filesystem devices. The frontend offers an explicit activation confirmation and a dry-run action; Dry Run opens a native picker in the rule's authorized source folder, validates the chosen regular file, and displays a planner-only preview without executing or recording activity.
- Activity is stored in the existing local JSON repository, capped at 500 records, and the latest 100 are returned by the backend.
- My Rules displays live watcher status. Activity loads persisted records, displays result details, and polls while open so new activity appears without navigation or restart.
- Watcher processing tracks Unix file identity after execution, preventing a renamed file in the watched folder from being processed repeatedly.
- Execution outcomes now record success, collision skips, and failures (including planner failures). Dry-run uses the same planner and creates no activity record.

## Verification

- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` — passed.
- `cargo check --manifest-path src-tauri/Cargo.toml` — passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` — passed, 41 tests passed, 1 ignored optional live Ollama test.
- `npm run build` — passed; rule creation stylesheet exceeds the configured 4 kB warning budget by 3.83 kB.
- `npx tsc --noEmit -p tsconfig.app.json` — passed.
- `npm run test -- --watch=false` — passed, 12 frontend tests.
- `npm run tauri:dev` — frontend and Rust app compiled and started; a visible TeachMyPC window was not available for manual click-through in this session.

## Known limitations

- The current watcher uses 500 ms polling rather than native filesystem notifications. My Rules now displays live per-rule Running/Stopped/Error status and watcher errors; explicit shutdown signaling is not yet implemented.
- Dashboard summaries now read saved rules, live watcher statuses, and persisted recent activity from existing backend commands. Settings reports the local Ollama endpoint/model and runs a bounded health check.
- Dry-run's native picker and visible preview have automated coverage; the native picker and preview still need a manual macOS GUI retest.
- Polling can miss a file created and removed between scans. Stability checks use file size only. Full race-free handling of hostile concurrent filesystem changes is not guaranteed.
- The Tauri development process compiled and started, but no visible app window was available for GUI click-through. Polling remains subject to missed short-lived files and file-size-only stability checks.
- No shell commands or deletion actions are supported. Cross-volume moves and collisions fail closed.

# Phase 5 status

- Dashboard includes real rule counts, running status, recent execution count, latest five activity records, and a Create Rule action, with loading/error/empty states.
- Rule creation presents the end-to-end steps and explains folder authorization. Activity displays the rule before the filename and retains result/error information.
- Settings reports app version, fixed Ollama endpoint/model, and local connectivity with a bounded request. See [DEMO_GUIDE.md](DEMO_GUIDE.md) for rehearsal steps.
- Dashboard and Settings behavior have focused Angular tests. GUI verification remains a manual macOS step.
- Phase 5 checks: Angular build and TypeScript check pass; Angular tests pass (14); Rust check and tests pass (41 passed, 1 optional live Ollama test ignored); Rust formatting check passes. The pre-existing rule creation stylesheet still exceeds its 4 kB component budget by 3.83 kB.

# Phase 5 final UI corrections

- Replaced the duplicate-number workflow line with a responsive stepper; only the current, state-backed step is emphasized.
- Activity now shows a concise destination summary and exposes the unchanged full destination path in an expandable details element. Path formatting affects display only.
- Dashboard’s large plus was already decorative (`aria-hidden` and noninteractive), so its presentation and Create Rule link remain unchanged.
- Rule action styling gives Enable/Disable primary emphasis, keeps Dry Run and Edit secondary, and tones down Delete; narrow layouts wrap the controls.
- Ollama health now requires a successful `/api/tags` response and reports whether `gemma3:4b` is installed. Offline and missing-model feedback are distinct.
- In recorded data, move/copy destinations are represented by resulting filesystem paths; `{date}` and `{originalName}` are rename-template placeholders, not destination formatting markers. Display code preserves any braces present in an actual path.
- Verification for this correction pass: Angular production build passes (with the existing component stylesheet soft-budget warning), TypeScript check passes, 16 Angular tests pass, Rust formatting/check pass, and 42 Rust tests pass with 1 optional live Ollama test ignored.

# Critical Rust safety hotfix — 2026-10-17

- Rename and move execution retains planner collision checks and same-device validation, then uses an atomic exclusive rename (`renameatx_np(RENAME_EXCL)` on macOS; `renameat2(RENAME_NOREPLACE)` on Linux). A destination appearing after planning is preserved; no copy-and-delete fallback is used. Collisions are recorded as skipped outcomes.
- Watcher execution now distinguishes success, terminal skip, retryable failure, partial failure, and terminal failure. Zero-action-completed transient failures receive at most two retries after the first attempt. Terminal outcomes are processed once; multi-action partial failures are recorded with completed actions/resulting path and are never automatically retried. Runtime status exposes last execution errors.
- Residual risks: multi-action changes are not rolled back. Filesystem guarantees are those of the platform's exclusive-rename primitive; this does not establish general race freedom for path authorization, folder replacement, or every filesystem implementation. Repeated transient attempts each create an Activity record. macOS packaged-app integration/manual GUI verification remains outstanding.
- Verification on macOS: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml` pass (50 passed, 1 optional live Ollama test ignored). Tests cover collision preservation, an executor-level destination created after planning, normal rename/move, retryable zero-action and partial multi-action failures, bounded watcher retries, partial terminal handling, and terminal watcher duplicate suppression/activity. Packaged-app GUI acceptance was not run.
- Manual regression: enable a rename-only rule and verify a free destination succeeds; create the intended target before triggering the file and verify it remains unchanged, the source remains, and Activity says skipped. Repeat for a move rule. To exercise the planning/execution interval, create the target while a stable file is being picked up by the watcher and verify no replacement. Trigger a multi-action rule with a competing later target and verify Activity shows `partial_failure`, the completed action and resulting path, and no automatic second run. Cause a transient disk/permission failure and verify an Activity failure plus runtime Error, with no more than two retries after the initial attempt.

# Workbench redesign — Step 1: Shell and design foundation

- Centralized the approved palette, spacing, radii, and system typography tokens in `src/styles.scss`, including Arabic-capable system fallbacks and shared focus treatment.
- Replaced the promotional sidebar and local-health badge with compact app branding and semantic navigation links labeled Workbench, Create rule, Activity, and Settings. Active links expose `aria-current="page"`.
- Kept existing routes and page components; updated their route titles/navigation metadata only. Responsive shell keeps all four navigation destinations available in narrow windows.
- Verification: `npm run build` passed (existing rule creation stylesheet budget warning); `npx tsc --noEmit -p tsconfig.app.json` passed; `npm run test -- --watch=false` passed (16 tests). The local app loaded at desktop browser width and the accessibility tree exposed all four semantic navigation links and updated Workbench document title. Narrow-window visual inspection was not available in this pass.
- Scope note: Workbench list/detail, dashboard, and rule editor visual redesign are not part of this step.
