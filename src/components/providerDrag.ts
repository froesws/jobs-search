/**
 * Pure geometry for the provider drag interaction.
 *
 * WebKitGTK does not deliver HTML5 drop events reliably, so the dialog
 * reorders providers from pointer coordinates. Keeping the math pure makes it
 * testable without a browser.
 */

export interface DragPoint {
  x: number;
  y: number;
}

export interface RowBounds {
  top: number;
  bottom: number;
}

const GHOST_OFFSET_X = 14;
const GHOST_OFFSET_Y = 18;

/** Viewport transform placing the ghost just below-right of the pointer. */
export function ghostTransform(point: DragPoint): string {
  return `translate(${point.x + GHOST_OFFSET_X}px, ${point.y - GHOST_OFFSET_Y}px)`;
}

/**
 * Resolve a pointer's vertical position to a row index.
 *
 * A coordinate inside a row selects that row. A coordinate in a gap, above the
 * first row, or below the last row selects the nearest row by center distance,
 * which also clamps to the ends of the list.
 */
export function rowIndexAt(rows: RowBounds[], clientY: number): number | null {
  if (rows.length === 0) return null;

  for (let index = 0; index < rows.length; index += 1) {
    const row = rows[index]!;
    if (clientY >= row.top && clientY <= row.bottom) return index;
  }

  let nearest = 0;
  let nearestDistance = Number.POSITIVE_INFINITY;
  rows.forEach((row, index) => {
    const center = (row.top + row.bottom) / 2;
    const distance = Math.abs(clientY - center);
    if (distance < nearestDistance) {
      nearest = index;
      nearestDistance = distance;
    }
  });
  return nearest;
}
