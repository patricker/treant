import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.HexWasm(p.size);
  const n = p.size;
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
      for (let i = 0; i < n * n; i++) if ((b[i] ?? ' ') === ' ') out.push(String(i));
      return out;
    },
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

const S = 10; // hex radius (centre → vertex) in SVG units; the SVG scales to fit
const SQRT3 = Math.sqrt(3);

function center(r: number, c: number) {
  return { cx: S * SQRT3 * (c + r / 2), cy: S * 1.5 * r };
}
function hexPoints(cx: number, cy: number) {
  return Array.from({ length: 6 }, (_, k) => {
    const a = ((60 * k - 90) * Math.PI) / 180;
    return `${(cx + S * Math.cos(a)).toFixed(2)},${(cy + S * Math.sin(a)).toFixed(2)}`;
  }).join(' ');
}

function HexBoard({ board, params, interactive, onMove }: BoardProps) {
  const { t } = useT();
  const n = params.size;
  const tl = center(0, 0);
  const tr = center(0, n - 1);
  const bl = center(n - 1, 0);
  const br = center(n - 1, n - 1);
  const pad = S * 1.6;
  const minX = tl.cx - S * SQRT3;
  const maxX = br.cx + S * SQRT3;
  const minY = -S - 2;
  const maxY = bl.cy + S + 2;
  const viewBox = `${minX - pad} ${minY - pad} ${maxX - minX + 2 * pad} ${maxY - minY + 2 * pad}`;

  return (
    <div>
      <div className={styles.shiftCaption}>
        <span className={styles.hexSwatch} style={{ background: 'var(--arc-p1)' }} />
        {t('Red joins top ↕ bottom')} &nbsp;·&nbsp;
        <span className={styles.hexSwatch} style={{ background: 'var(--arc-p2)' }} />
        {t('Gold joins left ↔ right')}
      </div>
      <div className={styles.hexBoardWrap}>
        <svg viewBox={viewBox} className={styles.hexSvg}>
          {/* coloured goal edges along the rhombus border */}
          <g strokeLinecap="round" fill="none">
            <line x1={tl.cx} y1={tl.cy} x2={tr.cx} y2={tr.cy} stroke="var(--arc-p1)" strokeWidth={S * 1.5} opacity="0.85" />
            <line x1={bl.cx} y1={bl.cy} x2={br.cx} y2={br.cy} stroke="var(--arc-p1)" strokeWidth={S * 1.5} opacity="0.85" />
            <line x1={tl.cx} y1={tl.cy} x2={bl.cx} y2={bl.cy} stroke="var(--arc-p2)" strokeWidth={S * 1.5} opacity="0.85" />
            <line x1={tr.cx} y1={tr.cy} x2={br.cx} y2={br.cy} stroke="var(--arc-p2)" strokeWidth={S * 1.5} opacity="0.85" />
          </g>
          {Array.from({ length: n }, (_, r) =>
            Array.from({ length: n }, (_, c) => {
              const i = r * n + c;
              const ch = board[i] ?? ' ';
              const { cx, cy } = center(r, c);
              const fill = ch === 'X' ? 'var(--arc-p1)' : ch === 'O' ? 'var(--arc-p2)' : 'var(--arc-card)';
              const clickable = interactive && ch === ' ';
              return (
                <polygon
                  key={i}
                  points={hexPoints(cx, cy)}
                  fill={fill}
                  stroke="rgba(255,255,255,0.16)"
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

export const hex: GameDefinition = {
  id: 'hex',
  name: 'Hex',
  icon: '⬡',
  blurb: 'Connect your two sides with one unbroken chain of stones. It can never be a draw.',
  rules:
    'Players take turns placing one stone on any empty hexagon. Red wins by linking the top edge to the bottom edge with a connected chain of red stones; Gold wins by linking the left edge to the right edge. Stones are never moved or captured, and exactly one player always completes a connection — Hex can never be a draw.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, size: 7 },
  presets: [
    { label: 'Classic 7×7', emoji: '⭐', params: { numPlayers: 2, size: 7 } },
    { label: 'Small 5×5', emoji: '🔳', params: { numPlayers: 2, size: 5 } },
    { label: 'Big 9×9', emoji: '🔲', params: { numPlayers: 2, size: 9 } },
    { label: 'Mega 11×11', emoji: '🤯', params: { numPlayers: 2, size: 11 } },
  ],
  knobs: [{ key: 'size', label: 'Size', min: 5, max: 11, step: 1 }],
  create: makeHandle,
  Board: HexBoard,
  playerLabels: ['Red', 'Gold'],
};
