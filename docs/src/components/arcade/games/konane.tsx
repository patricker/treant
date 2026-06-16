import type { GameDefinition } from '../gameTypes';
import { moveHandle, MoveBoard } from './frontline';

export const konane: GameDefinition = {
  id: 'konane',
  name: 'Kōnane',
  icon: '🟤',
  blurb: 'Hawaiian leap-and-capture. Jump over an enemy stone into the gap beyond. Last to jump wins.',
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Quick 4×4', emoji: '⚡', params: { numPlayers: 2, cols: 4, rows: 4 } },
    { label: 'Big 8×8', emoji: '🔲', params: { numPlayers: 2, cols: 8, rows: 8 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 8, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 8, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.KonaneWasm(p.cols, p.rows)),
  Board: MoveBoard,
  playerLabels: ['X', 'O'],
};
