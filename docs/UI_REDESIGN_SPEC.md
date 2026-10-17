# TeachMyPC Workbench Redesign

**Design direction:** Automation Workbench + Quiet Mac Utility  
**Product concept:** Teach your Mac once. See exactly what it learned.

## Scope

This specification changes the Angular presentation and navigation only. It preserves the Tauri commands and their payloads, rule persistence, runtime, folder authorization, file handling, and activity data. The workflow display reads the existing `SavedRule`, `FolderChoice`, and `RuleRuntimeStatus` data; it must not introduce a second workflow model.

The app already supports local Ollama interpretation, English and Arabic instructions, supported/clarification/unsupported responses, editable rule fields, folder selection, disabled-by-default saves, dry run for saved rules, explicit enable/disable, deletion confirmation, runtime state, and activity history. Do not imply scheduling, manual execution, drag-and-drop ordering, search across activity, custom triggers, or rule-level preferences that do not exist.

## 1. Product design principles

1. **Show the automation, not an AI dashboard.** A rule’s trigger, extension filter, source folder, ordered actions, and status are the primary visual content.
2. **Make the Mac utility feel quiet and native.** Use a compact title/navigation rail, restrained separators, system control conventions, and minimal ornament. Avoid oversized welcome copy, decorative hero art, metric-card grids, and promotional language.
3. **Separate the saved definition from execution.** Draft configuration is labeled as a draft; saved rules show enabled state and actual runtime state; history describes completed, skipped, or failed outcomes; dry run is visibly preview-only.
4. **Use density with hierarchy.** Keep rows compact and scannable, but preserve readable labels and enough spacing for pointer and keyboard use.
5. **Use mint sparingly.** Mint marks the selected destination/action, keyboard focus, or healthy state. It is not a general-purpose decoration.
6. **Use only real state.** Counts and labels derive from store signals. No sample rules, fake “last run” fields, fabricated health checks, or placeholder actions represented as live data.
7. **Keep safety visible at the decision point.** Show the source folder and destinations before save/enable; distinguish “Save disabled” from “Enable”; retain confirmation before enabling.
8. **Support mixed-direction input.** Keep instruction text and file/folder paths readable for Arabic and English, with direction chosen per text field/value instead of forcing the whole application RTL.

## 2. Information architecture

### Primary navigation

- **Workbench** (existing `/` route): searchable rules list and selected rule detail. This replaces the dashboard as the default route.
- **Create rule** (existing `/rules` route): natural-language composer and configuration editor. The saved-rules library moves to the Workbench so there is one canonical list.
- **Activity** (existing `/activity` route): actual execution history.
- **Settings** (existing `/settings` route): existing app/backend and local AI status only.

Keep paths stable where possible. The existing `/rules` title says “My Rules”; update route title/navigation text only as part of UI work, without changing Tauri contracts. The wildcard still returns to `/`.

### Workbench selection model

- The list is populated from `RuleStoreService.rules()` and filtered by a local text query against the existing rule title, extensions, and action type. Search is a view filter, not persisted state.
- Selecting a row displays that `SavedRule` in the detail pane. On narrow windows, selection opens a detail view with a Back-to-rules control; do not require horizontal scrolling.
- If there are no rules, show an empty state with a Create rule link. If a search has no matches, show “No matching rules” and a clear-search control.
- Do not add status filters unless they use already loaded states and are included in the implementation scope; search is the only newly proposed control.

## 3. Screen-by-screen layout specifications

### 3.1 Workbench (default)

Use a compact application frame: a narrow persistent sidebar (roughly 200–220 px at wide sizes), a top title/toolbar row, and a two-pane workspace. The list pane is approximately 300–340 px; the detail pane takes the remaining width. The page should work down to a 320 px window by switching to a single-pane list/detail navigation.

Top row: page title “Workbench”, a small local runtime context label only if based on existing available runtime signals, and a primary “New rule” action. Avoid a generic global “local/private” green badge that implies active health; privacy can remain quiet text in Settings.

List pane: “Rules” heading, count derived from loaded rules, labeled search field, and compact selectable rows. Each row shows title, a concise extension summary (including “All regular files” when the extension list is empty), ordered action names, and state. State should distinguish saved enabled state from runtime state: Disabled when `enabled === false`; otherwise Running/Starting/Stopped/Error from `statusFor(id)`, or “Status unavailable” on runtime error. Never display Running solely because `enabled` is true.

