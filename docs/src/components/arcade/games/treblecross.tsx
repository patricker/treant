import type { GameDefinition } from '../gameTypes';
import { cellHandle, MarkGridBoard } from './gridpack';

// A one-row strip. Both players drop the same X; the grid board renders it as a
// single line of cells (rows: 1). Reuses the generic place-on-a-cell handle.
export const treblecross: GameDefinition = {
  id: 'treblecross',
  name: 'Treblecross',
  icon: '➕',
  blurb: 'One strip, and you BOTH play X. Whoever completes three X’s in a row wins.',
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 11, rows: 1 },
  presets: [
    { label: 'Classic (11)', emoji: '⭐', params: { numPlayers: 2, cols: 11, rows: 1 } },
    { label: 'Short (7)', emoji: '⚡', params: { numPlayers: 2, cols: 7, rows: 1 } },
    { label: 'Long (15)', emoji: '🔲', params: { numPlayers: 2, cols: 15, rows: 1 } },
    { label: 'Epic (16)', emoji: '🤯', params: { numPlayers: 2, cols: 16, rows: 1 } },
  ],
  knobs: [{ key: 'cols', label: 'Length', min: 6, max: 16, step: 1 }],
  create: (wasm, p) => cellHandle(new wasm.TreblecrossWasm(p.cols), p.cols, 1),
  Board: MarkGridBoard,
  playerLabels: ['Player 1', 'Player 2'],
};
