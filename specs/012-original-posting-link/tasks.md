---

description: "Task list for opening the original posting link in the default browser"
---

# Tasks: Original Posting Link Opens in the Default Browser

**Input**: Bug report: "The original link announce is a link but when I click on
it the default navigator doesn't open" (the "Original posting" link in the job
drawer). Clicking the link does nothing; no browser window and no in-app window
appears.

**No `spec.md` or `plan.md` exists for this feature**, so these tasks are derived
from the report and the current source. The active feature pointer still targets
`specs/008-profile-layout-role-conflicts`, which is unrelated; this list is
self-contained and must not modify that feature.

**Prerequisites**: Node + npm from `package.json`, Rust 1.90+ (Tauri 2.12,
`tauri-plugin-opener` requires 1.90), a desktop session with a default browser
registered for `http`/`https` (Linux: `xdg-open`/`gio open`).

**Tests**: Required by the project rules for new behavior and failure paths.

**Organization**: `US1` (P1) makes the link open the system browser. `US2` (P2)
guarantees non-`http(s)` URLs never reach the OS. `US3` (P2) adds failure
feedback and keyboard acceptance. No story depends on another.

## Root Cause Notes (verified in the current source)

- The link is a plain anchor with `target="_blank"`:
  `src/components/JobDrawer.tsx` (`component="a"`, `href={url}`,
  `target="_blank"`, `rel="noreferrer"`, `data-testid="original-posting-url"`).
- Tauri v2 does not forward a webview `target="_blank"` to the OS. Without a
  new-window/navigation handler and without the opener plugin, the request is
  dropped, so nothing opens.
- `src-tauri/Cargo.toml` has only `tauri-plugin-dialog`; `src-tauri/src/lib.rs`
  registers only `tauri_plugin_dialog::init()`.
- `src-tauri/capabilities/default.json` grants `["core:default",
  "dialog:default"]` only, so `open_url` would be denied even if it were called.
- `safeExternalUrl` in `src/lib/format.ts` already restricts URLs to `http(s)`,
  and `JobDrawer` already derives `const url = job ? safeExternalUrl(job.url) :
  null`. Invalid URLs already render the "Original URL is missing or invalid"
  warning, so US2 is mostly regression coverage.
- `src/components/JobDrawer.test.tsx` currently asserts `target="_blank"`, which
  encodes the broken behavior and must be updated in US1.
- `useFeedback()` in `src/components/FeedbackProvider.tsx` is the existing way to
  surface a transient error to the user.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete
  task).
- **[Story]**: `[US1]` open in default browser, `[US2]` URL safety,
  `[US3]` failure feedback/accessibility.
- Every task includes an exact file path.

## Path Conventions

- Frontend components: `src/components/`
- Frontend utilities and Tauri wrappers: `src/lib/`
- Rust shell: `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`
- Capabilities: `src-tauri/capabilities/default.json`
- Feature docs: `specs/012-original-posting-link/`

---

## Phase 1: Setup

**Purpose**: Confirm the failure and add the one dependency the fix needs. Do not
change behavior yet.

- [X] T001 Reproduce the bug without changing code: run `npm run tauri dev`,
  open a job with a valid `https` URL, click "Original posting", and record that
  no browser opens and no new window appears. Also confirm `xdg-open
  https://example.com` works on this machine so the OS handler is not the cause
  (path: `src/components/JobDrawer.tsx`).
- [X] T002 Add `tauri-plugin-opener = "2"` to `[dependencies]` in
  `src-tauri/Cargo.toml` (`npm run tauri add opener` is the equivalent one-shot
  command and also edits this file).
- [X] T003 [P] Add `@tauri-apps/plugin-opener` (v2) to `dependencies` in
  `package.json` and run `npm install`; commit the updated
  `package-lock.json`.

**Checkpoint**: The failure is reproduced and both halves of the plugin are
installed; no behavior has changed.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Wire the plugin end to end so US1/US2/US3 can call one typed wrapper.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 Register `.plugin(tauri_plugin_opener::init())` in the
  `tauri::Builder` chain in `src-tauri/src/lib.rs`, next to
  `.plugin(tauri_plugin_dialog::init())` and before `.invoke_handler(...)`.