Detail pane: selected rule title, saved/enabled indicator, runtime status, and contextual Edit, Dry Run, Enable/Disable actions. Below, show the read-only workflow visualization, then compact definition metadata (source folder, extension filter, created/updated metadata only if actually displayed and useful). Enable retains the existing confirmation and states the folder that will be monitored. Dry Run invokes the existing sample-file picker and preview; present its result inline with the explicit “Preview only. No files will be modified.” assurance, match result, planned actions, collisions, and errors.

When runtime is unavailable, keep rule configuration visible and show the service error adjacent to status rather than replacing the screen. When list data fails to load, show a recoverable error and retry affordance only if an existing reload action can be exposed without new backend behavior.

### 3.2 Rule creation and editing

Keep the existing `/rules` route as a focused editor, not a page with a second rules library. Use a narrow content column (about 760–820 px) and a compact title (“Create rule” or “Edit rule”). The form is two clear zones:

1. **Describe**: labeled multiline instruction input, 2,000-character count, helper stating English and Arabic are supported, optional existing example prompts, and “Interpret” action.
2. **Review and configure**: only shown after a supported interpretation or when editing an existing rule. Fields remain title, extension filter, file-created trigger (read-only), authorized source folder, and ordered rename/move/copy actions. Use existing selectors and up/down/remove controls. A concise visual summary may accompany the editor but must not be a separate editable model.

Interpretation result states are inline and specific: supported opens the editor; clarification questions remain visible; unsupported stays explanatory; Ollama/model/timeout/schema errors offer retry. A new interpretation can overwrite draft fields only through the existing explicit Interpret action. Folder selection remains a native picker flow.

Primary save label is “Save disabled rule” for new rules and “Update rule” for edits. Save stays unavailable until title, source folder, at least one action, and every move/copy destination are present. Do not treat a successful save as enabled. Preserve the existing enable confirmation in Workbench.

The current UI advertises a five-step process including “Dry run, then enable”, but the draft itself has no dry-run operation: dry run is available for saved rules. Reflect this accurately in copy. A creation sequence can describe Interpret → configure/authorize → save disabled → dry run from the saved rule → enable, but do not add a preview IPC path for drafts.

### 3.3 Rule detail (Workbench detail pane)

This is a pane, not a separate route or duplicated business object. Header: rule title, saved state, actual runtime state, and action buttons. Show trigger and filter as a compact “When” block. Show source folder separately and clearly as the watched/authorized folder. Render each saved action in persisted order, with exact rename template or resolved destination folder label. If folder labels are unavailable, use the existing safe fallback “Authorized folder”; never infer a path from AI text.

Edit opens the existing editor prefilled by `edit(rule)`. Dry Run opens the current file picker and displays the existing `PlanPreview`; an empty picker selection closes/keeps the preview unchanged per current behavior. Enable/Disable uses current confirmation semantics. Delete remains available in the list/detail context only with the existing irreversible confirmation; do not give it visual prominence over safe primary actions.

### 3.4 Activity

Compact chronological list using the order returned by the current store. Each row shows outcome (success/skipped/failed), timestamp, rule title (or “Deleted rule”), source filename, action summary from completed actions when present and intended actions otherwise, and error when present. Preserve “Full destination path” disclosure using `resultingPath`. Avoid invented duration, trigger counts, filters, export, or per-step event timelines.

Use a narrow, readable column with light row separators rather than large cards. A failed/skipped record remains visually distinct through text and icon/color; color is not the only signal. Retain activity loading, no-history, and error states. Existing polling behavior is not part of the design change.

### 3.5 Settings

Keep two compact grouped sections: **Application** (name, version, backend connection) and **Local AI** (endpoint, model, connection/model availability). These values come from `AppInfoService`. Preserve error messages. Remove decorative privacy claims if their exact guarantee is not a surfaced runtime value; a concise local-processing explanation may describe the established product behavior. Do not add preferences or retry controls unless implemented using an existing service operation.

## 4. Workflow visualization specification

Render a read-only semantic sequence from the selected rule’s existing validated/saved data:

