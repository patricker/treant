import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import styles from '../arcade.module.css';

// Vertex i sits on a circle; edges are enumerated (i<j) row-major to match the
// Rust engine's edge indexing exactly.
function vpos(i: number): { x: number; y: number } {
  const a = ((i * 60 - 90) * Math.PI) / 180;
  return { x: 50 + 40 * Math.cos(a), y: 50 + 40 * Math.sin(a) };
}
const EDGES: [number, number][] = (() => {
  const e: [number, number][] = [];
  for (let i = 0; i < 6; i++) for (let j = i + 1; j < 6; j++) e.push([i, j]);
  return e;
})();

function SimBoard({ board, currentPlayer, interactive, legalMoves, onMove }: BoardProps) {
  const legal = new Set(legalMoves.map(Number).filter((n) => !Number.isNaN(n)));
  const myColor = currentPlayer === 0 ? 'var(--arc-p1)' : 'var(--arc-p2)';
  return (
    <div className={styles.simWrap}>
      <svg viewBox="0 0 100 100" className={styles.simSvg}>
        {EDGES.map(([i, j], e) => {
          const a = vpos(i);
          const b = vpos(j);
          const ch = board[e] ?? ' ';
          const colored = ch === 'X' ? 'var(--arc-p1)' : ch === 'O' ? 'var(--arc-p2)' : null;
          const clickable = interactive && legal.has(e);
          return (
            <g key={e}>
              <line
                x1={a.x}
                y1={a.y}
                x2={b.x}
                y2={b.y}
                stroke={colored ?? 'var(--arc-line, #d8ccc0)'}
                strokeWidth={colored ? 3 : 1.4}
                strokeLinecap="round"
              />
              {clickable && (
                <line
                  x1={a.x}
                  y1={a.y}
                  x2={b.x}
                  y2={b.y}
                  stroke="transparent"
                  strokeWidth={6}
                  strokeLinecap="round"
                  style={{ cursor: 'pointer' }}
                  onClick={() => onMove(String(e))}
                  onMouseEnter={(ev) => (ev.currentTarget.style.stroke = `color-mix(in srgb, ${myColor} 35%, transparent)`)}
                  onMouseLeave={(ev) => (ev.currentTarget.style.stroke = 'transparent')}
                />
              )}
            </g>
          );
        })}
        {Array.from({ length: 6 }, (_, i) => {
          const p = vpos(i);
          return <circle key={i} cx={p.x} cy={p.y} r={3.2} fill="var(--arc-ink, #4a4a5a)" />;
        })}
      </svg>
    </div>
  );
}

export const sim: GameDefinition = {
  id: 'sim',
  name: 'Sim',
  icon: '🔺',
  blurb: 'Colour the lines of a hexagon. Make a triangle in YOUR colour and you lose — someone always must.',
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2 },
  presets: [{ label: 'Classic', emoji: '⭐', params: { numPlayers: 2 } }],
  knobs: [],
  create: (wasm) => moveHandle(new wasm.SimWasm()),
  Board: SimBoard,
  playerLabels: ['Red', 'Gold'],
};
