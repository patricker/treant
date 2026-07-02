import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// 25 points on a 5×5 Alquerque lattice. Point i sits at (row, col) =
// (⌊i/5⌋, i%5); map each to a viewBox percent with a 10% margin.
const px = (v: number) => 10 + (v / 4) * 80;
const posOf = (i: number) => ({ x: px(i % 5), y: px(Math.floor(i / 5)) });

// The drawn lattice lines: orthogonals everywhere, diagonals only through the
// even points (where row+col is even) — mirrors the engine's adjacency.
const EDGES: [number, number][] = (() => {
  const e: [number, number][] = [];
  for (let r = 0; r < 5; r++) {
    for (let c = 0; c < 5; c++) {
      const i = r * 5 + c;
      if (c < 4) e.push([i, i + 1]); // right
      if (r < 4) e.push([i, i + 5]); // down
      if ((r + c) % 2 === 0 && r < 4) {
        if (c < 4) e.push([i, i + 6]); // down-right diagonal
        if (c > 0) e.push([i, i + 4]); // down-left diagonal
      }
    }
  }
  return e;
})();

function BaghchalBoard({ board, interactive, legalMoves, onMove }: BoardProps) {
  const { t } = useT();
  const [sel, setSel] = useState<number | null>(null);
  useEffect(() => setSel(null), [board]); // drop stale selection after any move

  const [cellStr, info = ''] = board.split('|');
  const cells = Array.from({ length: 25 }, (_, i) => cellStr[i] ?? ' ');
  const [inHand = 0, captured = 0] = info.split(',').map(Number);

  // Placement moves ("p<idx>") vs slides/jumps ("<from>-<to>").
  const placeSet = new Set<number>();
  const slideMap = new Map<number, Set<number>>();
  for (const m of legalMoves) {
    if (m.startsWith('p')) {
      placeSet.add(Number(m.slice(1)));
      continue;
    }
    const dash = m.indexOf('-');
    if (dash < 0) continue;
    const f = Number(m.slice(0, dash));
    const to = Number(m.slice(dash + 1));
    if (Number.isNaN(f) || Number.isNaN(to)) continue;
    if (!slideMap.has(f)) slideMap.set(f, new Set());
    slideMap.get(f)!.add(to);
  }
  const placing = placeSet.size > 0;
  const targets = sel != null ? (slideMap.get(sel) ?? new Set<number>()) : new Set<number>();

  const tap = (i: number) => {
    if (!interactive) return;
    if (placing) {
      if (placeSet.has(i)) onMove(`p${i}`);
      return;
    }
    if (sel != null && targets.has(i)) {
      onMove(`${sel}-${i}`);
      setSel(null);
      return;
    }
    setSel(slideMap.has(i) && i !== sel ? i : null);
  };

  const tappable = (i: number) => {
    if (placing) return placeSet.has(i);
    return slideMap.has(i) || targets.has(i);
  };

  const caption = placing
    ? t('Tap an empty point to place a goat')
    : sel == null
      ? t('Tap a piece')
      : t('Tap where to move');

  return (
    <div>
      <div className={styles.shiftCaption}>{caption}</div>
      <div
        style={{
          display: 'flex',
          gap: 16,
          justifyContent: 'center',
          alignItems: 'center',
          paddingBottom: 8,
          fontWeight: 800,
        }}
      >
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}>
          🐐 {t('Goats in hand: {n}', { n: inHand })}
        </span>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}>
          🐅 {t('Goats eaten: {n}', { n: captured })}
        </span>
      </div>
      <div className={styles.mtWrap} style={{ maxWidth: 360 }}>
        <svg viewBox="0 0 100 100" className={styles.mtLines}>
          {EDGES.map(([a, b], k) => {
            const pa = posOf(a);
            const pb = posOf(b);
            return <line key={k} x1={pa.x} y1={pa.y} x2={pb.x} y2={pb.y} />;
          })}
        </svg>
        {cells.map((ch, i) => {
          const p = posOf(i);
          const isSel = i === sel;
          const isTarget = targets.has(i);
          const stateWord = ch === 'T' ? t('tiger') : ch === 'G' ? t('goat') : t('empty');
          return (
            <button
              key={i}
              className={`${styles.mtCell} ${isSel ? styles.shiftSel : ''} ${isTarget ? styles.moveTarget : ''}`}
              disabled={!interactive || !tappable(i)}
              onClick={() => tap(i)}
              style={{
                left: `${p.x}%`,
                top: `${p.y}%`,
                width: '12%',
                height: '12%',
                fontSize: 17,
                lineHeight: 1,
                background: ch === 'G' ? 'var(--arc-p1)' : ch === 'T' ? 'var(--arc-p2)' : 'var(--arc-soft)',
              }}
              aria-label={t('Point {n}: {state}', { n: i, state: stateWord })}
            >
              {ch === 'T' ? '🐅' : ch === 'G' ? '🐐' : ''}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const baghchal: GameDefinition = {
  id: 'bagh-chal',
  name: 'Bagh-Chal',
  icon: '🐅',
  blurb: 'Four tigers hunt twenty goats — trap the tigers or eat five goats.',
  rules:
    'Goats move first. While goats remain in hand, the goat player places one goat on any empty point. Once all goats are placed, a goat steps one point along a line to an empty neighbour. Tigers step along a line too, or jump an adjacent goat — landing on the empty point just beyond — to capture it. Tigers win by eating enough goats; goats win by blocking every tiger so none can move.',
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 400, topK: 3, temp: 0.6 },
    hard: { playouts: 3000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, goats: 20, captures: 5 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, goats: 20, captures: 5 } },
    { label: 'Goat Horde', emoji: '🐐', params: { numPlayers: 2, goats: 24, captures: 5 } },
    { label: 'Quick Hunt', emoji: '⚡', params: { numPlayers: 2, goats: 20, captures: 3 } },
    { label: 'Tiger Feast', emoji: '🤯', params: { numPlayers: 2, goats: 24, captures: 8 } },
  ],
  knobs: [
    { key: 'goats', label: 'Goats', min: 12, max: 24, step: 1 },
    { key: 'captures', label: 'Captures to win', min: 3, max: 8, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.BaghchalWasm(p.goats, p.captures)),
  Board: BaghchalBoard,
  playerLabels: ['Goats', 'Tigers'],
};
