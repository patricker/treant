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
  playerLabels: ['Red', 'Yellow'],
};
