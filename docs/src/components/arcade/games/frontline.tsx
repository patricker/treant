import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle } from '../gameTypes';
import styles from '../arcade.module.css';

const COLOR: Record<string, string> = {
  X: 'var(--arc-p1)',
  O: 'var(--arc-p2)',
  A: 'var(--arc-p3)',
  B: 'var(--arc-p4)',
};

// Reusable handle for movement games whose WASM exposes `legal_moves()` as
// comma-separated "from-to" strings.
export function moveHandle(g: any): GameHandle {
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const s = g.legal_moves();
      return s ? s.split(',') : [];
    },
    free: () => g.free(),
  };
}

// Reusable select-then-move board. Uses the engine's legal moves (passed in via
// BoardProps) to know which pieces can move and where — no duplicated move-gen.
export function MoveBoard({ board, params, interactive, legalMoves, onMove }: BoardProps) {
  const { cols, rows } = params;
  const [sel, setSel] = useState<number | null>(null);
  // Clear any selection once the board actually changes (a move landed, or the
  // opponent/AI moved) so a stale highlight never points at the wrong cell.
  useEffect(() => setSel(null), [board]);
  const cells = Array.from({ length: cols * rows }, (_, i) => board[i] ?? ' ');

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
      <div className={styles.shiftCaption}>{sel == null ? 'Tap a piece' : 'Tap where to move'}</div>
      <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
        {cells.map((ch, i) => {
          const isSel = i === sel;
          const isTarget = targets.has(i);
          return (
            <button
              key={i}
              className={`${styles.tttCell} ${isSel ? styles.shiftSel : ''} ${isTarget ? styles.moveTarget : ''}`}
              disabled={!interactive || (!fromTo.has(i) && !isTarget)}
              onClick={() => tap(i)}
              style={{ color: COLOR[ch], fontSize: cols > 7 ? '0.85rem' : undefined }}
              aria-label={`Cell ${i + 1}: ${ch === ' ' ? 'empty' : ch}`}
            >
              {ch === ' ' ? (isTarget ? '•' : '') : ch}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const frontline: GameDefinition = {
  id: 'frontline',
  name: 'Frontline',
  icon: '⚔️',
  blurb: 'March pawns across the board. Capture diagonally. Reach the far row to win.',
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Big 8×8', emoji: '🔲', params: { numPlayers: 2, cols: 8, rows: 8 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 10, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.FrontlineWasm(p.cols, p.rows)),
  Board: MoveBoard,
  playerLabels: ['X', 'O'],
};
