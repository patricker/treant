import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { usePrevBoard, changedIndex } from '../boardDiff';
import styles from '../arcade.module.css';

// Col fills cells with solid colour "territory". Legal cells are a subset of
// the empties (those not touching your own colour), so the board is driven by
// the engine's legalMoves.
function ColBoard({ board, params, currentPlayer, interactive, legalMoves, onMove }: BoardProps) {
  const { cols, rows } = params;
  const legal = new Set(legalMoves.map(Number).filter((n) => !Number.isNaN(n)));
  const placed = changedIndex(usePrevBoard(board), board);
  const myColor = currentPlayer === 0 ? 'var(--arc-p1)' : 'var(--arc-p2)';
  return (
    <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
      {Array.from({ length: cols * rows }, (_, i) => {
        const ch = board[i] ?? ' ';
        const isLegal = interactive && legal.has(i);
        const fill = ch === 'X' ? 'var(--arc-p1)' : ch === 'O' ? 'var(--arc-p2)' : undefined;
        return (
          <button
            key={i}
            className={`${styles.tttCell} ${i === placed ? styles.popIn : ''} ${isLegal ? styles.colLegal : ''}`}
            disabled={!isLegal}
            onClick={() => onMove(String(i))}
            style={{ background: fill, ['--col-hint' as string]: myColor }}
            aria-label={`Cell ${i + 1}: ${ch === ' ' ? 'empty' : ch}`}
          />
        );
      })}
    </div>
  );
}

export const col: GameDefinition = {
  id: 'col',
  name: 'Col',
  icon: '🎨',
  blurb: 'Colour the map — but no two of your own patches may touch. Run out of room and you lose.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 5, rows: 5 },
  presets: [
    { label: 'Classic 5×5', emoji: '⭐', params: { numPlayers: 2, cols: 5, rows: 5 } },
    { label: 'Quick 4×4', emoji: '⚡', params: { numPlayers: 2, cols: 4, rows: 4 } },
    { label: 'Big 7×7', emoji: '🔲', params: { numPlayers: 2, cols: 7, rows: 7 } },
    { label: 'Mega 8×8', emoji: '🤯', params: { numPlayers: 2, cols: 8, rows: 8 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 8, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 8, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.ColWasm(p.cols, p.rows)),
  Board: ColBoard,
  playerLabels: ['Red', 'Blue'],
};
