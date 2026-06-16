import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.SubtractSquareWasm(p.stones);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => String(g.current_stones()),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    // Normal play: taking the last stone wins. At terminal the player to move
    // is the one who cannot take (the loser) -> the other seat won.
    result: () => (g.is_terminal() ? String(g.current_player() === 0 ? 2 : 1) : ''),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const s = g.legal_moves();
      return s ? s.split(',') : [];
    },
    free: () => g.free(),
  };
}

function SubtractSquareBoard({ board, interactive, legalMoves, onMove }: BoardProps) {
  const stones = Number(board) || 0;
  const squares = legalMoves.map(Number).filter((n) => !Number.isNaN(n)).sort((a, b) => a - b);
  return (
    <div className={styles.nimWrap}>
      <div className={styles.nimPile}>
        {Array.from({ length: stones }, (_, i) => (
          <span key={i} className={styles.nimStone} />
        ))}
      </div>
      <div className={styles.nimCount}>{stones} stones left</div>
      <div className={styles.nimButtons}>
        {squares.map((k) => (
          <button
            key={k}
            className={styles.nimTake}
            disabled={!interactive}
            onClick={() => onMove(String(k))}
          >
            Take {k}
          </button>
        ))}
      </div>
    </div>
  );
}

export const subtractSquare: GameDefinition = {
  id: 'subtract-square',
  name: 'Square Subtract',
  icon: '🟦',
  blurb: 'Remove a square number of stones — 1, 4, 9, 16… Take the last stone to win.',
  defaultParams: { numPlayers: 2, stones: 20 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, stones: 20 } },
    { label: 'Quick', emoji: '⚡', params: { numPlayers: 2, stones: 12 } },
    { label: 'Marathon', emoji: '🏃', params: { numPlayers: 2, stones: 35 } },
  ],
  knobs: [{ key: 'stones', label: 'Stones', min: 5, max: 40, step: 1 }],
  create: makeHandle,
  Board: SubtractSquareBoard,
};
