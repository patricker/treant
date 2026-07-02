import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';
import { usePrevBoard, changedIndex } from '../boardDiff';
import { useT } from '../i18n';

const SYM = ['X', 'O', 'A', 'B', 'C', 'D'];
const SEAT_COLOR: Record<string, string> = {
  X: 'var(--arc-p1)',
  O: 'var(--arc-p2)',
  A: 'var(--arc-p3)',
  B: 'var(--arc-p4)',
  C: 'var(--arc-p5)',
  D: 'var(--arc-p6)',
};

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.ShiftWasm(p.cols, p.rows, p.k, p.numPlayers, p.pieces);
  const legalMoves = () => {
    const board = g.get_board();
    const n = p.cols * p.rows;
    const empties: number[] = [];
    for (let i = 0; i < n; i++) if ((board[i] ?? ' ') === ' ') empties.push(i);
    if (g.in_placement_phase()) return empties.map((i) => 'P' + i);
    const me = SYM[g.current_player()];
    const out: string[] = [];
    for (let f = 0; f < n; f++) {
      if (board[f] !== me) continue;
      for (const t of empties) out.push(`M${f},${t}`);
    }
    return out;
  };
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves,
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

function ShiftBoard({ board, params, currentPlayer, interactive, onMove }: BoardProps) {
  const { cols, rows } = params;
  const { t } = useT();
  const me = SYM[currentPlayer];
  const [sel, setSel] = useState<number | null>(null);
  useEffect(() => setSel(null), [board]); // drop stale selection after any move
  const arrived = changedIndex(usePrevBoard(board), board);
  const cells = Array.from({ length: cols * rows }, (_, i) => board[i] ?? ' ');
  // The engine's placement phase is per-player; "can I still place?" is exactly
  // "I have fewer than my piece quota on the board".
  const myCount = cells.filter((c) => c === me).length;
  const placement = myCount < params.pieces;

  const tap = (i: number) => {
    if (!interactive) return;
    const ch = cells[i];
    if (placement) {
      if (ch === ' ') onMove('P' + i);
      return;
    }
    if (sel === null) {
      if (ch === me) setSel(i);
    } else if (i === sel) {
      setSel(null);
    } else if (ch === ' ') {
      onMove(`M${sel},${i}`);
      setSel(null);
    } else if (ch === me) {
      setSel(i);
    }
  };

  return (
    <div>
      <div className={styles.shiftCaption}>
        {placement
          ? t('Place your pieces')
          : sel === null
            ? t('Tap a piece to pick it up')
            : t('Tap an empty square to slide')}
      </div>
      <div className={styles.shiftGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {cells.map((ch, i) => (
          <button
            key={i}
            className={`${styles.shiftCell} ${sel === i ? styles.shiftSel : ''} ${i === arrived ? styles.popIn : ''}`}
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

export const shift: GameDefinition = {
  id: 'shift',
  name: 'Shift',
  icon: '🔀',
  blurb: 'Place your pieces, then slide them. Line them up to win.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  defaultParams: { numPlayers: 2, cols: 3, rows: 3, k: 3, pieces: 3 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, cols: 3, rows: 3, k: 3, pieces: 3 } },
    { label: 'Big', emoji: '🔲', params: { numPlayers: 2, cols: 5, rows: 4, k: 4, pieces: 4 } },
    { label: '3-Player', emoji: '👨‍👩‍👦', params: { numPlayers: 3, cols: 4, rows: 4, k: 3, pieces: 2 } },
    { label: '6-Player Scrum', emoji: '🤯', params: { numPlayers: 6, cols: 6, rows: 6, k: 3, pieces: 2 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 2, max: 8, step: 1 },
    { key: 'rows', label: 'Height', min: 2, max: 8, step: 1 },
    { key: 'k', label: 'In a row', min: 2, max: 6, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
    { key: 'pieces', label: 'Pieces', min: 1, max: 4, step: 1 },
  ],
  create: makeHandle,
  Board: ShiftBoard,
};