1. **Trigger** — `file_created`, labeled “A new file arrives” / “A new file is created”.
2. **Filter** — extension list from `rule.trigger.extensions`; empty means “All regular files” as used by the existing UI. Do not add hidden filters.
3. **Source** — folder resolved by `rule.sourceFolderId` against `store.folders()`; fallback label “Authorized folder” when no display name is loaded.
4. **Actions** — render `rule.actions` in array order. Rename shows `pattern` and note “Original extension is preserved” only where relevant to the supported rename behavior. Move/Copy shows action verb and folder resolved from `destinationFolderId`, with the safe fallback on missing names.
5. **Runtime** — a separate status badge from `RuleRuntimeStatus`, not another workflow node. Disabled rules display Disabled; enabled rules use actual state (Starting, Running, Stopped, Error) or Status unavailable.

Use a vertical connected sequence in the detail pane for clarity and narrow widths. On very wide windows, nodes may align on a horizontal track only if text still wraps and tab order remains logical. Use numbered action labels (“Action 1”, etc.) so order is clear without color. Avoid drag handles, editing affordances, animated execution, or node graph semantics. The read-only summary may be a shared presentational component with inputs derived directly from `SavedRule` and folder/status lookup functions; do not map into an independently mutable workflow store.

## 5. Component inventory

Reuse existing standalone page components and `RuleStoreService`/`RuleInterpreterService`/`AppInfoService`. Proposed presentational boundaries:

- `App` shell: navigation rail, page title context, router outlet.
- Workbench page: coordinates list selection/search and selected rule actions.
- Rule list/search region: query input and rule rows; no new data service.
- Rule detail region: selected rule metadata and contextual actions.
- Read-only workflow summary: trigger, filter, source, ordered actions, runtime state; inputs use existing model types.
- Dry-run result region: existing `PlanPreview` presentation, including warnings/errors.
- Rule editor sections: instruction composer, interpretation result, rule fields/actions, authorization/safety summary.
- Activity row: one actual `ActivityRecord` with path disclosure.
- Settings status groups: existing app/runtime and AI state.
- Shared empty/error/loading/status primitives only where repeated UI truly benefits; avoid a generalized design framework.

The current `EmptyFeaturePage` is not needed for core routes. Do not add UI dependencies.

## 6. Visual tokens

These are design targets, to be checked against existing values in `src/styles.scss`; implementation should centralize tokens rather than adding per-page literal colors.

| Token | Target | Use |
|---|---|---|
| Canvas | `#111719` | Window background |
| Sidebar | `#101517` | Navigation rail |
| Surface | `#171F21` | List/detail/editor regions |
| Raised surface | `#1B2527` | Hover, selected row, compact inset groups |
| Border | `#293336` | Hairlines and control outlines |
| Primary text | `#E9F0ED` | Titles and main content |
| Secondary text | `#A0ACAE` | Supporting labels |
| Muted text | `#7F8C8F` | Metadata; maintain contrast for actual text sizes |
| Mint accent | `#74DCBA` | Focus, selected/healthy state, primary safe action |
| Warning | `#D9B96F` | Skipped/collision state |
| Error | `#E4A49B` | Failures and actionable errors |
| Disabled | neutral gray, not mint | Inactive saved rule/control |

**Typography:** Prefer `-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif` for the desktop utility feel. Existing DM Sans/Manrope can be retained only if bundled/loaded consistently; avoid using an unprovided font name as an assumed asset. Body 13–14 px, list metadata 11–12 px, section labels 11 px with restrained tracking, page heading 22–26 px, rule title 20–24 px. Use tabular numerals for timestamps/counts where helpful. Respect Arabic shaping and choose a font with Arabic coverage.

**Spacing:** 4 px base rhythm; common increments 8, 12, 16, 20, 24. Sidebar horizontal padding 12–16 px, pane gap 12 px, row height around 58–68 px, form group spacing 16–20 px. Corner radius 6–9 px for controls and rows, 10–12 px for larger regions. Keep shadows absent or barely perceptible; use borders and surface shifts.

