import type { GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { MarkGridBoard } from './gridpack';

export const captureGo: GameDefinition = {
  id: 'first-capture',
  name: 'First Capture',
  icon: '⚫',
  blurb: 'Place stones and surround an enemy group to capture it. First capture wins!',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Small 5×5', emoji: '🔳', params: { numPlayers: 2, cols: 5, rows: 5 } },
    { label: 'Mega 9×9', emoji: '🤯', params: { numPlayers: 2, cols: 9, rows: 9 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 9, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 9, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.CaptureGoWasm(p.cols, p.rows)),
  Board: MarkGridBoard,
  playerLabels: ['X', 'O'],
};
