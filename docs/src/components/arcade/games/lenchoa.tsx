import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// Len Choa is a fixed-form 10-point triangular board. The geometry (points +
// drawn edges) is the engine's single source of truth, read once from
// get_layout() at create() time (create always runs before the board renders:
// setup preview and session both build an engine first), so the drawn board can
// never diverge from the legal moves.
let layoutStr: string | undefined;

function makeHandle(wasm: any) {
  const g = new wasm.LenChoaWasm();
  layoutStr = g.get_layout();
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

function LenChoaBoard({ board, interactive, legalMoves, onMove }: BoardProps) {
  const { t } = useT();
  const [sel, setSel] = useState<number | null>(null);
  useEffect(() => setSel(null), [board]); // drop stale selection after any move

  const layout = parseLayout(layoutStr);
  if (!layout) return <div className={styles.mtWrap} />;
  const n = layout.pts.length;

  const [cellStr, info = ''] = board.split('|');
  const cells = Array.from({ length: n }, (_, i) => cellStr[i] ?? ' ');
  const [inHand = 0, captured = 0] = info.split(',').map(Number);

  // Placement moves are a bare point index; slides/leaps are "from-to".
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
    // Move: a piece is selected and this is one of its destinations.
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
    // Otherwise an empty placement point drops a leopard.
    if (placeSet.has(i)) {
      setSel(null);
      onMove(`${i}`);
      return;
    }
    setSel(null);
  };

  const tappable = (i: number) => fromTo.has(i) || targets.has(i) || placeSet.has(i);

  const caption =
    placeSet.size > 0
      ? t('Tap an empty point to place a leopard')
      : sel != null
        ? t('Tap where to move')
        : t('Tap a piece to move');

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
          🐆 {t('Leopards to place: {n}', { n: inHand })}
        </span>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}>
          🐅 {t('Leopards eaten: {n} / 3', { n: captured })}
        </span>
      </div>
      <div className={styles.mtWrap} style={{ maxWidth: 340 }}>
        <svg viewBox="0 0 100 100" className={styles.mtLines}>
          {layout.edges.map(([a, b], k) => (
            <line key={k} x1={layout.pts[a].x} y1={layout.pts[a].y} x2={layout.pts[b].x} y2={layout.pts[b].y} />
          ))}
        </svg>
        {cells.map((ch, i) => {
          const p = layout.pts[i];
          const isSel = i === sel;
          const isTarget = targets.has(i);
          const isTiger = ch === 'T';
          const stateWord = isTiger ? t('tiger') : ch === 'L' ? t('leopard') : t('empty');
          // The tiger is drawn larger (and gold, --arc-p2) than the leopard dots
          // (purple-red, --arc-p1) so the one hunter reads instantly from the six.
          const sz = isTiger ? 15 : 11;
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
                fontSize: isTiger ? 17 : 13,
                lineHeight: 1,
                background: isTiger ? 'var(--arc-p2)' : ch === 'L' ? 'var(--arc-p1)' : 'var(--arc-soft)',
                border: ch === ' ' ? '2.5px solid var(--arc-soft)' : undefined,
                boxShadow: isTiger ? '0 0 0 2px var(--arc-p2)' : undefined,
              }}
              aria-label={t('Point {n}: {state}', { n: i + 1, state: stateWord })}
            >
              {isTiger ? '🐅' : ch === 'L' ? '🐆' : ''}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const lenChoa: GameDefinition = {
  id: 'len-choa',
  name: 'Len Choa',
  icon: '🐅',
  blurb: 'A Thai tiger hunt: six leopards try to corner one tiger before it eats three of them.',
  rules:
    'Leopards move first. While leopards remain in hand, the leopard player drops one on any empty point; once all six are placed, a leopard steps one point along a line to an empty neighbour. The tiger (starting on the apex) steps along a line too, or leaps a single adjacent leopard — landing on the empty point just beyond — to eat it. The leopards win by surrounding the tiger so it cannot move; the tiger wins by eating three leopards.',
  // Asymmetric hunt on a 10-point board; calibrated via self-play
  // (scripts/calibrate.sh len-choa). Seat-0 (leopards) win rate ~37% at equal
  // strength — the tiger is the stronger side, as in the traditional game.
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  defaultParams: { numPlayers: 2 },
  presets: [{ label: 'Classic', emoji: '⭐', params: { numPlayers: 2 } }],
  knobs: [],
  create: (wasm) => makeHandle(wasm),
  Board: LenChoaBoard,
  playerLabels: ['Leopards', 'Tiger'],
  // Both endings are box-in style (no winning line), so tell each story: the
  // leopard win is the tiger cornered; the tiger win is three leopards eaten.
  resultFlavor: ({ result, board, labels, seats, t }) => {
    if (result === 'Draw') return undefined;
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner) || winner < 0) return undefined;
    const captured = Number((board.split('|')[1] ?? '').split(',')[1] ?? 0);
    const name = (seat: number) => (labels[seat] ? t(labels[seat]) : t('Player {n}', { n: seat + 1 }));
    const humans = seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
    const lone = humans.length === 1 ? humans[0] : null;
    // Tiger (seat 1) win — only ever by eating three leopards.
    if (winner === 1 && captured >= 3) {
      if (lone === 1) return t('🐅 Feast! You ate three leopards — you win! 🎉');
      if (lone === 0) return t('🐅 Feast! The tiger ate three leopards — you lose.');
      return t('🐅 Feast! The tiger ate three leopards — {winner} wins!', { winner: name(1) });
    }
    // Leopard (seat 0) win — the tiger is cornered and cannot move.
    if (winner === 0) {
      if (lone === 0) return t('🐆 Cornered! The tiger can’t move — you win! 🎉');
      if (lone === 1) return t('🐆 Cornered! Your tiger can’t move — you lose.');
      return t('🐆 Cornered! The tiger can’t move — {winner} win.', { winner: name(0) });
    }
    return undefined;
  },
};
