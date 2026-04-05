/**
 * Widget size and orientation for adaptive layouts.
 * - small:  1×1
 * - medium: up to 2×2, with orientation hint (wide/tall/square)
 * - large:  3×3+ — full detail, auto-expand
 */

export type WidgetSize = 'small' | 'medium' | 'large';
export type Orientation = 'wide' | 'tall' | 'square';

export function getWidgetSize(w: number, h: number): WidgetSize {
  if (w <= 1 && h <= 1) return 'small';
  if (w <= 2 || h <= 2) return 'medium';
  return 'large';
}

export function getOrientation(w: number, h: number): Orientation {
  if (w > h) return 'wide';
  if (h > w) return 'tall';
  return 'square';
}

/** Estimate how many list items fit based on pixel height. */
export function itemsForHeight(h: number, rowHeightPx: number, perItemPx: number = 48): number {
  const overhead = 24;
  const available = h * rowHeightPx - overhead;
  return Math.max(1, Math.floor(available / perItemPx));
}

export interface WidgetDimensions {
  w: number;
  h: number;
  size: WidgetSize;
  orientation: Orientation;
  rowHeightPx: number;
}
