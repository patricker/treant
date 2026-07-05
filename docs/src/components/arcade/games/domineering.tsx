import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import styles from '../arcade.module.css';
import { useT } from '../i18n';

// Domineering & Cram: both use the DomineeringWasm engine, which emits legal
// moves as "from-to" cell-index pairs — the move carries BOTH covered cells, so
// the board never has to guess the domino's orientation. In standard
// Domineering each anchor has exactly one partner (player 0 places downward,
// player 1 rightward), so a tap places immediately. In Cram (impartial) an
// interior cell can host both a vertical and a horizontal domino, so tapping it
// enters a "pick where it points" second step.
function DomineeringBoard({ board, params, currentPlayer, interactive, legalMoves, lastCells, onMove }: BoardProps) {
  const { t } = useT();
  const { cols, rows } = params;
  const [hover, setHover] = useState<number | null>(null);
  const [sel, setSel] = useState<number | null>(null);
  const last = new Set(lastCells ?? []);

  // from-cell -> set of partner cells (one target in Domineering, up to two in Cram).
  const fromTo = new Map<number, Set<number>>();
  let hasVert = false;
  let hasHoriz = false;
  for (const m of legalMoves) {
    const dash = m.indexOf('-');
    if (dash < 0) continue;
    const f = Number(m.slice(0, dash));
    const to = Number(m.slice(dash + 1));
    if (Number.isNaN(f) || Number.isNaN(to)) continue;
    if (!fromTo.has(f)) fromTo.set(f, new Set());
    fromTo.get(f)!.add(to);
    if (to - f === cols) hasVert = true;
    else if (to - f === 1) hasHoriz = true;
  }

  // Clear a pending selection whenever the board changes (a move landed).
  useEffect(() => setSel(null), [board]);

  const selTargets = sel != null ? (fromTo.get(sel) ?? new Set<number>()) : new Set<number>();

  const tap = (i: number) => {
    if (!interactive) return;
    // Second step: complete a domino by tapping the chosen partner cell.
    if (sel != null && selTargets.has(i)) {
      onMove(`${sel}-${i}`);
      setSel(null);
      return;
    }
    const ts = fromTo.get(i);
    if (ts && ts.size > 0) {
      if (ts.size === 1) {
        onMove(`${i}-${[...ts][0]}`); // unambiguous — place at once
        setSel(null);
        return;
      }
      setSel(i === sel ? null : i); // two orientations — pick one next
      return;
    }
    setSel(null);
  };

  // Ghost preview: the selected anchor + its targets, else a hovered pair.
  const ghost = new Set<number>();
  if (sel != null) {
    ghost.add(sel);
    for (const tt of selTargets) ghost.add(tt);
  } else if (hover != null) {
    const ts = fromTo.get(hover);
    if (ts) {
      ghost.add(hover);
      for (const tt of ts) ghost.add(tt);
    }
  }

  const turnColor = currentPlayer === 0 ? 'var(--arc-p1)' : 'var(--arc-p2)';
  const hint =
    hasVert && hasHoriz
      ? t('↕↔ Your choice — place a domino either way')
      : hasHoriz
        ? t('↔ Horizontal — place a domino rightward')
        : hasVert
          ? t('↕ Vertical — place a domino downward')
          : '';

  return (
    <div>
      <div className={styles.domHint}>{hint}</div>
      <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))`, maxWidth: cols * 46 }}>
        {Array.from({ length: cols * rows }, (_, i) => {
          const ch = board[i] ?? ' ';
          const isFrom = interactive && fromTo.has(i);
          const isTarget = selTargets.has(i);
          const isSel = i === sel;
          const isGhost = ghost.has(i);
          const bg =
            ch === 'X'
              ? 'var(--arc-p1)'
              : ch === 'O'
                ? 'var(--arc-p2)'
                : isSel
                  ? `color-mix(in srgb, ${turnColor} 58%, transparent)`
                  : isGhost
                    ? `color-mix(in srgb, ${turnColor} 40%, transparent)`
                    : undefined;
          return (
            <button
              key={i}
              className={`${styles.tttCell} ${isFrom ? styles.domLegal : ''} ${last.has(i) ? styles.lastCell : ''}`}
              disabled={!interactive || !(isFrom || isTarget)}
              onClick={() => tap(i)}
              onMouseEnter={() => setHover(i)}
              onMouseLeave={() => setHover((h) => (h === i ? null : h))}
              style={{ background: bg, outline: isTarget ? `2px solid ${turnColor}` : undefined }}
              aria-label={t('Cell {n}: {state}', { n: i + 1, state: ch === ' ' ? t('empty') : ch })}
            />
          );
        })}
      </div>
    </div>
  );
}

export const domineering: GameDefinition = {
  id: 'domineering',
  name: 'Domineering',
  icon: '🁢',
  blurb: 'Place dominoes — you go vertical, the foe goes horizontal. Last to fit one in wins.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 300, topK: 3, temp: 0.6 },
  },
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Quick 5×5', emoji: '⚡', params: { numPlayers: 2, cols: 5, rows: 5 } },
    { label: 'Wide 8×8', emoji: '🔲', params: { numPlayers: 2, cols: 8, rows: 8 } },
    { label: 'Mega 10×10', emoji: '🤯', params: { numPlayers: 2, cols: 10, rows: 10 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 10, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.DomineeringWasm(p.cols, p.rows, 0)),
  Board: DomineeringBoard,
  playerLabels: ['Vertical', 'Horizontal'],
};

// Cram — the impartial twin of Domineering (same DomineeringWasm engine, cram
// flag on). Both players may place a domino either way; last to fit one wins.
export const cram: GameDefinition = {
  id: 'cram',
  name: 'Cram',
  icon: '🀫',
  blurb: 'Dominoes for two — but BOTH of you can place either way. Last to fit one wins.',
  // Calibrated on the Cram engine (scripts/calibrate.sh cram): its ladder is
  // monotonic/transitive, so these are the auto-selected rungs, not Domineering's.
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Quick 5×5', emoji: '⚡', params: { numPlayers: 2, cols: 5, rows: 5 } },
    { label: 'Wide 8×8', emoji: '🔲', params: { numPlayers: 2, cols: 8, rows: 8 } },
    { label: 'Mega 10×10', emoji: '🤯', params: { numPlayers: 2, cols: 10, rows: 10 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 10, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.DomineeringWasm(p.cols, p.rows, 1)),
  Board: DomineeringBoard,
  playerLabels: ['Player 1', 'Player 2'],
};
