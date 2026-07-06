import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// World Threes bundles eight traditional place-and-slide three-in-a-row games,
// one per preset. The board GEOMETRY is the customization: each board's points,
// drawn edges and legal moves all come from the Rust engine. The renderer reads
// the engine's own `get_layout()` (points + edges) so the drawn board can never
// diverge from the legal moves — the layout is cached here at `create()` time
// (create always runs before the board renders: setup preview and session both
// build a throwaway/real engine first).
const layoutCache = new Map<number, string>();

function makeHandle(wasm: any, board: number) {
  const g = new wasm.WorldThreesWasm(board);
  layoutCache.set(board, g.get_layout());
  return moveHandle(g);
}

type Layout = { pts: { x: number; y: number }[]; edges: [number, number][] };
function parseLayout(s: string | undefined): Layout | null {
  if (!s) return null;
  const [ptStr, edgeStr = ''] = s.split('|');
  const pts = ptStr.split(' ').map((p) => {
    const [x, y] = p.split(',').map(Number);
    return { x, y };
  });
  const edges = edgeStr
    ? edgeStr.split(',').map((e) => {
        const [a, b] = e.split('-').map(Number);
        return [a, b] as [number, number];
      })
    : [];
  return { pts, edges };
}

function WorldThreesBoard({ board, params, interactive, legalMoves, onMove }: BoardProps) {
  const { t } = useT();
  const [sel, setSel] = useState<number | null>(null);
  useEffect(() => setSel(null), [board]); // drop stale selection after any move

  const layout = parseLayout(layoutCache.get(params.board ?? 0));
  if (!layout) return <div className={styles.mtWrap} />;
  const n = layout.pts.length;
  const cells = Array.from({ length: n }, (_, i) => board[i] ?? ' ');

  // Placement moves are a bare point index; slides/jumps are "from-to".
  const placeSet = new Set<number>();
  const fromTo = new Map<number, Set<number>>();
  for (const m of legalMoves) {
    const dash = m.indexOf('-');
    if (dash < 0) {
      const p = Number(m);
      if (!Number.isNaN(p)) placeSet.add(p);
    } else {
      const f = Number(m.slice(0, dash));
      const to = Number(m.slice(dash + 1));
      if (!fromTo.has(f)) fromTo.set(f, new Set());
      fromTo.get(f)!.add(to);
    }
  }
  const targets = sel != null ? (fromTo.get(sel) ?? new Set<number>()) : new Set<number>();

  const tap = (i: number) => {
    if (!interactive) return;
    // Slide: a piece is selected and this is one of its targets.
    if (sel != null && targets.has(i)) {
      onMove(`${sel}-${i}`);
      setSel(null);
      return;
    }
    // Select one of your movable pieces (re-tap to deselect).
    if (fromTo.has(i)) {
      setSel(i !== sel ? i : null);
      return;
    }
    // Otherwise an empty placement point drops a man.
    if (placeSet.has(i)) {
      setSel(null);
      onMove(`${i}`);
      return;
    }
    setSel(null);
  };

  const caption =
    placeSet.size > 0
      ? t('Tap an empty point to place')
      : sel != null
        ? t('Tap where to move')
        : t('Tap a piece to move');

  const tappable = (i: number) => fromTo.has(i) || targets.has(i) || placeSet.has(i);
  const sz = n > 9 ? 11 : 15; // smaller dots on the crowded 13-point board

  return (
    <div>
      <div className={styles.shiftCaption}>{caption}</div>
      <div className={styles.mtWrap}>
        <svg viewBox="0 0 100 100" className={styles.mtLines}>
          {layout.edges.map(([a, b], k) => (
            <line key={k} x1={layout.pts[a].x} y1={layout.pts[a].y} x2={layout.pts[b].x} y2={layout.pts[b].y} />
          ))}
        </svg>
        {cells.map((ch, i) => {
          const p = layout.pts[i];
          const isSel = i === sel;
          const isTarget = targets.has(i);
          const stateWord = ch === 'X' ? t('Red') : ch === 'O' ? t('Gold') : t('empty');
          return (
            <button
              key={i}
              className={`${styles.mtCell} ${isSel ? styles.shiftSel : ''} ${isTarget ? styles.moveTarget : ''}`}
              disabled={!interactive || !tappable(i)}
              onClick={() => tap(i)}
              style={{
                left: `${p.x}%`,
                top: `${p.y}%`,
                width: `${sz}%`,
                height: `${sz}%`,
                background: ch === 'X' ? 'var(--arc-p1)' : ch === 'O' ? 'var(--arc-p2)' : 'transparent',
                border: ch === ' ' ? '2.5px solid var(--arc-soft)' : undefined,
              }}
              aria-label={t('Point {n}: {state}', { n: i + 1, state: stateWord })}
            />
          );
        })}
      </div>
    </div>
  );
}

export const worldThrees: GameDefinition = {
  id: 'world-threes',
  name: 'World Threes',
  icon: '🌍',
  blurb: 'Eight of the world’s traditional three-in-a-row games — place, then slide along the lines. Most race to three-in-a-row; a couple win by trapping. Pick a country to change the board.',
  // Solver-tiny boards: the self-play calibration ladder was degenerate/noisy
  // (Hard "plateaued" at a non-deterministic rung), so — as with the Morris
  // variants — we hand-tune. A deterministic p800 Hard lets the exact solver
  // play these small boards perfectly (the whole point); Easy is honestly
  // beatable; Medium sits between.
  difficulty: {
    easy: { playouts: 10, topK: 6, temp: 3 },
    medium: { playouts: 120, topK: 3, temp: 0.6 },
    hard: { playouts: 800, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, board: 0 },
  presets: [
    { label: 'Achi (Ghana)', emoji: '⭐', params: { numPlayers: 2, board: 0 } },
    { label: 'Tapatan (Philippines)', emoji: '🔷', params: { numPlayers: 2, board: 1 } },
    { label: 'Shisima (Kenya)', emoji: '🎯', params: { numPlayers: 2, board: 2 } },
    { label: 'Tant Fant (India)', emoji: '🔶', params: { numPlayers: 2, board: 3 } },
    { label: 'Nine Holes (England)', emoji: '🕳️', params: { numPlayers: 2, board: 4 } },
    { label: 'Tsoro Yematatu (Zimbabwe)', emoji: '🔺', params: { numPlayers: 2, board: 5 } },
    { label: 'Picaria (Zuni)', emoji: '💠', params: { numPlayers: 2, board: 6 } },
    { label: 'Pong Hau K’i (China)', emoji: '🤯', params: { numPlayers: 2, board: 7 } },
  ],
  knobs: [],
  create: (wasm, p) => makeHandle(wasm, p.board ?? 0),
  Board: WorldThreesBoard,
  playerLabels: ['Red', 'Gold'],
};
