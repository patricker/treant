import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { usePrevBoard, changedIndex } from '../boardDiff';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

const COLOR: Record<string, string> = {
  X: 'var(--arc-p1)',
  O: 'var(--arc-p2)',
};

// Unlike a plain "place anywhere" grid, NoGo's legal cells are a subset of the
// empty cells (placements that neither capture nor self-capture), so the board
// is driven by the engine's legalMoves rather than "any empty cell".
function NoGoBoard({ board, params, interactive, legalMoves, lastCells, onMove }: BoardProps) {
  const { cols, rows } = params;
  const { t } = useT();
  const legal = new Set(legalMoves.map(Number).filter((n) => !Number.isNaN(n)));
  const last = new Set(lastCells ?? []);
  const placed = changedIndex(usePrevBoard(board), board);
  return (
    <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
      {Array.from({ length: cols * rows }, (_, i) => {
        const ch = board[i] ?? ' ';
        const isLegal = interactive && legal.has(i);
        return (
          <button
            key={i}
            className={`${styles.tttCell} ${i === placed ? styles.popIn : ''} ${isLegal ? styles.nogoLegal : ''} ${last.has(i) ? styles.lastCell : ''}`}
            disabled={!isLegal}
            onClick={() => onMove(String(i))}
            style={{ color: COLOR[ch] }}
            aria-label={t('Cell {n}: {state}', { n: i + 1, state: ch === ' ' ? t('empty') : ch })}
          >
            {ch === ' ' ? '' : '●'}
          </button>
        );
      })}
    </div>
  );
}

export const nogo: GameDefinition = {
  id: 'nogo',
  name: 'NoGo',
  icon: '⛔',
  blurb: 'The anti-Go: place stones, but never capture or self-trap. Run out of safe moves and you lose.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
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
  create: (wasm, p) => moveHandle(new wasm.NoGoWasm(p.cols, p.rows)),
  Board: NoGoBoard,
  playerLabels: ['Red', 'Yellow'],
};
