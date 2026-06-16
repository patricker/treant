import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle } from '../gameTypes';
import styles from '../arcade.module.css';

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
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

// A rendered game piece: a chess-style pawn or a round stone, coloured per
// player. Sized to sit nicely inside a grid cell.
export function Piece({ ch, kind }: { ch: string; kind: 'pawn' | 'disc' }) {
  const color = ch === 'X' ? 'var(--arc-p1)' : 'var(--arc-p2)';
  if (kind === 'pawn') {
    return (
      <svg viewBox="0 0 24 24" width="72%" height="72%" style={{ display: 'block' }} aria-hidden>
        <g fill={color} stroke="rgba(0,0,0,0.28)" strokeWidth="0.7" strokeLinejoin="round">
          <circle cx="12" cy="6.4" r="3.1" />
          <path d="M9.2 10.6h5.6l-1 3.2h-3.6z" />
          <path d="M7.6 21c0-3.4 2-4.6 2.9-6.4h3c0.9 1.8 2.9 3 2.9 6.4z" />
          <rect x="6" y="20" width="12" height="2.6" rx="1.3" />
        </g>
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 24 24" width="74%" height="74%" style={{ display: 'block' }} aria-hidden>
      <circle cx="12" cy="12" r="8.6" fill={color} stroke="rgba(0,0,0,0.22)" strokeWidth="0.8" />
    </svg>
  );
}

// Reusable select-then-move board. Uses the engine's legal moves (passed in via
// BoardProps) to know which pieces can move and where — no duplicated move-gen.
// `pieceKind` picks how occupied cells are drawn (stones by default, pawns for
// the movement/race games).
export function makeMoveBoard(pieceKind: 'pawn' | 'disc') {
  return function MoveBoardInner({ board, params, interactive, legalMoves, onMove }: BoardProps) {
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
                aria-label={`Cell ${i + 1}: ${ch === ' ' ? 'empty' : ch}`}
              >
                {ch === 'X' || ch === 'O' ? <Piece ch={ch} kind={pieceKind} /> : isTarget ? <span className={styles.moveDot} /> : ''}
              </button>
            );
          })}
        </div>
      </div>
    );
  };
}

// Stones by default (Kōnane, etc.); Frontline overrides to pawns.
export const MoveBoard = makeMoveBoard('disc');

export const frontline: GameDefinition = {
  id: 'frontline',
  name: 'Frontline',
  icon: '⚔️',
  blurb: 'March pawns across the board. Capture diagonally. Reach the far row to win.',
  rules:
    'Each turn, move one of your pawns one square straight forward into an empty cell, or one square diagonally forward to capture an enemy pawn sitting there. You win the instant one of your pawns reaches the far row, or if you capture all of the opponent’s pawns. There are no draws — someone always breaks through.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3.0 },
    medium: { playouts: 100, topK: 4, temp: 1.0 },
    hard: { playouts: 4000, topK: 1, temp: 0.0 },
  },
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Big 8×8', emoji: '🔲', params: { numPlayers: 2, cols: 8, rows: 8 } },
    { label: 'Mega 10×10', emoji: '🤯', params: { numPlayers: 2, cols: 10, rows: 10 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 10, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.FrontlineWasm(p.cols, p.rows)),
  Board: makeMoveBoard('pawn'),
  playerLabels: ['Red', 'Gold'],
};
