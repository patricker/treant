import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

function ReversiBoard({ board, params, interactive, legalMoves, lastCells, playerNames, onMove }: BoardProps) {
  const { cols, rows } = params;
  const { t } = useT();
  const cells = Array.from({ length: cols * rows }, (_, i) => board[i] ?? ' ');
  const mustPass = legalMoves.length === 1 && legalMoves[0] === 'pass';
  const legalSet = new Set(legalMoves.filter((m) => m !== 'pass'));
  const last = new Set(lastCells ?? []);
  const x = cells.filter((c) => c === 'X').length;
  const o = cells.filter((c) => c === 'O').length;

  return (
    <div>
      <div className={styles.reversiScores}>
        <span style={{ color: 'var(--arc-p1)' }}>● {x}</span>
        <span style={{ color: 'var(--arc-p2)' }}>● {o}</span>
      </div>
      <div className={styles.reversiGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {cells.map((ch, i) => {
          const isLegal = interactive && legalSet.has(String(i));
          return (
            <button
              key={i}
              className={`${styles.reversiCell} ${last.has(i) ? styles.lastCell : ''}`}
              disabled={!isLegal}
              onClick={() => onMove(String(i))}
              aria-label={t('Cell {n}: {state}', { n: i + 1, state: ch === ' ' ? t('empty') : ch === 'X' ? (playerNames?.[0] ?? ch) : (playerNames?.[1] ?? ch) })}
            >
              {ch !== ' ' ? (
                <span
                  className={styles.reversiDisc}
                  style={{ background: ch === 'X' ? 'var(--arc-p1)' : 'var(--arc-p2)' }}
                />
              ) : isLegal ? (
                <span className={styles.reversiHint} />
              ) : null}
            </button>
          );
        })}
      </div>
      {mustPass && interactive && (
        <button className={styles.playBtn} style={{ marginTop: 12 }} onClick={() => onMove('pass')}>
          {t('No moves — Pass ⤳')}
        </button>
      )}
    </div>
  );
}

export const reversi: GameDefinition = {
  id: 'reversi',
  name: 'Reversi',
  icon: '⚫',
  blurb: 'Flank a line of enemy discs to flip them. Most discs when the board fills wins.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 8, rows: 8 },
  presets: [
    { label: 'Classic 8×8', emoji: '⭐', params: { numPlayers: 2, cols: 8, rows: 8 } },
    { label: 'Small 6×6', emoji: '🔳', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Mega 10×10', emoji: '🤯', params: { numPlayers: 2, cols: 10, rows: 10 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 10, step: 2 },
    { key: 'rows', label: 'Height', min: 4, max: 10, step: 2 },
  ],
  create: (wasm, p) => moveHandle(new wasm.ReversiWasm(p.cols, p.rows)),
  Board: ReversiBoard,
  playerLabels: ['Red', 'Yellow'],
};
