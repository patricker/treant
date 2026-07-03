import type { GameDefinition, GameHandle, GameParams, BoardProps } from '../gameTypes';
import styles from '../arcade.module.css';
import { usePrevBoard, changedIndex } from '../boardDiff';
import { useT } from '../i18n';

const DISC = ['', 'var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)', 'var(--arc-p5)', 'var(--arc-p6)'];

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.ConnectFourWasm(p.cols, p.rows, p.k, p.numPlayers);
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
      for (let c = 0; c < p.cols; c++) if ((board[c] ?? ' ') === ' ') out.push(String(c));
      return out;
    },
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    winningCells: () => {
      const s = g.winning_cells();
      return s ? s.split(',') : [];
    },
    free: () => g.free(),
  };
}

function ConnectFourBoard({ board, params, currentPlayer, interactive, winCells, onMove }: BoardProps) {
  const { t } = useT();
  const { cols, rows } = params;
  const cells = Array.from({ length: cols * rows }, (_, i) => board[i] ?? ' ');
  const dropped = changedIndex(usePrevBoard(board), board);
  const win = new Set(winCells ?? []);
  const legalCols = new Set<number>();
  for (let c = 0; c < cols; c++) if ((board[c] ?? ' ') === ' ') legalCols.add(c);
  return (
    <div className={styles.cfBoard}>
      <div className={styles.cfColHeaders} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {Array.from({ length: cols }, (_, c) => (
          <button
            key={c}
            className={styles.cfColButton}
            disabled={!interactive || !legalCols.has(c)}
            onClick={() => onMove(String(c))}
            aria-label={t('Drop in column {n}', { n: c + 1 })}
            // The arrow previews whose disc will drop — not a fixed accent.
            style={{ color: DISC[currentPlayer + 1] }}
          >
            ▾
          </button>
        ))}
      </div>
      <div className={styles.cfGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {cells.map((ch, i) => {
          const p = ch === ' ' ? 0 : Number(ch);
          return (
            <div key={i} className={styles.cfCell}>
              <span
                className={`${styles.cfDisc} ${i === dropped ? styles.dropIn : ''} ${win.has(i) ? styles.winCell : ''}`}
                style={{ background: p ? DISC[p] : 'transparent' }}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}

export const connectFour: GameDefinition = {
  id: 'connect-four',
  name: 'Connect Four',
  icon: '🔴',
  blurb: 'Drop discs, line up four. Or seven. With up to six players.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 300, topK: 3, temp: 0.6 },
  },
  defaultParams: { cols: 7, rows: 6, k: 4, numPlayers: 2 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { cols: 7, rows: 6, k: 4, numPlayers: 2 } },
    { label: 'Connect-5', emoji: '🖐️', params: { cols: 9, rows: 7, k: 5, numPlayers: 2 } },
    { label: '4-Player Frenzy', emoji: '🎉', params: { cols: 9, rows: 8, k: 4, numPlayers: 4 } },
    { label: 'Giant', emoji: '🦣', params: { cols: 10, rows: 10, k: 5, numPlayers: 2 } },
    { label: '6-Player Mayhem', emoji: '🤯', params: { cols: 10, rows: 10, k: 4, numPlayers: 6 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 3, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 3, max: 10, step: 1 },
    { key: 'k', label: 'In a row', min: 3, max: 10, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
  ],
  create: makeHandle,
  Board: ConnectFourBoard,
  moveSound: 'drop',
};
