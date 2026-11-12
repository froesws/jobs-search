# Implementation Plan: Provider dialog drag and drop

**Branch**: `main` (feature 016) | **Date**: 2026-11-12 | **Spec**: [spec.md](./spec.md)

## Summary

Repair the pointer-driven provider reordering in `ProviderConfigDialog` without
changing the dialog's save contract. Make the ghost mount at the press point and
render above the modal, resolve drops from live row geometry (including gaps and
boundaries), remove the dependency on pointer capture, ignore non-primary
pointers, and add Escape/`pointercancel` cancellation with an order snapshot.

## Technical Context

**Language/Version**: TypeScript 7.0.2, React 19.3.1

**Primary Dependencies**: MUI 9 (`Portal`/`Box`), `react-dom` `createPortal`,
Vitest + Testing Library + jsdom. **No new dependency.**

**Storage**: none; the provider order is local dialog draft state until Save.

**Testing**: `src/components/ProviderConfigDialog.test.tsx` with stubbed
`getBoundingClientRect`; add `src/components/providerDrag.test.ts` for pure
geometry helpers. Manual verification in `npm run tauri dev` (WebKitGTK).

**Target Platform**: Tauri v2 desktop shell on Linux (WebKitGTK).

**Constraints**: no HTML5 drop events; no pointer capture as a requirement; ghost
must sit above `theme.zIndex.modal`; keyboard reordering must keep working.

## Root Causes

| # | Defect | Location |
|---|--------|----------|
| 1 | Ghost transform is set on `pointerdown` before the ghost is mounted, so it first renders at the viewport origin. | `startDrag` and the conditional ghost in `ProviderConfigDialog.tsx` |
| 2 | The ghost is a sibling of the dialog outside its portal, which is fragile against stacking contexts and clipping. | ghost JSX |
| 3 | Hit-testing reads an index-keyed ref array and rejects gap coordinates. | `targetIndex` in the `dragging` effect |
| 4 | `setPointerCapture` on the handle that React re-orders can throw or be lost, ending the drag after one move. | `startDrag` |
| 5 | No Escape/`pointercancel` restore and no active-pointer guard. | drag effect |

## File Structure

```text
src/components/
  ProviderConfigDialog.tsx       dialog, ghost portal, drag effect
  ProviderConfigDialog.test.tsx  interaction coverage
  providerDrag.ts                pure ghost/hit-test helpers
  providerDrag.test.ts           helper coverage
specs/016-provider-drag-ghost/
  spec.md, plan.md, quickstart.md, tasks.md
```

## Design Decisions

### Ghost mounting

Keep the ghost rendered from dialog state, but store the latest pointer position
in a ref and position the ghost in a `useLayoutEffect` that runs when the drag
starts (and when the ghost element changes). The ghost is wrapped in
`createPortal(..., document.body)` so its `zIndex: theme.zIndex.modal + 2`
competes directly with the modal root. Position is computed as
`clientX + 14`, `clientY - 18` by one helper used by both start and move.

### Hit-testing

Read rows from a container ref (`querySelectorAll('[data-drag-row]')`) on each
move and build `{ index, top, bottom }` from live rects. Select the row whose
rect contains `clientY`; otherwise pick the nearest row by center distance.
Clamp below the first and above the last row. Never cache the rect list.

### Input handling

Do not call `setPointerCapture`. Attach `pointermove`/`pointerup`/`pointercancel`
to `window` while a drag is active, store the initiating `pointerId`, and ignore
events from any other pointer. `pointerup` commits; `pointercancel` and Escape
restore the snapshot and clear the ghost.

### Accessibility

Keep the handle role, `tabIndex`, `aria-label`, and the arrow buttons. Add a
visually hidden `aria-live="polite"` region that announces
`"<Provider> moved to position <n> of <total>"` after each reorder.

## Verification

- `npx vitest run src/components/ProviderConfigDialog.test.tsx src/components/providerDrag.test.ts`
- `./scripts/verify.sh`
- Manual: `npm run tauri dev`, then drag with the mouse on Linux.
