import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

const SEAT_COLOR = ['var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)', 'var(--arc-p5)', 'var(--arc-p6)'];
const NUM_PLAYERS = 2;

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.OwareWasm(p.pits, p.seeds);
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

// Board string: "c0,c1,…,c{2p-1}|s0,s1" — house counts in ring order (P0 houses
// then P1 houses), a '|', then the two capture-pile scores.
function OwareBoard({ board, params, currentPlayer, interactive, legalMoves, onMove }: BoardProps) {
  const { t } = useT();
  const pits = params.pits;
  const [countsStr, scoresStr] = board.split('|');
  const counts = (countsStr ?? '').split(',').map(Number);
  const scores = (scoresStr ?? '0,0').split(',').map(Number);
  const houseVal = (pl: number, i: number) => counts[pl * pits + i] ?? 0;
  const legal = new Set(legalMoves);

  return (
    <div className={styles.mancalaWrap}>
      {Array.from({ length: NUM_PLAYERS }, (_, pl) => {
        const mine = pl === currentPlayer;
        return (
          <div
            key={pl}
            className={`${styles.mancalaRow} ${mine ? styles.mancalaRowActive : ''}`}
            style={{ borderColor: SEAT_COLOR[pl] }}
          >
            <div
              className={styles.mancalaStore}
              style={{ background: SEAT_COLOR[pl] }}
              aria-label={t('Player {n} captured: {v}', { n: pl + 1, v: scores[pl] ?? 0 })}
            >
              {scores[pl] ?? 0}
            </div>
            <div className={styles.mancalaPits}>
              {Array.from({ length: pits }, (_, i) => {
                const v = houseVal(pl, i);
                // Only the current player's houses are ever tappable, and only the
                // ones the engine reports as legal (the starvation rule can forbid
                // an otherwise non-empty house).
                const playable = interactive && mine && legal.has(String(i));
                return (
                  <button
                    key={i}
                    className={styles.mancalaPit}
                    disabled={!playable}
                    onClick={() => onMove(String(i))}
                    style={{ borderColor: SEAT_COLOR[pl] }}
                    aria-label={t('Player {n} house {i}: {v} seeds', { n: pl + 1, i: i + 1, v })}
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

export const oware: GameDefinition = {
  id: 'oware',
  name: 'Oware',
  icon: '🫘',
  blurb: 'Sow seeds around the board and capture the pits you bring to 2 or 3.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, pits: 6, seeds: 4 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, pits: 6, seeds: 4 } },
    { label: 'Quick', emoji: '⚡', params: { numPlayers: 2, pits: 4, seeds: 3 } },
    { label: 'Big', emoji: '🔲', params: { numPlayers: 2, pits: 8, seeds: 4 } },
    { label: 'Loaded', emoji: '🤯', params: { numPlayers: 2, pits: 6, seeds: 6 } },
  ],
  // Oware is a strictly 2-player game — no numPlayers knob.
  knobs: [
    { key: 'pits', label: 'Pits', min: 4, max: 8, step: 1 },
    { key: 'seeds', label: 'Seeds', min: 3, max: 6, step: 1 },
  ],
  create: makeHandle,
  Board: OwareBoard,
};
