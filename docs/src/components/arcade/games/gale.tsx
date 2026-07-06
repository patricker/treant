import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.GaleWasm(p.size, p.pie ?? 0);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (k) => g.playout_n(k),
    legalMoves: () => {
      // The engine surfaces the pie-rule "swap" reply alongside crossing indices.
      const s: string = g.legal_moves();
      return s ? s.split(',') : [];
    },
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    winningCells: () => {
      const s = g.winning_cells();
      return s ? s.split(',') : [];
    },
    free: () => g.free(),
  };
}

// --- crossing geometry (mirrors gale.rs decode + edge/centre maps exactly) ---
// World coords live on [0, 2n] × [0, 2n]. A crossing is Type H (idx < n²) or
// Type V. Its centre is where the Red and Gold candidate edges cross.
type Cross = { r: number; c: number; v: boolean };
function decode(idx: number, n: number): Cross {
  const h = n * n;
  if (idx < h) return { r: Math.floor(idx / n), c: idx % n, v: false };
  const j = idx - h;
  return { r: Math.floor(j / (n - 1)), c: j % (n - 1), v: true };
}
function centre({ r, c, v }: Cross): [number, number] {
  return v ? [2 * c + 2, 2 * r + 2] : [2 * c + 1, 2 * r + 1];
}
// Endpoints of the drawn edge, by owner ('X' = Red, 'O' = Gold).
function edgeEnds(x: Cross, owner: string): [number, number, number, number] {
  const { r, c, v } = x;
  if (!v) {
    return owner === 'X'
      ? [2 * c, 2 * r + 1, 2 * c + 2, 2 * r + 1] // red horizontal
      : [2 * c + 1, 2 * r, 2 * c + 1, 2 * r + 2]; // gold vertical
  }
  return owner === 'X'
    ? [2 * c + 2, 2 * r + 1, 2 * c + 2, 2 * r + 3] // red vertical
    : [2 * c + 1, 2 * r + 2, 2 * c + 3, 2 * r + 2]; // gold horizontal
}

