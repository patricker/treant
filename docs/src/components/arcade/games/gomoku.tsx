import type { GameDefinition } from '../gameTypes';
import { cellHandle, MarkGridBoard } from './gridpack';

// Gomoku is just the (well-tested) Tic-Tac-Toe m,n,k engine on a big board with
// k = 5 — but it's a famous game in its own right, so it gets its own tile.
export const gomoku: GameDefinition = {
  id: 'gomoku',
  name: 'Gomoku',
  icon: '⬛',
  blurb: 'Five stones in a row on a big board. Simple to learn, deep to master.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 80, topK: 4, temp: 1 },
  },
  defaultParams: { numPlayers: 2, cols: 13, rows: 13, k: 5 },
  presets: [
    { label: 'Classic 13×13', emoji: '⭐', params: { numPlayers: 2, cols: 13, rows: 13, k: 5 } },
    { label: 'Big 15×15', emoji: '🔲', params: { numPlayers: 2, cols: 15, rows: 15, k: 5 } },
    { label: 'Quick 9×9', emoji: '⚡', params: { numPlayers: 2, cols: 9, rows: 9, k: 5 } },
    { label: '4-Player 15×15', emoji: '🎉', params: { numPlayers: 4, cols: 15, rows: 15, k: 5 } },
    { label: '8-in-a-row', emoji: '🤯', params: { numPlayers: 2, cols: 15, rows: 15, k: 8 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 9, max: 15, step: 1 },
    { key: 'rows', label: 'Height', min: 9, max: 15, step: 1 },
    { key: 'k', label: 'In a row', min: 4, max: 8, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
  ],
  create: (wasm, p) => cellHandle(new wasm.TicTacToeWasm(p.cols, p.rows, p.k, p.numPlayers), p.cols, p.rows),
  Board: MarkGridBoard,
  playerLabels: ['X', 'O'],
};
