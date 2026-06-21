import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import styles from '../arcade.module.css';

function ChompBoard({ board, params, interactive, onMove }: BoardProps) {
  const { cols, rows } = params;
  const cells = Array.from({ length: cols * rows }, (_, i) => board[i] ?? ' ');
  return (
    <div>
      <div className={styles.shiftCaption}>Eat a cookie + everything right & below. Don&apos;t eat the 💀!</div>
      <div className={styles.chompGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {cells.map((ch, i) => {
          const present = ch === '#';
          const poison = i === 0;
          return (
            <button
              key={i}
              className={styles.chompCell}
              disabled={!interactive || !present}
              onClick={() => onMove(String(i))}
              style={{
                visibility: present ? 'visible' : 'hidden',
                background: poison ? 'var(--arc-ink)' : 'var(--arc-p2)',
              }}
              aria-label={`Cell ${i + 1}: ${present ? (poison ? 'poison' : 'cookie') : 'eaten'}`}
            >
              {poison && present ? '💀' : ''}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const chomp: GameDefinition = {
  id: 'chomp',
  name: 'Chomp',
  icon: '🍪',
  blurb: 'Eat cookies (and everything right & below). Whoever eats the poison loses.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  defaultParams: { numPlayers: 2, cols: 5, rows: 4 },
  presets: [
    { label: 'Classic 5×4', emoji: '⭐', params: { numPlayers: 2, cols: 5, rows: 4 } },
    { label: 'Square 4×4', emoji: '🔳', params: { numPlayers: 2, cols: 4, rows: 4 } },
    { label: 'Big 6×5', emoji: '🔲', params: { numPlayers: 2, cols: 6, rows: 5 } },
    { label: 'Mega 7×7', emoji: '🤯', params: { numPlayers: 2, cols: 7, rows: 7 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 2, max: 7, step: 1 },
    { key: 'rows', label: 'Height', min: 2, max: 7, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.ChompWasm(p.cols, p.rows)),
  Board: ChompBoard,
  playerLabels: ['Player 1', 'Player 2'],
};
