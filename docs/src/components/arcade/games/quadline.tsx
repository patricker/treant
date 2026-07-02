import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';
import { usePrevBoard, changedIndex } from '../boardDiff';
import { useT } from '../i18n';

// Two seats only: Red (X) and Yellow (O).
const SYM = ['X', 'O'];
const SEAT_COLOR: Record<string, string> = {
  X: 'var(--arc-p1)',
  O: 'var(--arc-p2)',
};
const PIECES = 4; // fixed by the rules — drop four, then slide

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.QuadlineWasm(p.size, p.diagonals);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const s: string = g.legal_moves();
      return s.length ? s.split(',') : [];
    },
    weakMove: (pl, k, t, s) => g.weak_move(pl, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

function QuadlineBoard({ board, params, currentPlayer, interactive, legalMoves, onMove }: BoardProps) {
  const size: number = params.size;
  const { t } = useT();
  const me = SYM[currentPlayer];
  const [sel, setSel] = useState<number | null>(null);
  useEffect(() => setSel(null), [board]); // drop stale selection after any move
  const arrived = changedIndex(usePrevBoard(board), board);
  const cells = Array.from({ length: size * size }, (_, i) => board[i] ?? ' ');
  // Placement phase for me = I still have fewer than my four pieces down.
  const myCount = cells.filter((c) => c === me).length;
  const placement = myCount < PIECES;

  // In the move phase, the legal slide targets for the picked-up piece are the
  // engine's own moves that start at `sel` — no need to recompute adjacency.
  const targets = new Set<number>();
  if (!placement && sel !== null) {
    for (const mv of legalMoves) {
      const [from, to] = mv.split('-');
      if (Number(from) === sel) targets.add(Number(to));
    }
  }

  const tap = (i: number) => {
    if (!interactive) return;
    const ch = cells[i];
    if (placement) {
      if (ch === ' ') onMove('d' + i);
      return;
    }
    if (sel === null) {
      if (ch === me) setSel(i);
    } else if (i === sel) {
      setSel(null);
    } else if (ch === me) {
      setSel(i);
    } else if (targets.has(i)) {
      onMove(`${sel}-${i}`);
      setSel(null);
    }
  };

  return (
    <div>
      <div className={styles.shiftCaption}>
        {placement
          ? t('Place your pieces')
          : sel === null
            ? t('Tap a piece to pick it up')
            : t('Tap an adjacent square')}
      </div>
      <div className={styles.shiftGrid} style={{ gridTemplateColumns: `repeat(${size}, minmax(0, 1fr))` }}>
        {cells.map((ch, i) => (
          <button
            key={i}
            className={`${styles.shiftCell} ${sel === i ? styles.shiftSel : ''} ${targets.has(i) ? styles.moveTarget : ''} ${i === arrived ? styles.popIn : ''}`}
            disabled={!interactive}
            onClick={() => tap(i)}
            style={{ color: ch === ' ' ? 'var(--ifm-color-emphasis-300)' : SEAT_COLOR[ch] }}
            aria-label={t('Cell {n}: {state}', { n: i + 1, state: ch === ' ' ? t('empty') : ch })}
          >
            {ch === ' ' ? '·' : ch}
          </button>
        ))}
      </div>
    </div>
  );
}

export const quadline: GameDefinition = {
  id: 'quadline',
  name: 'Quadline',
  icon: '🔶',
  blurb: 'Drop four, then slide — line them up or square them off.',
  difficulty: {
    easy: { playouts: 15, topK: 6, temp: 2.5 },
    medium: { playouts: 400, topK: 3, temp: 0.6 },
    hard: { playouts: 1500, topK: 1, temp: 0.2 },
  },
  defaultParams: { numPlayers: 2, size: 5, diagonals: 1 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, size: 5, diagonals: 1 } },
    { label: 'Tight', emoji: '🔳', params: { numPlayers: 2, size: 4, diagonals: 1 } },
    { label: 'Roomy', emoji: '🔲', params: { numPlayers: 2, size: 6, diagonals: 1 } },
    { label: 'Huge', emoji: '🤯', params: { numPlayers: 2, size: 7, diagonals: 1 } },
    { label: 'No Diagonals', emoji: '📐', params: { numPlayers: 2, size: 5, diagonals: 0 } },
  ],
  knobs: [
    { key: 'size', label: 'Board size', min: 4, max: 7, step: 1 },
    { key: 'diagonals', label: 'Diagonals', min: 0, max: 1, step: 1 },
  ],
  create: makeHandle,
  Board: QuadlineBoard,
  playerLabels: ['Red', 'Yellow'],
};
