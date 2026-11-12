# Quickstart: Provider dialog drag and drop

**Feature**: `016-provider-drag-ghost` | **Date**: 2026-11-12

## Automated checks

```bash
npx vitest run src/components/providerDrag.test.ts src/components/ProviderConfigDialog.test.tsx
./scripts/verify.sh
```

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
| | | | |
