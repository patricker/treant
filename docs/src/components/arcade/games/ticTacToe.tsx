import type { GameDefinition, GameHandle, GameParams, BoardProps } from '../gameTypes';
import styles from '../arcade.module.css';
import { usePrevBoard, changedIndex } from '../boardDiff';
import { useT } from '../i18n';

const COLOR: Record<string, string> = {
  X: 'var(--arc-p1)',
  O: 'var(--arc-p2)',
  A: 'var(--arc-p3)',
  B: 'var(--arc-p4)',
  C: 'var(--arc-p5)',
  D: 'var(--arc-p6)',
};

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.TicTacToeWasm(p.cols, p.rows, p.k, p.numPlayers);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const board = g.get_board();
      const out: string[] = [];
      for (let i = 0; i < p.cols * p.rows; i++) if ((board[i] ?? ' ') === ' ') out.push(String(i));
      return out;
    },
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

function TicTacToeBoard({ board, params, interactive, onMove }: BoardProps) {
  const { cols, rows } = params;
  const { t } = useT();
  const placed = changedIndex(usePrevBoard(board), board);
  return (
    <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
      {Array.from({ length: cols * rows }, (_, i) => {
        const ch = board[i] ?? ' ';
        return (
          <button
            key={i}
            className={`${styles.tttCell} ${i === placed ? styles.popIn : ''}`}
            disabled={!interactive || ch !== ' '}
            onClick={() => onMove(String(i))}
            style={{ color: COLOR[ch] }}
            aria-label={t('Cell {n}: {state}', { n: i + 1, state: ch === ' ' ? t('empty') : ch })}
          >
            {ch === ' ' ? '' : ch}
          </button>
        );
      })}
    </div>
  );
}

export const ticTacToe: GameDefinition = {
  id: 'tic-tac-toe',
  name: 'Tic-Tac-Toe',
  icon: '⭕',
  blurb: 'Classic 3×3 — or a giant 5-in-a-row brain-bender.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 100, topK: 4, temp: 1 },
  },
  defaultParams: { cols: 3, rows: 3, k: 3, numPlayers: 2 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { cols: 3, rows: 3, k: 3, numPlayers: 2 } },
    { label: 'Gomoku-lite', emoji: '🧠', params: { cols: 9, rows: 9, k: 5, numPlayers: 2 } },
    { label: 'Big Board', emoji: '🔲', params: { cols: 6, rows: 6, k: 4, numPlayers: 2 } },
    { label: '3-Player', emoji: '👨‍👩‍👦', params: { cols: 6, rows: 6, k: 4, numPlayers: 3 } },
    { label: '6-Player Chaos', emoji: '🤯', params: { cols: 10, rows: 10, k: 4, numPlayers: 6 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 2, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 2, max: 10, step: 1 },
    { key: 'k', label: 'In a row', min: 2, max: 10, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
  ],
  create: makeHandle,
  Board: TicTacToeBoard,
};