- [ ] T005 Add `"opener:default"` to `permissions` in
  `src-tauri/capabilities/default.json`. That set grants `allow-open-url` plus
  `allow-default-urls` (`http`, `https`, `mailto`, `tel`). Do not add
  `opener:allow-open-path`; the app only opens web URLs.
- [ ] T006 Add `openExternalUrl(url: string): Promise<void>` to
  `src/lib/ipc.ts`: validate with `safeExternalUrl` from `src/lib/format.ts`,
  throw the project `AppError` (via the existing `toAppError` path) for anything
  that is not `http(s)` **without** invoking the plugin, then
  `await openUrl(safe)` imported from `@tauri-apps/plugin-opener`. Components
  must call this wrapper, never the plugin directly (project IPC rule).
- [ ] T007 Verify the wiring compiles and the capability schema regenerates:
  `cargo check --manifest-path src-tauri/Cargo.toml`, `npm run typecheck`, and
  confirm `src-tauri/gen/schemas/` now lists the opener permissions.

**Checkpoint**: The plugin is registered, permitted, and callable through one
typed wrapper.

---

## Phase 3: User Story 1 - Click opens the default browser (Priority: P1) 🎯 MVP

**Goal**: Clicking "Original posting" opens the exact posting URL in the OS
default browser; the app stays on the drawer and opens no second webview.

**Independent Test**: With the drawer open for a job whose URL is
`https://example.com/jobs/1`, clicking `data-testid="original-posting-url"`
calls `openExternalUrl` once with that URL, and in `npm run tauri dev` the
default browser opens it.

### Tests for User Story 1

> **NOTE: Write these tests FIRST; they must fail before T010/T011.**

- [ ] T008 [P] [US1] Create `src/lib/ipc.test.ts`: mock
  `@tauri-apps/plugin-opener`, assert `openExternalUrl('https://example.com/jobs/1')`
  calls `openUrl` exactly once with the normalized URL, and assert a plugin
  rejection is rethrown as an `AppError` (`src/lib/errors.ts`).
- [ ] T009 [P] [US1] Update `src/components/JobDrawer.test.tsx`: mock
  `@/lib/ipc`, drop the `target="_blank"` assertion, keep the `href`,
  text-content, and `wordBreak`/`userSelect` assertions, and assert that
  `fireEvent.click(link)` calls `openExternalUrl` once with `job.url`
  (`src/components/JobDrawer.test.tsx`).

### Implementation for User Story 1

- [ ] T010 [US1] In `src/components/JobDrawer.tsx`, change the
  "Original posting" anchor: keep `component="a"`, `href={url}`,
  `data-testid="original-posting-url"`, and the existing styles; remove
  `target="_blank"`; add
  `onClick={(event) => { event.preventDefault(); void openExternalUrl(url); }}`.
  Do not use `window.open`.
- [ ] T011 [US1] Manual acceptance: `npm run tauri dev`, click the link for a
  real job. The OS default browser opens exactly the stored URL; no blank Tauri
  window appears; the drawer state is unchanged. Repeat after `npm run tauri
  build` if a release build is available.

**Checkpoint**: US1 ships alone as the MVP.

---

## Phase 4: User Story 2 - Unsafe URLs never reach the OS (Priority: P2)

**Goal**: `javascript:`, `file:`, `data:`, and malformed values are refused with
a visible message and never handed to the OS.

**Independent Test**: A drawer job with `url: 'javascript:alert(1)'` shows the
existing warning, renders no clickable posting target, and the wrapper rejects
without calling `openUrl`.

### Tests for User Story 2

- [ ] T012 [P] [US2] Extend `src/lib/ipc.test.ts`: `openExternalUrl` rejects for
  `javascript:alert(1)`, `file:///etc/passwd`, `data:text/html,x`, and
  `'not a url'`, and `openUrl` is never called.
- [ ] T013 [P] [US2] Extend `src/components/JobDrawer.test.tsx`: with
  `url: 'javascript:alert(1)'` and with `url: 'not a url'`, the
  "Original URL is missing or invalid" warning renders, no
  `original-posting-url` test id exists, and `openExternalUrl` is not called
  (`src/components/JobDrawer.tsx`, `src/lib/format.ts`).
- [ ] T014 [US2] Keep `safeExternalUrl` in `src/lib/format.ts` as the single
  validator and use it in both `JobDrawer` and `openExternalUrl`. Do not
  duplicate protocol checks; extend `src/lib/format.test.ts` only if a case is
  missing.

