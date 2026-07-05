import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// Select-then-move on the dark squares of a checkerboard. Fox (X) and Hounds
// (O) get distinct glyphs; the light/dark tint makes the diagonal-only movement
// legible. Legal targets come straight from the engine's "from-to" moves.
function FoxHoundsBoard({ board, params, interactive, legalMoves, winCells, lastCells, onMove }: BoardProps) {
  const { t } = useT();
  const { cols, rows } = params;
  const wins = new Set(winCells ?? []);
  const last = new Set(lastCells ?? []);
  const [sel, setSel] = useState<number | null>(null);
  useEffect(() => setSel(null), [board]); // drop stale selection after any move

  const fromTo = new Map<number, Set<number>>();
  for (const m of legalMoves) {
    const dash = m.indexOf('-');
    if (dash < 0) continue;
    const f = Number(m.slice(0, dash));
    const t = Number(m.slice(dash + 1));
    if (Number.isNaN(f) || Number.isNaN(t)) continue;
    if (!fromTo.has(f)) fromTo.set(f, new Set());
    fromTo.get(f)!.add(t);
  }
  const targets = sel != null ? (fromTo.get(sel) ?? new Set<number>()) : new Set<number>();

  const tap = (i: number) => {
    if (!interactive) return;
    if (sel != null && targets.has(i)) {
      onMove(`${sel}-${i}`);
      setSel(null);
      return;
    }
    setSel(fromTo.has(i) && i !== sel ? i : null);
  };

  return (
    <div>
      <div className={styles.shiftCaption}>{sel == null ? t('Tap a piece') : t('Tap where to move')}</div>
      <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {Array.from({ length: cols * rows }, (_, i) => {
          const ch = board[i] ?? ' ';
          const r = Math.floor(i / cols);
          const c = i % cols;
          const dark = (r + c) % 2 === 1;
          const isSel = i === sel;
          const isTarget = targets.has(i);
          return (
            <button
              key={i}
              className={`${styles.fhCell} ${dark ? styles.fhDark : ''} ${isSel ? styles.shiftSel : ''} ${isTarget ? styles.moveTarget : ''} ${wins.has(i) ? styles.winCell : ''} ${last.has(i) ? styles.lastCell : ''}`}
              disabled={!interactive || (!fromTo.has(i) && !isTarget)}
              onClick={() => tap(i)}
              aria-label={t('Cell {n}: {state}', {
                n: i + 1,
                state: ch === 'X' ? t('fox') : ch === 'O' ? t('hound') : t('empty'),
              })}
            >
              {ch === 'X' ? '🦊' : ch === 'O' ? '🐕' : isTarget ? '•' : ''}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const foxHounds: GameDefinition = {
  id: 'fox-hounds',
  name: 'Fox & Hounds',
  icon: '🦊',
  blurb: "You are the Fox, starting at the bottom. Reach the top row — the Hounds' home edge — to win. The four Hounds only move forward (down) and win by boxing you in so you can't move.",
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 8, rows: 8 },
  presets: [
    { label: 'Classic 8×8', emoji: '⭐', params: { numPlayers: 2, cols: 8, rows: 8 } },
    { label: 'Small 6×6', emoji: '🔳', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Hound Wall 12×8', emoji: '🐕', params: { numPlayers: 2, cols: 12, rows: 8 } },
    { label: 'Long Chase 8×12', emoji: '🏃', params: { numPlayers: 2, cols: 8, rows: 12 } },
    { label: 'Thunderdome 12×12', emoji: '🤯', params: { numPlayers: 2, cols: 12, rows: 12 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 6, max: 12, step: 1 },
    { key: 'rows', label: 'Height', min: 6, max: 12, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.FoxHoundsWasm(p.cols, p.rows)),
  Board: FoxHoundsBoard,
  playerLabels: ['Fox', 'Hounds'],
};
