import type { GameDefinition, GameHandle, GameParams, BoardProps } from '../gameTypes';
import styles from '../arcade.module.css';
import { usePrevBoard, changedIndex } from '../boardDiff';
import { useT } from '../i18n';

const DISC = ['', 'var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)', 'var(--arc-p5)', 'var(--arc-p6)'];

function makeHandle(wasm: any, p: GameParams, variant: number): GameHandle {
  const g = new wasm.ConnectFourWasm(p.cols, p.rows, p.k, p.numPlayers, variant);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    // The engine lists both drop columns ("3") and — in Pop Out — pop moves
    // ("pop3", legal only when that column's bottom disc is the mover's).
    legalMoves: () => {
      const s = g.legal_moves();
      return s ? s.split(',') : [];
    },
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    winningCells: () => {
      const s = g.winning_cells();
      return s ? s.split(',') : [];
    },
    free: () => g.free(),
  };
}

function ConnectFourBoard({ board, params, currentPlayer, interactive, legalMoves, winCells, lastCells, onMove }: BoardProps) {
  const { t } = useT();
  const { cols, rows } = params;
  const cells = Array.from({ length: cols * rows }, (_, i) => board[i] ?? ' ');
  const dropped = changedIndex(usePrevBoard(board), board);
  const win = new Set(winCells ?? []);
  const last = new Set(lastCells ?? []);
  const legalCols = new Set<number>();
  for (let c = 0; c < cols; c++) if ((board[c] ?? ' ') === ' ') legalCols.add(c);
  // Pop Out affordance: the engine lists "pop<c>" for columns whose bottom disc
  // belongs to the mover. Show the ⤵ row only when a pop is actually available.
  const popCols = new Set<number>();
  for (const mv of legalMoves ?? []) if (mv.startsWith('pop')) popCols.add(Number(mv.slice(3)));
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
            <div key={i} className={`${styles.cfCell} ${last.has(i) ? styles.lastCell : ''}`}>
              <span
                className={`${styles.cfDisc} ${i === dropped ? styles.dropIn : ''} ${win.has(i) ? styles.winCell : ''}`}
                style={{ background: p ? DISC[p] : 'transparent' }}
              />
            </div>
          );
        })}
      </div>
      {popCols.size > 0 && (
        <div className={styles.cfPopRow} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
          {Array.from({ length: cols }, (_, c) => (
            <button
              key={c}
              className={styles.cfColButton}
              disabled={!interactive || !popCols.has(c)}
              onClick={() => onMove(`pop${c}`)}
              aria-label={t('Pop the bottom disc from column {n}', { n: c + 1 })}
              style={{ color: DISC[currentPlayer + 1] }}
            >
              ⤵
            </button>
          ))}
        </div>
      )}
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
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { cols: 7, rows: 6, k: 4, numPlayers: 2 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { cols: 7, rows: 6, k: 4, numPlayers: 2 } },
    { label: 'Connect-5', emoji: '🖐️', params: { cols: 9, rows: 7, k: 5, numPlayers: 2 } },
    { label: 'Chaos Two', emoji: '⚡', params: { cols: 7, rows: 6, k: 2, numPlayers: 2 } },
    { label: '4-Player Frenzy', emoji: '🎉', params: { cols: 9, rows: 8, k: 4, numPlayers: 4 } },
    { label: 'Giant', emoji: '🦣', params: { cols: 12, rows: 12, k: 5, numPlayers: 2 } },
    { label: '6-Player Mayhem', emoji: '🤯', params: { cols: 12, rows: 12, k: 4, numPlayers: 6 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 3, max: 12, step: 1 },
    { key: 'rows', label: 'Height', min: 3, max: 12, step: 1 },
    { key: 'k', label: 'In a row', min: 2, max: 12, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
  ],
  create: (wasm, p) => makeHandle(wasm, p, 0),
  Board: ConnectFourBoard,
  moveSound: 'drop',
};

export const popOut: GameDefinition = {
  id: 'pop-out',
  variantOf: 'connect-four',
  name: 'Pop Out',
  icon: '⤵️',
  blurb: 'Connect Four where discs can leave: pop your own bottom disc and watch the column fall.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 300, topK: 3, temp: 0.6 },
  },
  defaultParams: { cols: 7, rows: 6, k: 4, numPlayers: 2 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { cols: 7, rows: 6, k: 4, numPlayers: 2 } },
    { label: 'Connect-5', emoji: '🖐️', params: { cols: 9, rows: 7, k: 5, numPlayers: 2 } },
    { label: '4-Player Frenzy', emoji: '🎉', params: { cols: 9, rows: 8, k: 4, numPlayers: 4 } },
    { label: 'Giant', emoji: '🦣', params: { cols: 12, rows: 12, k: 5, numPlayers: 2 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 3, max: 12, step: 1 },
    { key: 'rows', label: 'Height', min: 3, max: 12, step: 1 },
    { key: 'k', label: 'In a row', min: 2, max: 12, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
  ],
  create: (wasm, p) => makeHandle(wasm, p, 1),
  Board: ConnectFourBoard,
  moveSound: 'drop',
};

export const cylinderFour: GameDefinition = {
  id: 'cylinder-four',
  variantOf: 'connect-four',
  name: 'Cylinder Four',
  icon: '🛢️',
  blurb: 'Connect Four on a tube — lines wrap around the edges.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  defaultParams: { cols: 12, rows: 6, k: 4, numPlayers: 2 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { cols: 12, rows: 6, k: 4, numPlayers: 2 } },
    { label: 'Connect-5', emoji: '🖐️', params: { cols: 12, rows: 7, k: 5, numPlayers: 2 } },
    { label: '4-Player Frenzy', emoji: '🎉', params: { cols: 12, rows: 8, k: 4, numPlayers: 4 } },
    { label: '6-Player Mayhem', emoji: '🤯', params: { cols: 12, rows: 12, k: 4, numPlayers: 6 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 3, max: 12, step: 1 },
    { key: 'rows', label: 'Height', min: 3, max: 12, step: 1 },
    { key: 'k', label: 'In a row', min: 2, max: 12, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
  ],
  create: (wasm, p) => makeHandle(wasm, p, 2),
  Board: ConnectFourBoard,
  moveSound: 'drop',
};
