import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';

const SEAT_COLOR = ['var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)', 'var(--arc-p5)', 'var(--arc-p6)'];

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.MancalaWasm(p.pits, p.stones, p.numPlayers);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    // WASM returns "P{N}" / "Draw" / "" — normalize the winner to a bare number.
    result: () => {
      const r = g.result();
      return r.startsWith('P') ? r.slice(1) : r;
    },
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const s = g.legal_moves();
      return s ? s.split(',') : [];
    },
    free: () => g.free(),
  };
}

// Ring layout (per WASM get_board doc): P0 pit0..pits-1, P0 store, P1 pits, P1 store, …
function MancalaBoard({ board, params, currentPlayer, interactive, onMove }: BoardProps) {
  const pits = params.pits;
  const np = params.numPlayers;
  const counts = board.split(',').map(Number);
  const stride = pits + 1;
  const pitVal = (pl: number, i: number) => counts[pl * stride + i] ?? 0;
  const storeVal = (pl: number) => counts[pl * stride + pits] ?? 0;
  return (
    <div className={styles.mancalaWrap}>
      {Array.from({ length: np }, (_, pl) => {
        const mine = pl === currentPlayer;
        return (
          <div
            key={pl}
            className={`${styles.mancalaRow} ${mine ? styles.mancalaRowActive : ''}`}
            style={{ borderColor: SEAT_COLOR[pl] }}
          >
            <div className={styles.mancalaStore} style={{ background: SEAT_COLOR[pl] }}>
              {storeVal(pl)}
            </div>
            <div className={styles.mancalaPits}>
              {Array.from({ length: pits }, (_, i) => {
                const v = pitVal(pl, i);
                return (
                  <button
                    key={i}
                    className={styles.mancalaPit}
                    disabled={!interactive || !mine || v === 0}
                    onClick={() => onMove(String(i))}
                    style={{ borderColor: SEAT_COLOR[pl] }}
                    aria-label={`Player ${pl + 1} pit ${i + 1}: ${v} stones`}
                  >
                    {v}
                  </button>
                );
              })}
            </div>
            <div className={styles.mancalaLabel}>P{pl + 1}</div>
          </div>
        );
      })}
    </div>
  );
}

export const mancala: GameDefinition = {
  id: 'mancala',
  name: 'Mancala',
  icon: '🫘',
  blurb: 'Sow seeds, capture, and fill your store. Kalah rules.',
  defaultParams: { numPlayers: 2, pits: 6, stones: 4 },
  presets: [
    { label: 'Kalah', emoji: '⭐', params: { numPlayers: 2, pits: 6, stones: 4 } },
    { label: 'Quick', emoji: '⚡', params: { numPlayers: 2, pits: 3, stones: 3 } },
    { label: '4-Player Ring', emoji: '🎉', params: { numPlayers: 4, pits: 4, stones: 3 } },
    { label: '6-Player Sow', emoji: '🤯', params: { numPlayers: 6, pits: 6, stones: 4 } },
  ],
  knobs: [
    { key: 'pits', label: 'Pits', min: 3, max: 8, step: 1 },
    { key: 'stones', label: 'Stones', min: 2, max: 6, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
  ],
  create: makeHandle,
  Board: MancalaBoard,
};
