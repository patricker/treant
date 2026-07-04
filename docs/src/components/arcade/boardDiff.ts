import { useEffect, useRef } from 'react';

/**
 * Returns the board string from the previous committed render (or '' on the
 * first render). Used to drive one-shot "what just changed" animations without
 * a per-piece identity model.
 */
export function usePrevBoard(board: string): string {
  const ref = useRef('');
  const prev = ref.current;
  useEffect(() => {
    ref.current = board;
  }, [board]);
  return prev;
}

/**
 * Every index whose character differs between `prev` and `curr` — placements,
 * departures AND in-place changes (Reversi flips). Empty when there is no
 * prior board or the length changed. Drives the "last move" marker.
 */
export function diffCells(prev: string, curr: string): number[] {
  if (!prev || prev.length !== curr.length) return [];
  const out: number[] = [];
  for (let i = 0; i < curr.length; i++) {
    if (prev[i] !== curr[i]) out.push(i);
  }
  return out;
}

/**
 * The first index that went from empty -> occupied between `prev` and `curr`
 * (a placement or a piece that just arrived), or -1. Returns -1 when there is
 * no prior board (first render) so nothing animates on mount.
 */
export function changedIndex(prev: string, curr: string, empty = ' '): number {
  if (!prev || prev.length !== curr.length) return -1;
  for (let i = 0; i < curr.length; i++) {
    if ((prev[i] ?? empty) === empty && (curr[i] ?? empty) !== empty) return i;
  }
  return -1;
}