function GaleBoard({ board, params, currentPlayer, interactive, legalMoves, winCells, onMove }: BoardProps) {
  const { t } = useT();
  const n = params.size;
  const nCross = 2 * n * n - 2 * n + 1;
  const wins = new Set(winCells ?? []);
  const canSwap = interactive && legalMoves.includes('swap');
  // Gale is the only board without a "here's what's tappable" cue. Tint every
  // unclaimed, legal crossing in the CURRENT player's colour so a first-timer
  // sees where to tap — red on Red's turn, gold after. Only while a human is to
  // move (interactive); no tint while the AI thinks or in the setup preview.
  const targets = new Set(legalMoves.map(Number).filter((v) => Number.isFinite(v)));
  const turnColor = `var(--arc-p${currentPlayer + 1})`;
  const pad = 1.4;
  const span = 2 * n + 2 * pad;
  const viewBox = `${-pad} ${-pad} ${span} ${span}`;

  const redDots = [];
  for (let r = 0; r < n; r++) {
    for (let c = 0; c <= n; c++) redDots.push([2 * c, 2 * r + 1] as const);
  }
  const goldDots = [];
  for (let r = 0; r <= n; r++) {
    for (let c = 0; c < n; c++) goldDots.push([2 * c + 1, 2 * r] as const);
  }

  return (
    <div>
      <div className={styles.shiftCaption}>
        <span className={styles.hexSwatch} style={{ background: 'var(--arc-p1)' }} />
        {t('Red joins left ↔ right')} &nbsp;·&nbsp;
        <span className={styles.hexSwatch} style={{ background: 'var(--arc-p2)' }} />
        {t('Gold joins top ↕ bottom')}
      </div>
      {canSwap && (
        <div style={{ textAlign: 'center', paddingBottom: 8 }}>
          <button
            className={styles.secondaryBtn}
            style={{ padding: '8px 16px', width: 'auto' }}
            onClick={() => onMove('swap')}
          >
            {t('♻ Swap sides')}
          </button>
        </div>
      )}
      <div className={styles.hexBoardWrap}>
        <svg viewBox={viewBox} className={styles.hexSvg}>
          {/* Coloured goal edges: Red owns left & right, Gold owns top & bottom. */}
          <g strokeLinecap="round" fill="none" opacity="0.8">
            <line x1={0} y1={0} x2={0} y2={2 * n} stroke="var(--arc-p1)" strokeWidth={0.5} />
            <line x1={2 * n} y1={0} x2={2 * n} y2={2 * n} stroke="var(--arc-p1)" strokeWidth={0.5} />
            <line x1={0} y1={0} x2={2 * n} y2={0} stroke="var(--arc-p2)" strokeWidth={0.5} />
            <line x1={0} y1={2 * n} x2={2 * n} y2={2 * n} stroke="var(--arc-p2)" strokeWidth={0.5} />
          </g>
          {/* The two dot lattices in player colours. */}
          <g>
            {redDots.map(([x, y], i) => (
              <circle key={`r${i}`} cx={x} cy={y} r={0.34} fill="var(--arc-p1)" opacity={0.62} />
            ))}
            {goldDots.map(([x, y], i) => (
              <circle key={`g${i}`} cx={x} cy={y} r={0.34} fill="var(--arc-p2)" opacity={0.62} />
            ))}
          </g>
          {/* Drawn bridges + faint tap guides + hit targets. */}
          {Array.from({ length: nCross }, (_, i) => {
            const x = decode(i, n);
            const [mx, my] = centre(x);
            const owner = board[i] ?? ' ';
            const claimed = owner === 'X' || owner === 'O';
            const clickable = interactive && !claimed;
            if (claimed) {
              const [x1, y1, x2, y2] = edgeEnds(x, owner);
              const won = wins.has(i);
              return (
                <line
                  key={`e${i}`}
                  x1={x1}
                  y1={y1}
                  x2={x2}
                  y2={y2}
                  stroke={owner === 'X' ? 'var(--arc-p1)' : 'var(--arc-p2)'}
                  strokeWidth={0.62}
                  strokeLinecap="round"
                  className={won ? styles.galeWinEdge : undefined}
                />
              );
            }
            // Tint the tap guide in the current player's colour when it's a
            // legal, tappable crossing for a human seat; otherwise leave the
            // neutral faint dot (AI thinking / setup preview / already decided).
            const tinted = clickable && targets.has(i);
            return (
              <g key={`c${i}`}>
                {/* guide so players can see where a bridge would go; tinted to
                    the current player's colour when it's their tappable move */}
                <circle
                  cx={mx}
                  cy={my}
                  r={tinted ? 0.24 : 0.16}
                  fill={tinted ? turnColor : 'var(--arc-ink)'}
                  opacity={tinted ? 0.6 : 0.28}
                />
                {/* invisible tap target. r=0.7 keeps adjacent crossings'
                    targets from overlapping (nearest crossings are √2 world
                    units apart), so a tap always resolves to the nearest bridge
                    point rather than whichever circle happens to draw on top. */}
                <circle
                  cx={mx}
                  cy={my}
                  r={0.7}
                  fill="transparent"
                  style={{ cursor: clickable ? 'pointer' : 'default' }}
                  onClick={clickable ? () => onMove(String(i)) : undefined}
                  aria-label={t('Bridge point {n}', { n: i + 1 })}
                />
              </g>
            );
          })}
        </svg>
      </div>
    </div>
  );
}

export const gale: GameDefinition = {
  id: 'gale',
  name: 'Gale',
  icon: '🌉',
  blurb: 'A bridge-building duel — connect your two sides before your rival cuts you off.',
  rules:
    'Gale (the grid Shannon switching game). Two dot grids interlock: Red’s dots and Gold’s dots. On your turn, build one bridge between two of YOUR neighbouring dots. Bridges may never cross, so every bridge you build also blocks the opponent bridge that would have crossed it. Red wins by joining the left and right sides with an unbroken chain of red bridges; Gold wins by joining the top and bottom. Exactly one player always connects — Gale can never be a draw. With the pie rule on, the second player may answer the opening bridge with a single Swap, claiming the mirrored bridge as their own, which blunts the first-move advantage.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  defaultParams: { numPlayers: 2, size: 5, pie: 0 },
  presets: [
    { label: 'Classic 5', emoji: '⭐', params: { numPlayers: 2, size: 5, pie: 0 } },
    { label: 'Small 3', emoji: '⚡', params: { numPlayers: 2, size: 3, pie: 0 } },
    { label: 'Big 6', emoji: '🤯', params: { numPlayers: 2, size: 6, pie: 0 } },
    { label: 'Tournament', emoji: '♻️', params: { numPlayers: 2, size: 5, pie: 1 } },
  ],
  // Size caps at 6: on a 390px phone n=7 packs the interleaved crossings so
  // tight that the largest non-overlapping tap target is only ~30px (measured);
  // n=6 keeps them ~34px. (Judgment point per the plan — the family arcade needs
  // reliably tappable bridge points over one more row of board.)
  knobs: [
    { key: 'size', label: 'Size', min: 3, max: 6, step: 1 },
    { key: 'pie', label: 'Pie rule', min: 0, max: 1, step: 1 },
  ],
  create: makeHandle,
  Board: GaleBoard,
  playerLabels: ['Red', 'Gold'],
};