**Checkpoint**: US1 and US2 work independently; no unsafe scheme can be opened.

---

## Phase 5: User Story 3 - Failure feedback and keyboard access (Priority: P2)

**Goal**: If the OS cannot open the URL, the user sees an error and can still
copy the address; keyboard activation works.

**Independent Test**: A rejected wrapper call shows one error notification with
the URL; pressing Enter on the focused link opens the browser.

### Tests for User Story 3

- [ ] T015 [P] [US3] Extend `src/components/JobDrawer.test.tsx`: when
  `openExternalUrl` rejects, `notify` (from `useFeedback`) is called with
  severity `'error'` and a message containing the URL, and the drawer remains
  open (`src/components/FeedbackProvider.tsx`).
- [ ] T016 [US3] Manual keyboard check: Tab to the "Original posting" link and
  press Enter; the browser opens. Ctrl/Cmd-click and middle-click must not create
  a blank webview (the handler prevents the default navigation).

### Implementation for User Story 3

- [ ] T017 [US3] In `src/components/JobDrawer.tsx`, use `const feedback =
  useFeedback();` and append
  `.catch(() => feedback.notify(\`Could not open the posting link. Select the
  address in the drawer to copy it: ${url}\`, 'error'))` to the wrapper call.
  Keep the URL text selectable (`userSelect: 'text'`).

**Checkpoint**: All stories are independently functional; failures are visible.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T018 [P] Confirm the change stays within the project stack: MUI/Emotion
  only, function components and hooks, no new data-fetching library, Tauri calls
  only through `src/lib/ipc.ts`.
- [ ] T019 Run and report exactly which checks ran:
  `npm run typecheck`, `npm test`, `npm run lint`, `npm run format:check`,
  `cargo test --manifest-path src-tauri/Cargo.toml`,
  `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [ ] T020 [P] Review the capability diff: only `opener:default` is added; the
  dialog and `core` permissions, the MCP servers, and the CSP
  (`src-tauri/tauri.conf.json`) are untouched. No `opener:allow-open-path`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Starts immediately. T002 and T003 are independent but both
  are needed before T004.
- **Foundational (Phase 2)**: Depends on Phase 1; blocks all stories. T005
  depends on T002; T006 depends on T003; T007 depends on T004 and T005.
- **User Stories (Phase 3+)**: All depend on Phase 2. US1, US2, and US3 edit the
  same two test files, so run them sequentially in priority order to avoid
  conflicts; the `[P]` markers are for independent files within a story.
- **Polish (Phase 6)**: Depends on all selected stories.

### Within Each User Story

- Tests (T008/T009, T012/T013, T015) are written first and must fail.
- Wrapper (`src/lib/ipc.ts`) before the component (`src/components/JobDrawer.tsx`).
- Manual acceptance before marking the story complete.

### Parallel Opportunities

- T003 can run while T002 installs the Rust crate.
- T008 and T009 touch different files and can run together.
- T012 and T013 touch different files and can run together.
- T018 and T020 can run together after the stories land.

---

## Parallel Example: User Story 1

```bash
# US1 tests (different files, same contract):
Task: "Create src/lib/ipc.test.ts covering openExternalUrl"
Task: "Update src/components/JobDrawer.test.tsx to assert the click handler"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Complete Phase 1 (setup and reproduction).
2. Complete Phase 2 (plugin registered, permitted, wrapper exported).
3. Complete Phase 3 (US1) and run T011 manually.
4. **STOP and VALIDATE**: the link opens in the default browser.
5. Ship/deploy.

### Incremental Delivery

1. Phase 1 + Phase 2 → foundation ready.
2. US1 → validate → MVP.
3. US2 → validate → no unsafe scheme can be opened.
4. US3 → validate → failures are actionable and keyboard-friendly.
5. Phase 6 → full verification and capability review.

---

## Notes

- The `[P]` marker means different files and no incomplete dependency.
- Never open the URL with `window.open` or by rendering a bare
  `target="_blank"` anchor; Tauri drops both.
- Keep the visible URL text selectable as a manual fallback.
- Do not add `opener:allow-open-path` or accept `file:`/`data:` URLs.
- Commit after each story; stop at each checkpoint to validate independently.
