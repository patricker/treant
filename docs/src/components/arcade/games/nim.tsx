import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.NimWasm(p.stones);
  const seat = () => (g.current_player() === 'P1' ? 0 : 1);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => String(g.current_stones()),
    currentPlayer: seat,
    isTerminal: () => g.current_stones() === 0,
    // Normal play: whoever takes the last stone wins. At terminal the current
    // player is the one who cannot move (the loser) -> winner is the other seat.
    result: () => (g.current_stones() === 0 ? String(seat() === 0 ? 2 : 1) : ''),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const s = g.current_stones();
      return s >= 2 ? ['Take1', 'Take2'] : s === 1 ? ['Take1'] : [];
    },
    free: () => g.free(),
  };
}

function NimBoard({ board, interactive, onMove }: BoardProps) {
  const stones = Number(board) || 0;
  return (
    <div className={styles.nimWrap}>
      <div className={styles.nimPile}>
        {Array.from({ length: stones }, (_, i) => (
          <span key={i} className={styles.nimStone} />
        ))}
      </div>
      <div className={styles.nimCount}>{stones} stones left</div>
      <div className={styles.nimButtons}>
        <button
          className={styles.nimTake}
          disabled={!interactive || stones < 1}
          onClick={() => onMove('Take1')}
        >
          Take 1
        </button>
        <button
          className={styles.nimTake}
          disabled={!interactive || stones < 2}
          onClick={() => onMove('Take2')}
        >
          Take 2
        </button>
      </div>
    </div>
  );
}

export const nim: GameDefinition = {
  id: 'nim',
  name: 'Nim',
  icon: '🪨',
  blurb: 'Take 1 or 2 stones. Take the last one to win.',
  defaultParams: { numPlayers: 2, stones: 15 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, stones: 15 } },
    { label: 'Quick', emoji: '⚡', params: { numPlayers: 2, stones: 7 } },
    { label: 'Marathon', emoji: '🏃', params: { numPlayers: 2, stones: 25 } },
    { label: 'Epic 30', emoji: '🤯', params: { numPlayers: 2, stones: 30 } },
  ],
  knobs: [{ key: 'stones', label: 'Stones', min: 3, max: 30, step: 1 }],
  create: makeHandle,
  Board: NimBoard,
};