**States:** Selected list row has a clear border/left marker and surface change; hover is subtler. Keyboard focus uses a 2 px mint outline with 2–3 px offset. Healthy runtime has text plus a small dot; warning/error use icon/text and distinct hue. Disabled is neutral. Loading uses stable text or a small spinner with `role="status"`; respect reduced-motion preference. Buttons have clear enabled/disabled states and minimum hit target 36 px, with 44 px preferred for touch-like narrow-window use.

## 7. Interaction and keyboard behavior

- Navigation links remain semantic anchors and expose the active route. Sidebar controls and all actions are keyboard reachable.
- Search uses a visible or programmatic label, supports `Escape` to clear only when focused/has text, and does not steal focus on route load.
- Rule rows are buttons or links with an accessible selected state; avoid clickable generic containers. `Enter`/`Space` selects. Do not invent global shortcuts.
- Workbench selection has a visible focus ring. When selection changes, keep focus on the selected row; narrow detail Back restores focus to the originating row.
- Action reorder remains explicit up/down buttons with meaningful accessible names and disabled boundaries; no drag-and-drop.
- Folder controls announce their purpose (“Select source folder”, “Select destination for action 2”).
- Save, enable, delete, dry-run, and interpretation retain their current semantics. Confirmation dialogs remain keyboard accessible; prefer a native accessible dialog if the app’s established patterns support it.
- Status updates should not repeatedly announce each polling tick. Use live regions for meaningful completion/error transitions, not the entire changing list.
- `textarea` supports both scripts. Apply `dir="auto"` to instruction input and user-provided text values; preserve LTR direction for paths/extensions where needed while allowing wrapping.

## 8. Empty, loading, and error states

- **Workbench rules loading:** compact “Loading rules…” status; avoid showing a zero count until load completes.
- **No saved rules:** explain that rules appear after saving a reviewed rule; primary Create rule action.
- **Search no match:** explain query returned no rules and offer clear search.
- **Rule load error:** concise error from store with retry if implemented through existing `load()`.
- **Runtime error:** show “Status unavailable” and available runtime error near status; retain rule and actions.
- **Interpretation loading:** disable duplicate submission and label “Interpreting…”.
- **Clarification/unsupported/error:** preserve returned message and questions; retry only for retryable errors or user rephrasing.
- **Folder picker canceled:** remain in editor without replacing a previously selected folder.
- **Save error:** inline alert near save controls; retain entered fields.
- **Dry run loading:** announce sample-file picker/preview loading. Cancel leaves no fabricated result. Preview mismatch, errors, and collisions use distinct readable messages.
- **Activity loading/no history/error:** preserve current states and do not represent absence as a successful empty run.
- **Settings loading/unavailable:** preserve current explanatory and connection states.

## 9. Accessibility and responsive behavior

- Use landmark elements once per screen, hierarchical headings, explicit labels, semantic lists/tables where useful, and `aria-current`/`aria-selected` where appropriate.
- Maintain WCAG AA contrast for text, status indicators, and focus. Muted small labels need contrast validation; do not rely on low-contrast tracking text.
- Never use hue as the only status cue. Pair state names with icon/shape or border treatment.
- Keep a logical DOM/tab order: navigation → list/search → detail actions → workflow → remaining content. On narrow selected detail view, focus moves to the detail heading and returns to the prior row on Back.
- At wide windows (≥ 1000 px): persistent narrow navigation plus list/detail split.
- At intermediate windows (roughly 700–999 px): reduce navigation width and list pane; selected detail remains legible. If insufficient, switch to list/detail navigation rather than squeezing text.
- At narrow windows (< 700 px): compact top navigation or collapsible navigation consistent with current horizontal navigation; list and detail occupy separate views. Stack editor controls, action rows, and settings metadata; allow path and Arabic/English text wrapping. No horizontal page scrolling at 320 px.
- At all widths, retain usable text sizing and controls; do not hide Settings or primary actions solely for space.
- Respect `prefers-reduced-motion`; animations are optional and should not communicate state exclusively.

## 10. Angular implementation mapping

