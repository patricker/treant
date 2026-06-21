import type { GameDefinition } from '../gameTypes';
import { moveHandle, MoveBoard } from './frontline';

export const clobber: GameDefinition = {
  id: 'clobber',
  name: 'Clobber',
  icon: '💥',
  blurb: 'Hop a stone onto an adjacent enemy to clobber it. Last to move wins.',
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 5, rows: 5 },
  presets: [
    { label: 'Classic 5×5', emoji: '⭐', params: { numPlayers: 2, cols: 5, rows: 5 } },
    { label: 'Small 4×4', emoji: '🔳', params: { numPlayers: 2, cols: 4, rows: 4 } },
    { label: 'Wide 6×5', emoji: '🔲', params: { numPlayers: 2, cols: 6, rows: 5 } },
    { label: 'Mega 8×8', emoji: '🤯', params: { numPlayers: 2, cols: 8, rows: 8 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 3, max: 8, step: 1 },
    { key: 'rows', label: 'Height', min: 3, max: 8, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.ClobberWasm(p.cols, p.rows)),
  Board: MoveBoard,
  playerLabels: ['Red', 'Yellow'],
};
