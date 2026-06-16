import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.EuclidWasm(p.a, p.b);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => (g.is_terminal() ? String(g.current_player() === 0 ? 2 : 1) : ''),
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

function EuclidBoard({ board, interactive, legalMoves, onMove }: BoardProps) {
  const [a, b] = board.split(',').map(Number);
  const hi = Math.max(a, b);
  const targets = legalMoves.map(Number).filter((n) => !Number.isNaN(n)).sort((x, y) => y - x);
  return (
    <div className={styles.euclidWrap}>
      <div className={styles.euclidNums}>
        <span className={`${styles.euclidTile} ${a >= b ? styles.euclidBig : ''}`}>{a}</span>
        <span className={styles.euclidTile + ' ' + (b > a ? styles.euclidBig : '')}>{b}</span>
      </div>
      <div className={styles.nimCount}>Reduce the larger number ({hi}) toward zero</div>
      <div className={styles.euclidButtons}>
        {targets.map((t) => (
          <button key={t} className={styles.nimTake} disabled={!interactive} onClick={() => onMove(String(t))}>
            {hi} → {t}
          </button>
        ))}
      </div>
    </div>
  );
}

export const euclid: GameDefinition = {
  id: 'euclid',
  name: "Euclid's Game",
  icon: '➗',
  blurb: 'Two numbers. Subtract a multiple of the smaller from the larger. Make a zero to win.',
  defaultParams: { numPlayers: 2, a: 25, b: 16 },
  presets: [
    { label: 'Classic 25 & 16', emoji: '⭐', params: { numPlayers: 2, a: 25, b: 16 } },
    { label: 'Quick 12 & 7', emoji: '⚡', params: { numPlayers: 2, a: 12, b: 7 } },
    { label: 'Big 40 & 9', emoji: '🔲', params: { numPlayers: 2, a: 40, b: 9 } },
    { label: 'Tangle 40 & 37', emoji: '🤯', params: { numPlayers: 2, a: 40, b: 37 } },
  ],
  knobs: [
    { key: 'a', label: 'First number', min: 4, max: 40, step: 1 },
    { key: 'b', label: 'Second number', min: 4, max: 40, step: 1 },
  ],
  create: makeHandle,
  Board: EuclidBoard,
};