- `src/app/app.html` and `src/app/app.scss`: replace marketing-style sidebar copy/badge and establish compact shell; preserve existing router links and mobile navigation semantics.
- `src/app/app.routes.ts`: keep paths, set page titles to Workbench/Create rule/Activity/Settings as appropriate; `/` becomes Workbench.
- `src/app/features/dashboard/dashboard-page.page.*`: convert the current welcome hero, metric cards, and latest activity preview into the Workbench list/detail composition. Use existing store signals and runtime helpers; no backend changes.
- `src/app/features/rules/rule-creation.page.*`: focus on creation/editing; remove the saved rules library from this screen and move rule actions to Workbench. Reuse existing signals, validation, folder picker, save, dry-run service calls, and enable confirmation where actions move.
- `src/app/features/activity/activity-page.page.*`: compact actual-record rows; retain helper semantics and full destination disclosure.
- `src/app/features/settings/settings-page.page.*`: compact status groups with same service signals.
- `src/styles.scss`: centralize target tokens, focus, baseline typography, and Arabic/path direction utilities; remove globally scoped `.rule-page .workflow` presentation rules when workflow layout is represented in scoped styles.
- `src/app/core/models/interpretation.model.ts`, all service files, Rust source, and Tauri command contracts: no changes required or permitted for this redesign.

Use Angular standalone components and signals already present. Keep event orchestration in pages and render-only workflow/activity components focused. Do not use `any`, unsafe casts, or silent catches. If keeping a single component is simpler for this small app, avoid extracting components solely to match this inventory.

## 11. Incremental implementation plan

1. **Shell and tokens:** refine existing shell/navigation and establish tokens/focus/type foundations. Confirm all routes remain reachable and title context remains correct.
2. **Workbench:** replace the dashboard hero/metrics with searchable rules list, selected detail, workflow summary, and contextual actions wired to existing services. Handle empty/error/runtime states.
3. **Rule editor:** simplify the current rule page to composer plus configuration, remove duplicate saved library, preserve all existing validation and interpretation states, and make post-save navigation/action discoverable.
4. **Activity and Settings:** restyle existing screens without altering their data sources or refresh behavior.
5. **Responsive/accessibility pass:** verify narrow/wide layouts, keyboard selection/actions, screen-reader state names, Arabic input, wrapping paths, and reduced motion.
6. **Regression verification:** exercise existing interpretation, folder selection, save-disabled, edit, dry-run, enable/disable, delete, runtime display, and activity outcomes in the desktop app. Do not modify automation/backend behavior to satisfy a visual state.

## 12. Regression risks and acceptance criteria

### Regression risks

- Moving My Rules out of `/rules` could strand users unless navigation and post-save destination are clear; preserve the route and editor access.
- Current editor’s `edit(rule)` restores saved data; moving that interaction must not lose the existing source folder, destination labels, or action order.
- Enable confirmation communicates monitored folder and possible file operations. Keep it when moving the action.
- Runtime status is polled and can be Starting/Running/Stopped/Error; do not infer from the enabled boolean.
- Dry run requires a saved rule ID and a selected sample path. Do not imply unsaved drafts can be previewed.
- Empty extension arrays mean all regular files in current labels; preserve that interpretation.
- Destination folder names may be missing from loaded folders; retain a safe fallback and never display AI-proposed path as authorized.
- Activity action summary prefers completed actions and falls back to intended actions; preserve that distinction and path/error details.
- Arabic is currently accepted in instruction input and fallback titles; redesign must not force LTR direction or clip mixed-script strings.
- Desktop-only APIs report unavailable outside Tauri; keep those errors truthful in browser/dev contexts.

### Acceptance criteria

- Default route presents actual saved rules and their workflow, with no decorative hero or fabricated metrics.
- Every workflow element is derived from the selected existing `SavedRule`, folder list, and runtime status; action sequence matches array order.
- List labels distinguish disabled, starting, running, stopped, error, and unavailable accurately.
- Search filters only loaded actual rule data and has empty/no-match states.
- Create/edit retains bilingual instruction input, interpretation clarification/unsupported/error states, editable configuration, explicit folder authorization, and validation.
- New rules save disabled. Enable remains a separate explicit confirmed action. Dry run remains preview-only and only operates on saved rules.
- Activity shows actual status, timestamps, filenames, action result/intention, errors, and optional full resulting path.
- Settings shows only existing app/runtime and Ollama/model information and its current state.
- All existing routes remain reachable; no Tauri IPC signatures, Rust/runtime behavior, persistence format, watcher, or filesystem behavior changes.
- Keyboard-only users can navigate and activate every control with visible focus; status is not communicated by color alone.
- Layout remains usable at 320 px and on a wide macOS window without horizontal page scrolling; Arabic and long paths wrap correctly.
- No new UI library or separate workflow/business data model is added.

