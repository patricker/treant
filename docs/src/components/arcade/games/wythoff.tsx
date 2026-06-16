import type { GameDefinition } from '../gameTypes';
import { moveHandle, MoveBoard } from './frontline';

export const wythoff: GameDefinition = {
  id: 'wythoff',
  name: "Wythoff's Queen",
  icon: '👑',
  blurb: 'Slide the queen up, left, or diagonally toward the corner. Land on it to win.',
  defaultParams: { numPlayers: 2, cols: 8, rows: 8 },
  presets: [
    { label: 'Classic 8×8', emoji: '⭐', params: { numPlayers: 2, cols: 8, rows: 8 } },
    { label: 'Small 6×6', emoji: '🔳', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Big 10×10', emoji: '🔲', params: { numPlayers: 2, cols: 10, rows: 10 } },
    { label: 'Mega 12×12', emoji: '🤯', params: { numPlayers: 2, cols: 12, rows: 12 } },
  ],
  knobs: [],
  create: (wasm, p) => moveHandle(new wasm.WythoffWasm(p.cols)),
  Board: MoveBoard,
  playerLabels: ['Player 1', 'Player 2'],
};
