import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// Strand's turn is TWO taps: pick where your pawn steps, then pick any tile to
// punch out. Move strings are the engine's three-part "from-to-removed". The
// board drives this as a local two-phase interaction:
//   phase 1 (dest === null): highlight every legal destination; a tap picks one.
//   phase 2 (dest set):      highlight every removable tile; a tap completes the
//                            move. Tapping your own pawn cancels back to phase 1.
function StrandBoard({ board, params, currentPlayer, interactive, legalMoves, lastCells, onMove }: BoardProps) {
  const { cols, rows } = params;
  const { t } = useT();
  const [dest, setDest] = useState<number | null>(null);
  // The session diffs the board string across a move; for Strand that changes
  // all THREE cells (vacated, stepped-onto, punched), so the last-move ring
  // covers the AI's whole two-part move.
  const last = new Set(lastCells ?? []);
  // Drop any pending destination whenever the board changes (a committed move,
  // an undo, or the opponent replying) so a stale phase-2 never lingers.
  useEffect(() => setDest(null), [board]);

  const cells = Array.from({ length: cols * rows }, (_, i) => board[i] ?? ' ');

  // All legal moves share one `from` (the current pawn). Collect destinations
  // and, per destination, the tiles it lets you remove.
  let from = -1;
  const dests = new Set<number>();
  const removableFor = new Map<number, Set<number>>();
  for (const m of legalMoves) {
    const [f, to, rem] = m.split('-').map(Number);
    from = f;
    dests.add(to);
    if (!removableFor.has(to)) removableFor.set(to, new Set());
    removableFor.get(to)!.add(rem);
  }
  const removable = dest != null ? (removableFor.get(dest) ?? new Set<number>()) : new Set<number>();

  const myColor = currentPlayer === 0 ? 'var(--arc-p1)' : 'var(--arc-p2)';

  const tap = (i: number) => {
    if (!interactive) return;
    if (dest == null) {
      // Phase 1: choose a destination.
      if (dests.has(i)) setDest(i);
      return;
    }
    // Phase 2: tapping your pawn (or the pending destination) cancels.
    if (i === from || i === dest) {
      setDest(null);
      return;
    }
    if (removable.has(i)) {
      onMove(`${from}-${dest}-${i}`);
      setDest(null);
    }
  };

  const tappable = (i: number) =>
    interactive && (dest == null ? dests.has(i) : removable.has(i) || i === from || i === dest);

  const caption =
    dest == null ? t('Tap where to step') : t('Now remove any tile 🕳️ — tap your pawn to cancel');

  return (
    <div>
      <div className={styles.shiftCaption}>{caption}</div>
      <div className={styles.strandGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {cells.map((ch, i) => {
          const isHole = ch === '.';
          const isPawn = ch === 'X' || ch === 'O';
          const isDestChoice = interactive && dest == null && dests.has(i);
          const isPendingDest = dest === i;
          // The vacated `from` cell is removable in the engine, but in the UI it
          // still shows your pawn and doubles as the cancel target — so don't
          // paint it as a removal choice (tapping it cancels instead).
          const isRemovable = interactive && dest != null && removable.has(i) && i !== from;
          const cls = [
            styles.strandCell,
            isHole ? styles.strandHole : '',
            isDestChoice ? styles.strandTarget : '',
            isRemovable ? styles.strandRemovable : '',
            isPendingDest ? styles.strandDest : '',
            last.has(i) ? styles.lastCell : '',
          ]
            .filter(Boolean)
            .join(' ');
          const stateWord = isHole ? t('hole') : isPawn ? ch : t('empty');
          return (
            <button
              key={i}
              className={cls}
              disabled={!tappable(i)}
              onClick={() => tap(i)}
              style={{
                background: isPawn
                  ? ch === 'X'
                    ? 'var(--arc-p1)'
                    : 'var(--arc-p2)'
                  : undefined,
                ...(isPendingDest ? { boxShadow: `inset 0 0 0 3px ${myColor}` } : {}),
              }}
              aria-label={t('Cell {n}: {state}', { n: i + 1, state: stateWord })}
            >
              {isPawn ? <span className={styles.strandDot} /> : isRemovable ? '🕳️' : isDestChoice ? '•' : ''}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const strand: GameDefinition = {
  id: 'strand',
  variantOf: 'trails',
  name: 'Strand',
  icon: '🏝️',
  blurb: 'Step your pawn, then punch a hole anywhere on the board. Strand your rival with nowhere to step — last pawn moving wins.',
  // Solver-on family game; calibrated by the self-play harness (scripts/calibrate.sh
  // strand 20). Hard caps at p800 — strength plateaued there.
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  defaultParams: { numPlayers: 2, cols: 7, rows: 7 },
  presets: [
    { label: 'Classic 7×7', emoji: '⭐', params: { numPlayers: 2, cols: 7, rows: 7 } },
    { label: 'Tight 5×5', emoji: '⚡', params: { numPlayers: 2, cols: 5, rows: 5 } },
    { label: 'Grand 8×8', emoji: '🤯', params: { numPlayers: 2, cols: 8, rows: 8 } },
  ],
  // Engine clamps each side to 5–8 (a hole-punch board needs room but stays
  // small enough for the solver to shine).
  knobs: [
    { key: 'cols', label: 'Width', min: 5, max: 8, step: 1 },
    { key: 'rows', label: 'Height', min: 5, max: 8, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.StrandWasm(p.cols, p.rows)),
  Board: StrandBoard,
  playerLabels: ['Red', 'Gold'],
  // Marooning ending: the loser simply ran out of tiles to step onto — there is
  // no winning line to glow, so tell the story instead of an arbitrary trophy.
  resultFlavor: ({ result, labels, seats, t }) => {
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner) || winner < 0) return undefined;
    const loser = winner === 0 ? 1 : 0;
    const name = (seat: number) => (labels[seat] ? t(labels[seat]) : t('Player {n}', { n: seat + 1 }));
    const humans = seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
    if (humans.length === 1) {
      return humans[0] === loser
        ? t('🏝️ Marooned! You had nowhere to step — {winner} wins.', { winner: name(winner) })
        : t('🏝️ Marooned! {loser} had nowhere to step — you win! 🎉', { loser: name(loser) });
    }
    return t('🏝️ Marooned! {loser} had nowhere to step — {winner} wins.', {
      winner: name(winner),
      loser: name(loser),
    });
  },
};
