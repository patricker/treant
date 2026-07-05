import type { GameDefinition } from '../gameTypes';
import { moveHandle, MoveBoard } from './frontline';

export const wythoff: GameDefinition = {
  id: 'wythoff',
  name: "Wythoff's Queen",
  icon: '👑',
  blurb: 'Slide the queen up, left, or diagonally toward the corner. Land on it to win.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 100, topK: 4, temp: 1 },
  },
  defaultParams: { numPlayers: 2, cols: 8, rows: 8 },
  presets: [
    { label: 'Classic 8×8', emoji: '⭐', params: { numPlayers: 2, cols: 8, rows: 8 } },
    { label: 'Small 6×6', emoji: '🔳', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Big 10×10', emoji: '🔲', params: { numPlayers: 2, cols: 10, rows: 10 } },
    { label: 'Mega 12×12', emoji: '🤯', params: { numPlayers: 2, cols: 12, rows: 12 } },
  ],
  knobs: [{ key: 'cols', label: 'Size', min: 4, max: 12, step: 1 }],
  create: (wasm, p) => moveHandle(new wasm.WythoffWasm(p.cols)),
  // The engine is square (n×n): mirror the single Size knob into rows so the
  // shared MoveBoard renders the same board the engine plays.
  Board: (props) => <MoveBoard {...props} params={{ ...props.params, rows: props.params.cols }} />,
  playerLabels: ['Player 1', 'Player 2'],
};
