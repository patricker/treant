import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

const SEAT_COLOR = ['var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)', 'var(--arc-p5)', 'var(--arc-p6)'];

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.MancalaWasm(p.pits, p.stones, p.numPlayers);
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

// Ring layout (per WASM get_board doc): P0 pit0..pits-1, P0 store, P1 pits, P1 store, …
function MancalaBoard({ board, params, currentPlayer, interactive, onMove }: BoardProps) {
  const { t } = useT();
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
                    aria-label={t('Player {p} pit {i}: {v} stones', { p: pl + 1, i: i + 1, v })}
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
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, pits: 6, stones: 4 },
  presets: [
    { label: 'Kalah', emoji: '⭐', params: { numPlayers: 2, pits: 6, stones: 4 } },
    { label: 'Quick', emoji: '⚡', params: { numPlayers: 2, pits: 3, stones: 3 } },
    { label: '4-Player Ring', emoji: '🎉', params: { numPlayers: 4, pits: 4, stones: 3 } },
    { label: 'Big 4-Player', emoji: '🤯', params: { numPlayers: 4, pits: 8, stones: 6 } },
    { label: 'Micro', emoji: '🤏', params: { numPlayers: 2, pits: 2, stones: 1 } },
    { label: 'Overflow', emoji: '🤯', params: { numPlayers: 4, pits: 8, stones: 8 } },
  ],
  // The engine only supports 2 or 4 players (the sow/capture ring geometry is
  // defined for those alone — Mancala::new asserts it), so the Players knob is
  // capped to {2, 4} via step 2. Offering 3/5/6 would trap the WASM module.
  knobs: [
    { key: 'pits', label: 'Pits', min: 2, max: 8, step: 1 },
    { key: 'stones', label: 'Stones', min: 1, max: 8, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 4, step: 2 },
  ],
  create: makeHandle,
  Board: MancalaBoard,
};
