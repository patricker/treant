import type { GameDefinition } from '../gameTypes';
import { moveHandle, MoveBoard } from './frontline';

export const konane: GameDefinition = {
  id: 'konane',
  name: 'Kōnane',
  icon: '🟤',
  blurb: 'Hawaiian leap-and-capture. Jump over an enemy stone into the gap beyond. Last to jump wins.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Quick 4×4', emoji: '⚡', params: { numPlayers: 2, cols: 4, rows: 4 } },
    { label: 'Big 8×8', emoji: '🔲', params: { numPlayers: 2, cols: 8, rows: 8 } },
    { label: 'Mega 10×10', emoji: '🤯', params: { numPlayers: 2, cols: 10, rows: 10 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 10, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.KonaneWasm(p.cols, p.rows)),
  Board: MoveBoard,
  playerLabels: ['Red', 'Gold'],
  // Kōnane ends when a player has no jump left — "last to jump wins" reads as an
  // arbitrary trophy otherwise. Tell the story. (Shared resultFlavor seam.)
  resultFlavor: ({ result, winCells, labels, seats, t }) => {
    if (winCells.length > 0) return undefined; // a win-cell terminal (e.g. a line win) keeps the classic line
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner) || winner < 0) return undefined;
    const loser = winner === 0 ? 1 : 0;
    const name = (seat: number) => (labels[seat] ? t(labels[seat]) : t('Player {n}', { n: seat + 1 }));
    const humans = seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
    if (humans.length === 1) {
      return humans[0] === loser
        ? t('🌺 No jumps left! You’re out of moves — {winner} wins.', { winner: name(winner) })
        : t('🌺 No jumps left! {loser} is out of moves — you win! 🎉', { loser: name(loser) });
    }
    return t('🌺 No jumps left! {loser} had no move — {winner} wins!', {
      winner: name(winner),
      loser: name(loser),
    });
  },
};
