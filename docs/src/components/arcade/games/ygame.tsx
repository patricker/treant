import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// The board string is the triangular enumeration idx = r*(r+1)/2 + c, one char
// per cell (' '/'X'/'O'), so cell (r, c) lives at index tri(r, c).
function tri(r: number, c: number) {
  return (r * (r + 1)) / 2 + c;
}
function cellCount(n: number) {
  return (n * (n + 1)) / 2;
}

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.YGameWasm(p.size);
  const total = cellCount(p.size);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (k) => g.playout_n(k),
    legalMoves: () => {
      const b = g.get_board();
      const out: string[] = [];
      for (let i = 0; i < total; i++) if ((b[i] ?? ' ') === ' ') out.push(String(i));
      return out;
    },
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

const S = 10; // hex radius (centre → vertex) in SVG units; the SVG scales to fit
const SQRT3 = Math.sqrt(3);

// Symmetric upward-pointing triangle: row r is centred on x = 0 and widens down.
function center(r: number, c: number) {
  return { cx: S * SQRT3 * (c - r / 2), cy: S * 1.5 * r };
}
function hexPoints(cx: number, cy: number) {
  return Array.from({ length: 6 }, (_, k) => {
    const a = ((60 * k - 90) * Math.PI) / 180;
    return `${(cx + S * Math.cos(a)).toFixed(2)},${(cy + S * Math.sin(a)).toFixed(2)}`;
  }).join(' ');
}

function YBoard({ board, params, interactive, onMove }: BoardProps) {
  const { t } = useT();
  const n = params.size;
  const apex = center(0, 0);
  const bl = center(n - 1, 0);
  const br = center(n - 1, n - 1);

  const halfW = S * SQRT3 * ((n - 1) / 2) + S * SQRT3 * 0.5;
  const pad = S * 1.6;
  const minX = -halfW;
  const maxX = halfW;
  const minY = -S - 2;
  const maxY = bl.cy + S + 2;
  const viewBox = `${minX - pad} ${minY - pad} ${maxX - minX + 2 * pad} ${maxY - minY + 2 * pad}`;

  return (
    <div>
      <div className={styles.shiftCaption}>{t('Connect all three sides with one group')}</div>
      <div className={styles.hexBoardWrap}>
        <svg viewBox={viewBox} className={styles.hexSvg}>
          {/* the three goal sides of the triangle — a single group must touch all three */}
          <g strokeLinecap="round" fill="none" stroke="var(--arc-p3)" strokeWidth={S * 1.4} opacity="0.28">
            <line x1={apex.cx} y1={apex.cy} x2={bl.cx} y2={bl.cy} />
            <line x1={apex.cx} y1={apex.cy} x2={br.cx} y2={br.cy} />
            <line x1={bl.cx} y1={bl.cy} x2={br.cx} y2={br.cy} />
          </g>
          {Array.from({ length: n }, (_, r) =>
            Array.from({ length: r + 1 }, (_, c) => {
              const i = tri(r, c);
              const ch = board[i] ?? ' ';
              const { cx, cy } = center(r, c);
              const fill = ch === 'X' ? 'var(--arc-p1)' : ch === 'O' ? 'var(--arc-p2)' : 'var(--arc-card)';
              const clickable = interactive && ch === ' ';
              return (
                <polygon
                  key={i}
                  points={hexPoints(cx, cy)}
                  fill={fill}
                  stroke="rgba(0,0,0,0.16)"
                  strokeWidth="0.6"
                  style={{ cursor: clickable ? 'pointer' : 'default' }}
                  onClick={clickable ? () => onMove(String(i)) : undefined}
                  aria-label={t('Cell {n}: {state}', { n: i + 1, state: ch === ' ' ? t('empty') : ch })}
                />
              );
            }),
          )}
        </svg>
      </div>
    </div>
  );
}

export const ygame: GameDefinition = {
  id: 'y',
  name: 'Y',
  icon: '🔺',
  blurb: 'Connect all three sides of the triangle with one connected group. Like Hex, it can never be a draw.',
  rules:
    'Players take turns placing one stone on any empty cell of the triangular board. You win by connecting all THREE sides of the triangle with a single connected group of your stones — the three corners each count for both of their sides. Stones are never moved or captured, and someone always connects, so Y can never be a draw.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, size: 8 },
  presets: [
    { label: 'Classic 8', emoji: '⭐', params: { numPlayers: 2, size: 8 } },
    { label: 'Quick 5', emoji: '⚡', params: { numPlayers: 2, size: 5 } },
    { label: 'Big 10', emoji: '🔲', params: { numPlayers: 2, size: 10 } },
    { label: 'Giant 12', emoji: '🤯', params: { numPlayers: 2, size: 12 } },
  ],
  knobs: [{ key: 'size', label: 'Size', min: 5, max: 12, step: 1 }],
  create: makeHandle,
  Board: YBoard,
  playerLabels: ['Red', 'Gold'],
};
