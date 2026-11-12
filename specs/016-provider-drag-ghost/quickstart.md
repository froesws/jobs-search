# Quickstart: Provider dialog drag and drop

**Feature**: `016-provider-drag-ghost` | **Date**: 2026-11-12

## Automated checks

```bash
npx vitest run src/components/providerDrag.test.ts src/components/ProviderConfigDialog.test.tsx
./scripts/verify.sh
```

## Observed defects (2026-11-12)

Reproduce each before fixing:

1. Press a reorder handle and do not move. The ghost appears at the top-left
   corner of the window (0,0) and only jumps to the pointer on the first move.
2. Move the pointer through the gap between two rows, or beyond the first/last
   row. The list does not follow, or jumps back to the previous row.
3. Drag quickly across several rows. The drag sometimes stops after one position
   and the ghost freezes.
4. There is no way to cancel a drag; Escape leaves the dialog in the dragged
   order.

## Manual verification (required)

jsdom cannot reproduce WebKitGTK pointer behavior, so verify in the real shell:

```bash
npm run tauri dev
```

1. Open the gear button next to the provider indicator.
2. Press and hold a reorder handle without moving the pointer.
   - Expected: the ghost appears at the press point (offset down-right), not at
     the window corner.
3. Move the pointer slowly across rows and through the gaps between rows.
   - Expected: the ghost tracks the pointer; the dragged provider follows the
     row under the pointer with no lost moves.
4. Drag the last provider over the first row in one quick motion.
   - Expected: it lands in position 1.
5. Drag above the list and below the list.
   - Expected: the provider clamps to the first or last position.
6. Start a drag, move two positions, press Escape.
   - Expected: the original order returns and the ghost disappears.
7. Start a drag and release outside the dialog.
   - Expected: the order commits and the ghost disappears.
8. Use the ↑/↓ buttons and the handle's ArrowUp/ArrowDown keys.
   - Expected: the keyboard reorder still works and the position is announced.
9. Save, reopen the dialog.
   - Expected: the saved order is shown.

Record any WebKitGTK-specific deviation in this file.

## Result log

| Date | Environment | Result | Notes |
|------|-------------|--------|-------|
| 2026-11-12 | Linux, Wayland, `tauri dev` | Passed | Operator confirmed all four scenarios in the running app: ghost at the press point, gap and multi-row drops, last-to-first drag, Escape restore. |

## Automated coverage

| Scenario | Covered by |
|----------|------------|
| Ghost mounts at the press point | `ProviderConfigDialog.test.tsx` "positions the ghost at the press point…" |
| Ghost is portaled above the modal | same test (`parentElement`, `zIndex`) |
| Multi-row, gap, and boundary drops | "moves the dragged provider across several rows…", "uses the nearest row…", "clamps a drop above or below the list…" |
| Multi-pointer isolation | "ignores pointers other than the one that started the drag" |
| Mid-drag re-render | "keeps hit-testing after the dialog re-renders mid-drag" |
| Escape / pointercancel restore | "restores the pre-drag order when Escape cancels the drag", "…on pointercancel" |
| pointerup commits | "keeps the new order when the drag commits on pointerup" |
| Live announcements | "announces each reorder to assistive technology" |
| Pure geometry | `providerDrag.test.ts` |
