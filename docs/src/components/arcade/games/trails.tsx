import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

function TrailsBoard({ board, params, interactive, legalMoves, onMove }: BoardProps) {
  const { cols, rows } = params;
  const { t } = useT();
  const cells = Array.from({ length: cols * rows }, (_, i) => board[i] ?? ' ');
  // every legal move shares the same `from` (the current token); collect targets
  let from = -1;
  const tos = new Set<number>();
  for (const m of legalMoves) {
    const dash = m.indexOf('-');
    if (dash < 0) continue;
    from = Number(m.slice(0, dash));
    tos.add(Number(m.slice(dash + 1)));
  }
  return (
    <div className={styles.trailsGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
      {cells.map((ch, i) => {
        const isWall = ch === '#';
        const isToken = ch === 'X' || ch === 'O';
        const isTarget = interactive && tos.has(i);
        return (
          <button
            key={i}
            className={`${styles.trailsCell} ${isTarget ? styles.trailsTarget : ''}`}
            disabled={!isTarget}
            onClick={() => from >= 0 && onMove(`${from}-${i}`)}
            style={{
              background: isWall
                ? 'var(--arc-ink)'
                : isToken
                  ? ch === 'X'
                    ? 'var(--arc-p1)'
                    : 'var(--arc-p2)'
                  : 'var(--arc-soft)',
            }}
            aria-label={t('Cell {n}: {state}', { n: i + 1, state: isWall ? t('wall') : isToken ? ch : t('empty') })}
          >
            {isToken ? <span className={styles.trailsDot} /> : isTarget ? '•' : ''}
          </button>
        );
      })}
    </div>
  );
}

export const trails: GameDefinition = {
  id: 'trails',
  name: 'Trails',
  icon: '🟥',
  blurb: 'Move and leave a wall behind. Box in your opponent — last to move wins.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Big 8×8', emoji: '🔲', params: { numPlayers: 2, cols: 8, rows: 8 } },
    { label: 'Mega 9×9', emoji: '🤯', params: { numPlayers: 2, cols: 9, rows: 9 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 9, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 9, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.TrailsWasm(p.cols, p.rows, 0)),
  Board: TrailsBoard,
  playerLabels: ['Red', 'Gold'],
  // Boxing the opponent in is the whole game — the generic winner line hides the
  // trap. Tell the story. (Shared resultFlavor seam.)
  resultFlavor: ({ result, labels, seats, t }) => {
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner) || winner < 0) return undefined;
    const loser = winner === 0 ? 1 : 0;
    const name = (seat: number) => (labels[seat] ? t(labels[seat]) : t('Player {n}', { n: seat + 1 }));
    const humans = seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
    if (humans.length === 1) {
      return humans[0] === loser
        ? t('🧱 Boxed in! You have no move — {winner} wins.', { winner: name(winner) })
        : t('🧱 Boxed in! {loser} has no move — you win! 🎉', { loser: name(loser) });
    }
    return t('🧱 Boxed in! {loser} had no move — {winner} wins!', {
      winner: name(winner),
      loser: name(loser),
    });
  },
};

export const joust: GameDefinition = {
  id: 'joust',
  variantOf: 'trails',
  name: 'Joust',
  icon: '🐴',
  blurb: 'Knights on a burning board: leap, scorch the square you left, outlast your rival.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Big 8×8', emoji: '🔲', params: { numPlayers: 2, cols: 8, rows: 8 } },
    // Engine clamps boards to 4–9, so the 🤯 preset tops out at 9×9.
    { label: 'Mega 9×9', emoji: '🤯', params: { numPlayers: 2, cols: 9, rows: 9 } },
  ],
  // Knobs start at 5: a knight needs room to leap, so we keep boards comfortably
  // above the 4×4 floor (every spawn still has a legal knight move at 5×5+).
  knobs: [
    { key: 'cols', label: 'Width', min: 5, max: 9, step: 1 },
    { key: 'rows', label: 'Height', min: 5, max: 9, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.TrailsWasm(p.cols, p.rows, 1)),
  Board: TrailsBoard,
  playerLabels: ['Red', 'Gold'],
  // A knight loses when its own scorched trail leaves it nowhere to leap. Tell
  // the story instead of a bare winner line. (Shared resultFlavor seam.)
  resultFlavor: ({ result, labels, seats, t }) => {
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner) || winner < 0) return undefined;
    const loser = winner === 0 ? 1 : 0;
    const name = (seat: number) => (labels[seat] ? t(labels[seat]) : t('Player {n}', { n: seat + 1 }));
    const humans = seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
    if (humans.length === 1) {
      return humans[0] === loser
        ? t('🔥 Scorched in! Your knight can’t leap — {winner} wins.', { winner: name(winner) })
        : t('🔥 Scorched in! {loser} can’t leap — you win! 🎉', { loser: name(loser) });
    }
    return t('🔥 Scorched in! {loser} had no leap — {winner} wins!', {
      winner: name(winner),
      loser: name(loser),
    });
  },
};
