import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.ToadsFrogsWasm(p.toads, p.frogs, p.gaps);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (k) => g.playout_n(k),
    legalMoves: () => {
      const s: string = g.legal_moves();
      return s ? s.split(',') : [];
    },
    weakMove: (pl, k, t, s) => g.weak_move(pl, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

// A single creature drawn in profile so its FACING is unmistakable — the whole
// game's legibility rides on "which way does it hop?". Toads (seat 0) face RIGHT
// (they only ever move right); Frogs (seat 1) face LEFT. The base art points
// right; frogs are mirrored with scaleX(-1). The eye + pointed snout both sit on
// the leading edge, so direction reads even in a tiny cell.
function Hopper({ seat }: { seat: number }) {
  const color = `var(--arc-p${seat + 1})`;
  const facesRight = seat === 0;
  return (
    <svg
      viewBox="0 0 24 24"
      className={styles.tfCreature}
      style={{ transform: facesRight ? 'none' : 'scaleX(-1)' }}
      aria-hidden
    >
      <g stroke="rgba(0,0,0,0.35)" strokeWidth="0.7" strokeLinejoin="round">
        {/* body */}
        <ellipse cx="9.5" cy="13.5" rx="7.5" ry="5.8" fill={color} />
        {/* snout — a wedge pointing the way it hops */}
        <path d="M15 9.8 L22.5 13.5 L15 17.2 Z" fill={color} />
        {/* haunch/leg hint at the back */}
        <path d="M4 18 Q2.6 13.5 6.5 13.2 Q6 15.6 8.5 17 Z" fill={color} />
      </g>
      {/* eye on the leading edge, pupil pushed toward the front */}
      <circle cx="11" cy="10.4" r="2.5" fill="#fff" stroke="rgba(0,0,0,0.3)" strokeWidth="0.5" />
      <circle cx="12.1" cy="10.4" r="1.15" fill="#161616" />
    </svg>
  );
}

// Toads & Frogs board — a single strip. Move = the origin index of a creature
// (its destination is forced), so a legal creature is a ONE-TAP move. Movable
// creatures glow in the mover's colour; tapping a stuck creature head-shakes.
function ToadsFrogsBoard({ board, currentPlayer, interactive, legalMoves, lastCells, onMove }: BoardProps) {
  const { t } = useT();
  const cells = board.split('');
  const movable = new Set(legalMoves.map(Number));
  const last = new Set(lastCells ?? []);
  const turnColor = `var(--arc-p${currentPlayer + 1})`;

  const [shake, setShake] = useState<number | null>(null);
  // A stale shake would linger onto whatever creature slides into that index
  // after a move — clear it whenever the board changes.
  useEffect(() => setShake(null), [board]);

  const tap = (i: number, ch: string) => {
    if (!interactive) return;
    if (movable.has(i)) {
      onMove(String(i));
      return;
    }
    if (ch !== ' ') {
      setShake(i);
      window.setTimeout(() => setShake(null), 550);
    }
  };

  return (
    <div>
      <div className={styles.shiftCaption}>
        <span className={styles.hexSwatch} style={{ background: 'var(--arc-p1)' }} />
        {t('Toads hop ▶')} &nbsp;·&nbsp;
        <span className={styles.hexSwatch} style={{ background: 'var(--arc-p2)' }} />
        {t('◀ Frogs hop')} &nbsp;— {t('tap a glowing creature to move it')}
      </div>
      <div className={styles.tfRow} style={{ gridTemplateColumns: `repeat(${cells.length}, minmax(0, 1fr))` }}>
        {cells.map((ch, i) => {
          const isToad = ch === 'T';
          const isFrog = ch === 'F';
          const isMovable = interactive && movable.has(i);
          const cls = [
            styles.tfCell,
            isMovable ? styles.tfMovable : '',
            last.has(i) ? styles.lastCell : '',
            i === shake ? styles.shakeCell : '',
          ]
            .filter(Boolean)
            .join(' ');
          return (
            <button
              key={i}
              className={cls}
              disabled={!isMovable}
              onClick={() => tap(i, ch)}
              style={isMovable ? { outlineColor: turnColor } : undefined}
              aria-label={t('Square {n}: {state}', {
                n: i + 1,
                state: isToad ? t('toad') : isFrog ? t('frog') : t('empty'),
              })}
            >
              {isToad ? <Hopper seat={0} /> : isFrog ? <Hopper seat={1} /> : ''}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const toadsFrogs: GameDefinition = {
  id: 'toads-frogs',
  name: 'Toads & Frogs',
  icon: '🐸',
  blurb: 'Conway’s hopping game. Toads only go right, Frogs only go left — box your rival in so they can’t move. Toads move first and have the edge — swap seats between rounds!',
  rules:
    'A single row of squares. Toads (red) only ever move RIGHT; Frogs (gold) only ever move LEFT. On your turn either SLIDE one of your creatures forward into the next empty square, or HOP it over a single opposing creature sitting right in front, landing on the empty square just beyond (you can’t hop your own kind, and nothing is captured). No backward moves. If it’s your turn and none of your creatures can move, you lose!',
  // Calibrated via scripts/calibrate.sh toads-frogs 20 (2026-07). Strength
  // plateaus early on this tiny solved game, so Hard caps at p100.
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 100, topK: 4, temp: 1 },
  },
  defaultParams: { numPlayers: 2, toads: 3, frogs: 3, gaps: 2 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, toads: 3, frogs: 3, gaps: 2 } },
    { label: 'Tiny', emoji: '⚡', params: { numPlayers: 2, toads: 2, frogs: 2, gaps: 1 } },
    { label: 'Long March', emoji: '🤯', params: { numPlayers: 2, toads: 6, frogs: 6, gaps: 3 } },
  ],
  knobs: [
    { key: 'toads', label: 'Toads', min: 1, max: 6, step: 1 },
    { key: 'frogs', label: 'Frogs', min: 1, max: 6, step: 1 },
    { key: 'gaps', label: 'Gaps', min: 1, max: 3, step: 1 },
  ],
  create: makeHandle,
  Board: ToadsFrogsBoard,
  playerLabels: ['Toads', 'Frogs'],
  // Box-in ending: the loser simply had no move (there is no winning line to
  // glow), so the generic "{winner} wins!" trophy reads as arbitrary. Tell the
  // story instead — someone got stuck. (Adopts the shared resultFlavor seam.)
  resultFlavor: ({ result, labels, seats, t }) => {
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner) || winner < 0) return undefined;
    const loser = winner === 0 ? 1 : 0;
    const name = (seat: number) => (labels[seat] ? t(labels[seat]) : t('Player {n}', { n: seat + 1 }));
    const humans = seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
    if (humans.length === 1) {
      return humans[0] === loser
        ? t('🐸 Stuck! You have no move — {winner} wins.', { winner: name(winner) })
        : t('🐸 Stuck! {loser} has no move — you win! 🎉', { loser: name(loser) });
    }
    return t('🐸 Stuck! {loser} had no move — {winner} wins!', {
      winner: name(winner),
      loser: name(loser),
    });
  },
};
