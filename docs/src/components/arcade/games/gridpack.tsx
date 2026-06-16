import { useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';
import { usePrevBoard, changedIndex } from '../boardDiff';

const COLOR: Record<string, string> = {
  X: 'var(--arc-p1)',
  O: 'var(--arc-p2)',
  A: 'var(--arc-p3)',
  B: 'var(--arc-p4)',
};

// A generic "place a mark on a grid cell" handle over a TTT-style WASM class.
export function cellHandle(g: any, cols: number, rows: number): GameHandle {
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const b = g.get_board();
      const out: string[] = [];
      for (let i = 0; i < cols * rows; i++) if ((b[i] ?? ' ') === ' ') out.push(String(i));
      return out;
    },
    free: () => g.free(),
  };
}

// A generic grid board: tap an empty cell to place. Reused by Connect Six,
// Trap-Three, No-Tac-Toe and Square Up.
export function MarkGridBoard({ board, params, interactive, onMove }: BoardProps) {
  const { cols, rows } = params;
  const placed = changedIndex(usePrevBoard(board), board);
  return (
    <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
      {Array.from({ length: cols * rows }, (_, i) => {
        const ch = board[i] ?? ' ';
        return (
          <button
            key={i}
            className={`${styles.tttCell} ${i === placed ? styles.popIn : ''}`}
            disabled={!interactive || ch !== ' '}
            onClick={() => onMove(String(i))}
            style={{ color: COLOR[ch], fontSize: cols > 7 ? '0.9rem' : undefined }}
            aria-label={`Cell ${i + 1}: ${ch === ' ' ? 'empty' : ch}`}
          >
            {ch === ' ' ? '' : ch}
          </button>
        );
      })}
    </div>
  );
}

// ---------- Connect Six ----------
export const connectSix: GameDefinition = {
  id: 'connect-six',
  name: 'Connect Six',
  icon: '⬛',
  blurb: 'Place TWO stones per turn; first to six-in-a-row wins.',
  defaultParams: { numPlayers: 2, cols: 9, rows: 9 },
  presets: [
    { label: 'Classic 9×9', emoji: '⭐', params: { numPlayers: 2, cols: 9, rows: 9 } },
    { label: 'Big 12×12', emoji: '🔲', params: { numPlayers: 2, cols: 12, rows: 12 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 7, max: 12, step: 1 },
    { key: 'rows', label: 'Height', min: 7, max: 12, step: 1 },
  ],
  create: (wasm, p) => cellHandle(new wasm.Connect6Wasm(p.cols, p.rows), p.cols, p.rows),
  Board: MarkGridBoard,
  playerLabels: ['X', 'O'],
};

// ---------- Trap-Three (Squava) ----------
export const trapThree: GameDefinition = {
  id: 'trap-three',
  name: 'Trap-Three',
  icon: '⚠️',
  blurb: 'Four-in-a-row wins — but three-in-a-row LOSES. Watch your step.',
  defaultParams: { numPlayers: 2, cols: 5, rows: 5 },
  presets: [
    { label: 'Classic 5×5', emoji: '⭐', params: { numPlayers: 2, cols: 5, rows: 5 } },
    { label: 'Roomy 7×7', emoji: '🔲', params: { numPlayers: 2, cols: 7, rows: 7 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 8, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 8, step: 1 },
  ],
  create: (wasm, p) => cellHandle(new wasm.SquavaWasm(p.cols, p.rows), p.cols, p.rows),
  Board: MarkGridBoard,
  playerLabels: ['X', 'O'],
};

// ---------- No-Tac-Toe (Notakto) ----------
export const noTacToe: GameDefinition = {
  id: 'no-tac-toe',
  name: 'No-Tac-Toe',
  icon: '🚫',
  blurb: 'Everyone plays X. Make three-in-a-row and you LOSE.',
  defaultParams: { numPlayers: 2, cols: 3, rows: 3 },
  presets: [
    { label: 'Classic 3×3', emoji: '⭐', params: { numPlayers: 2, cols: 3, rows: 3 } },
    { label: 'Cramped 4×4', emoji: '🔲', params: { numPlayers: 2, cols: 4, rows: 4 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 3, max: 6, step: 1 },
    { key: 'rows', label: 'Height', min: 3, max: 6, step: 1 },
  ],
  create: (wasm, p) => cellHandle(new wasm.NotaktoWasm(p.cols, p.rows), p.cols, p.rows),
  Board: MarkGridBoard,
  playerLabels: ['Player 1', 'Player 2'],
};

// ---------- Square Up ----------
export const squareUp: GameDefinition = {
  id: 'square-up',
  name: 'Square Up',
  icon: '⬜',
  blurb: 'Win when four of your marks form a square — any size, even tilted.',
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Small 5×5', emoji: '🔳', params: { numPlayers: 2, cols: 5, rows: 5 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 8, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 8, step: 1 },
  ],
  create: (wasm, p) => cellHandle(new wasm.SquareUpWasm(p.cols, p.rows), p.cols, p.rows),
  Board: MarkGridBoard,
  playerLabels: ['X', 'O'],
};

// ---------- Order & Chaos ----------
function ocHandle(g: any, cols: number, rows: number): GameHandle {
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const b = g.get_board();
      const out: string[] = [];
      for (let i = 0; i < cols * rows; i++) {
        if ((b[i] ?? ' ') === ' ') {
          out.push(`${i},0`);
          out.push(`${i},1`);
        }
      }
      return out;
    },
    free: () => g.free(),
  };
}

function OrderChaosBoard({ board, params, interactive, onMove }: BoardProps) {
  const { cols, rows } = params;
  const [sym, setSym] = useState(0); // 0 = X, 1 = O
  const placed = changedIndex(usePrevBoard(board), board);
  return (
    <div>
      <div className={styles.ocToggle}>
        <span className={styles.ocLabel}>Place:</span>
        <button
          className={`${styles.ocSym} ${sym === 0 ? styles.ocSymOn : ''}`}
          style={{ color: sym === 0 ? '#fff' : COLOR.X }}
          onClick={() => setSym(0)}
        >
          X
        </button>
        <button
          className={`${styles.ocSym} ${sym === 1 ? styles.ocSymOn : ''}`}
          style={{ color: sym === 1 ? '#fff' : COLOR.O }}
          onClick={() => setSym(1)}
        >
          O
        </button>
      </div>
      <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
        {Array.from({ length: cols * rows }, (_, i) => {
          const ch = board[i] ?? ' ';
          return (
            <button
              key={i}
              className={`${styles.tttCell} ${i === placed ? styles.popIn : ''}`}
              disabled={!interactive || ch !== ' '}
              onClick={() => onMove(`${i},${sym}`)}
              style={{ color: COLOR[ch], fontSize: cols > 7 ? '0.9rem' : undefined }}
              aria-label={`Cell ${i + 1}: ${ch === ' ' ? 'empty' : ch}`}
            >
              {ch === ' ' ? '' : ch}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const orderChaos: GameDefinition = {
  id: 'order-chaos',
  name: 'Order & Chaos',
  icon: '☯️',
  blurb: 'Both players place X or O. Order wants five-in-a-row; Chaos wants to stop it.',
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Wide 8×8', emoji: '🔲', params: { numPlayers: 2, cols: 8, rows: 8 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 5, max: 9, step: 1 },
    { key: 'rows', label: 'Height', min: 5, max: 9, step: 1 },
  ],
  create: (wasm, p) => ocHandle(new wasm.OrderChaosWasm(p.cols, p.rows), p.cols, p.rows),
  Board: OrderChaosBoard,
  playerLabels: ['Order', 'Chaos'],
};
