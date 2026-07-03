import type { BoardProps, GameDefinition } from '../gameTypes';
import { PLAYER_LABEL } from '../gameTypes';
import { useT } from '../i18n';
import { cellHandle } from './gridpack';
import styles from '../arcade.module.css';
import { usePrevBoard, changedIndex } from '../boardDiff';

const SEAT = ['var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)'];
// Board symbols (engine) → seat index.
const SYM_SEAT: Record<string, number> = { X: 0, O: 1, A: 2, B: 3 };

// Custom board: a grid of stones (like MarkGridBoard) plus a per-seat
// captured-pairs scoreline. The engine encodes "<cells>|<pairs>" into get_board.
function PinchFiveBoard({ board, params, currentPlayer, interactive, onMove }: BoardProps) {
  const { t } = useT();
  const { cols, rows, numPlayers } = params;
  const [cells, countsStr] = board.split('|');
  const counts = (countsStr ?? '').split(',').map(Number);
  const placed = changedIndex(usePrevBoard(cells), cells);

  return (
    <div>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 10, justifyContent: 'center', marginBottom: 10 }}>
        {Array.from({ length: numPlayers }, (_, p) => (
          <div
            key={p}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              fontWeight: 700,
              fontSize: '0.9rem',
              opacity: p === currentPlayer ? 1 : 0.6,
            }}
          >
            <span
              style={{ width: 12, height: 12, borderRadius: '50%', background: SEAT[p], display: 'inline-block' }}
            />
            <span>{t('{name}: {pairs} pairs', { name: t(PLAYER_LABEL[p]), pairs: counts[p] ?? 0 })}</span>
          </div>
        ))}
      </div>

      <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {Array.from({ length: cols * rows }, (_, i) => {
          const ch = cells?.[i] ?? ' ';
          const seat = SYM_SEAT[ch];
          return (
            <button
              key={i}
              className={`${styles.tttCell} ${i === placed ? styles.popIn : ''}`}
              disabled={!interactive || ch !== ' '}
              onClick={() => onMove(String(i))}
              aria-label={t('Cell {n}: {state}', { n: i + 1, state: ch === ' ' ? t('empty') : ch })}
            >
              {seat !== undefined ? (
                <span
                  style={{ width: '70%', aspectRatio: '1', borderRadius: '50%', background: SEAT[seat], display: 'block' }}
                />
              ) : (
                ''
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const pinchFive: GameDefinition = {
  id: 'pinch-five',
  name: 'Pinch-Five',
  icon: '🤏',
  blurb: 'Get five-in-a-row — or capture five enemy pairs. Bracket a pair to snap it off the board.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 200, topK: 3, temp: 0.5 },
  },
  defaultParams: { numPlayers: 2, cols: 13, rows: 13, pairs: 5 },
  presets: [
    { label: 'Classic 13×13', emoji: '⭐', params: { numPlayers: 2, cols: 13, rows: 13, pairs: 5 } },
    { label: 'Quick 9×9', emoji: '⚡', params: { numPlayers: 2, cols: 9, rows: 9, pairs: 3 } },
    { label: 'Full 19×19', emoji: '🔲', params: { numPlayers: 2, cols: 19, rows: 19, pairs: 5 } },
    { label: '4-Player 13×13', emoji: '🎉', params: { numPlayers: 4, cols: 13, rows: 13, pairs: 5 } },
    { label: '4-Player 19×19', emoji: '🤯', params: { numPlayers: 4, cols: 19, rows: 19, pairs: 5 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 9, max: 19, step: 1 },
    { key: 'rows', label: 'Height', min: 9, max: 19, step: 1 },
    { key: 'pairs', label: 'Pairs to win', min: 3, max: 8, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 4, step: 1 },
  ],
  create: (wasm, p) => cellHandle(new wasm.PinchFiveWasm(p.cols, p.rows, p.pairs, p.numPlayers), p.cols, p.rows),
  Board: PinchFiveBoard,
};