## ASCII wireframes

### Workbench

```text
┌ TeachMyPC ──────────────────────────────────────────────────────────────────┐
│ Workbench                                      Search rules…      + New rule │
├──────────────┬───────────────────────────┬──────────────────────────────────┤
│ Workbench    │ Rules (3)                 │ Invoice PDFs              [Edit] │
│ Create rule  │ [ Search rules…        ]  │ Enabled · Running   [Dry Run]   │
│ Activity     │                           │                         [Disable]│
│              │ Invoice PDFs              │ WHEN  A new file is created      │
│ Settings     │ .pdf · Rename → Move      │ IF    .pdf                         │
│              │ ● Running                 │ IN    Downloads (authorized)     │
│              │                           │ THEN  1 Rename {date}-{original} │
│              │ Photos backup             │       2 Move → Invoices          │
│              │ .jpg · Copy               │                                  │
│              │ ○ Disabled                │                                  │
│              │                           │                                  │
│              │ + Add rule                │                                  │
└──────────────┴───────────────────────────┴──────────────────────────────────┘
```

### Rule creation

```text
┌ TeachMyPC ──────────────────────────────────────────────────────────────────┐
│ Create rule                                                   Workbench     │
│ Describe a file task; review every field before saving.                      │
│                                                                              │
│ INSTRUCTION · English and Arabic supported                                  │
│ ┌────────────────────────────────────────────────────────────────────────┐ │
│ │ When a new PDF arrives, rename it with today's date…                   │ │
│ └────────────────────────────────────────────────────────────────────────┘ │
│ 124 / 2,000                                      [ Interpret instruction ]   │
│                                                                              │
│ INTERPRETATION · Supported                                                   │
│ ┌ Rule title ──────────────────┐  ┌ Extensions ──────────────────────────┐  │
│ │ Date-stamp PDFs              │  │ .pdf                                │  │
│ └──────────────────────────────┘  └─────────────────────────────────────┘  │
│ WHEN  A new file is created                                                 │
│ IN    Downloads (authorized folder)                     [ Select folder ]   │
│ THEN  1 Rename  [{date}-{originalName}]                         [↑][↓]     │
│       + Rename   + Move   + Copy                                            │
│                                                                              │
│ Save stores this rule disabled. Dry Run is available after saving.          │
│                                                    [ Save disabled rule ]    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Rule detail (narrow-window detail view)

```text
┌ TeachMyPC ─────────────────────────────┐
│ ‹ Rules                                │
│ Invoice PDFs                           │
│ Enabled · Running                      │
│ [Edit] [Dry Run] [Disable]             │
│                                        │
│ WORKFLOW                               │
│ ① WHEN                                  │
│ │ A new file is created                 │
│ ② FILTER                                │
│ │ .pdf                                  │
│ ③ SOURCE                                │
│ │ Downloads · authorized                │
│ ④ ACTIONS                               │
│ │ 1. Rename {date}-{originalName}       │
│ │ 2. Move → Invoices                   │
│                                        │
│ Dry Run result appears here after the   │
│ sample-file picker; preview only.      │
└────────────────────────────────────────┘
```

### Activity

```text
┌ TeachMyPC ──────────────────────────────────────────────────────────────────┐
│ Activity                                                                    │
│ Recent actions performed by enabled rules                                  │
│                                                                              │
│ SUCCESS · 10:42 AM                                                          │
│ Invoice PDFs                                                                │
│ invoice-scan.pdf                                                            │
│ Rename → Move to Invoices                                                   │
│ ▸ Full destination path                                                     │
├──────────────────────────────────────────────────────────────────────────────┤
│ FAILED · 10:18 AM                                                           │
│ Photos backup                                                               │
│ IMG_2048.jpg                                                                │
│ Copy to Photos                                                              │
│ Could not complete operation…                                               │
│ ▸ Full destination path                                                     │
└──────────────────────────────────────────────────────────────────────────────┘
```
