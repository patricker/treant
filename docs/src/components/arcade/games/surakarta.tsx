import { useEffect, useRef, useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// Surakarta ("Roundabouts") — 6×6 points on the grid LINES, with the eight
// corner loops drawn as two nested rounded circuits hugging the board. The
// signature mechanic is the arc capture: a piece slides along its row/column,
// AROUND at least one corner loop, and lands on the first enemy it meets. The
// engine (SurakartaWasm) owns all legality; the UI never re-derives geometry.
//
// The star of the show is the capture ANIMATION: the moving piece travels the
// engine's `canonical_arc(from,to)` path (fetched BEFORE the move is applied,
// while the pre-move board still makes the route legal). A teleport would read
// as magic; showing the road the piece drives is what makes "you can take that
// piece from across the board" legible to a child.

// ---------------------------------------------------------------------------
// Board geometry (all in SVG user units; a padded viewBox leaves room for the
// loops that bulge outside the grid). Points sit at ORIGIN + n*STEP; the loops
// are 3/4 circles concentric at each corner (inner r=14, outer r=26).
// ---------------------------------------------------------------------------
const N = 6;
const STEP = 12;
const ORIGIN = 20; // point (r,c) -> (ORIGIN + c*STEP, ORIGIN + r*STEP), i.e. 20..80
const VIEW_MIN = -14;
const VIEW_SIZE = 128; // viewBox "-14 -14 128 128" -> spans -14..114
const VIEW_BOX = `${VIEW_MIN} ${VIEW_MIN} ${VIEW_SIZE} ${VIEW_SIZE}`;

type Pt = { x: number; y: number };
const ptOf = (idx: number): Pt => ({ x: ORIGIN + (idx % N) * STEP, y: ORIGIN + Math.floor(idx / N) * STEP });
// SVG user coord -> percentage of the (viewBox-sized) container, for the
// absolutely-positioned point buttons layered over the SVG.
const pct = (v: number): number => ((v - VIEW_MIN) / VIEW_SIZE) * 100;

// The four corner loop centres (2 units diagonally outside each corner point).
const CORNER = {
  TL: { x: 18, y: 18 },
  TR: { x: 82, y: 18 },
  BL: { x: 18, y: 82 },
  BR: { x: 82, y: 82 },
} as const;

// The eight corner arcs (from the engine's ARCS table): each connects two
// board points via a corner loop. Inner circuit r=14 (lines 1-in), outer r=26
// (lines 2-in). Endpoints are flat indices idx = r*6 + c.
type Arc = { a: number; b: number; c: Pt; r: number };
const ARCS: Arc[] = [
  // inner circuit (rows/cols 1 & 4)
  { a: 6, b: 1, c: CORNER.TL, r: 14 }, // (1,0) <-> (0,1)
  { a: 11, b: 4, c: CORNER.TR, r: 14 }, // (1,5) <-> (0,4)
  { a: 29, b: 34, c: CORNER.BR, r: 14 }, // (4,5) <-> (5,4)
  { a: 24, b: 31, c: CORNER.BL, r: 14 }, // (4,0) <-> (5,1)
  // outer circuit (rows/cols 2 & 3)
  { a: 12, b: 2, c: CORNER.TL, r: 26 }, // (2,0) <-> (0,2)
  { a: 17, b: 3, c: CORNER.TR, r: 26 }, // (2,5) <-> (0,3)
  { a: 23, b: 33, c: CORNER.BR, r: 26 }, // (3,5) <-> (5,3)
  { a: 18, b: 32, c: CORNER.BL, r: 26 }, // (3,0) <-> (5,2)
];
const arcFor = (u: number, v: number): Arc | undefined =>
  ARCS.find((k) => (k.a === u && k.b === v) || (k.a === v && k.b === u));

// The tangent point where the grid-line extension from board point `idx` meets
// its corner loop: a point on a vertical edge (col 0/5) exits horizontally, so
// the tangent shares the circle's x; a point on a horizontal edge exits
// vertically, sharing the circle's y.
function tangent(idx: number, c: Pt): Pt {
  const col = idx % N;
  const p = ptOf(idx);
  return col === 0 || col === N - 1 ? { x: c.x, y: p.y } : { x: p.x, y: c.y };
}

// Sample the 3/4 (270°) loop from tangent Ta to tangent Tb around centre `c`,
// taking the way that goes OUTSIDE the board (the direction that lands on Tb
// after 270°). Returns intermediate points from Ta to Tb inclusive.
function sampleLoop(c: Pt, ta: Pt, tb: Pt, r: number): Pt[] {
  const aA = Math.atan2(ta.y - c.y, ta.x - c.x);
  const aB = Math.atan2(tb.y - c.y, tb.x - c.x);
  const norm = (x: number) => {
    let v = x;
    while (v > Math.PI) v -= 2 * Math.PI;
    while (v < -Math.PI) v += 2 * Math.PI;
    return v;
  };
  const sweep = (3 * Math.PI) / 2; // 270°
  // Exactly one direction (±) travels 270° from aA and lands on aB.
  const dir = Math.abs(norm(aA + sweep - aB)) < 1e-4 ? 1 : -1;
  const steps = 20;
  const out: Pt[] = [];
  for (let i = 0; i <= steps; i++) {
    const a = aA + dir * sweep * (i / steps);
    out.push({ x: c.x + r * Math.cos(a), y: c.y + r * Math.sin(a) });
  }
  return out;
}

// Full pixel-point list for one corner-loop jump A -> B (grid point to grid
// point, out along the extension, around the loop, back in). Used for both the
// drawn circuits and the capture animation.
function loopPoints(a: number, b: number, arc: Arc): Pt[] {
  const ta = tangent(a, arc.c);
  const tb = tangent(b, arc.c);
  return [ptOf(a), ...sampleLoop(arc.c, ta, tb, arc.r), ptOf(b)];
}

const dOf = (pts: Pt[]): string =>
  pts.map((p, i) => `${i === 0 ? 'M' : 'L'}${p.x.toFixed(2)} ${p.y.toFixed(2)}`).join(' ');

// The eight circuit paths, drawn once as board decoration.
const CIRCUIT_PATHS: string[] = ARCS.map((arc) => dOf(loopPoints(arc.a, arc.b, arc)));

// Build the full animation path for a capture, given the engine's arc as a list
// of flat point indices. Consecutive orthogonally-adjacent points are a straight
// step; a non-adjacent pair is a corner-loop jump (traced along its loop).
function capturePath(arc: number[]): Pt[] {
  if (arc.length < 2) return [];
  const pts: Pt[] = [ptOf(arc[0])];
  for (let i = 1; i < arc.length; i++) {
    const u = arc[i - 1];
    const v = arc[i];
    const dr = Math.abs(Math.floor(u / N) - Math.floor(v / N));
    const dc = Math.abs((u % N) - (v % N));
    if (dr + dc === 1) {
      pts.push(ptOf(v)); // orthogonal single step along a grid line
    } else {
      const k = arcFor(u, v);
      if (k) pts.push(...loopPoints(u, v, k).slice(1));
      else pts.push(ptOf(v)); // defensive: unknown pair, straight line
    }
  }
  return pts;
}

// ---------------------------------------------------------------------------
// Handle: wraps SurakartaWasm and, on every capture (human OR AI), captures the
// canonical arc from the PRE-move board (once applied, the route is no longer a
// capture). Stashed at module scope so the pure Board can pick it up when the
// board string changes. Only one Surakarta game is ever live at a time.
// ---------------------------------------------------------------------------
type CaptureAnim = { to: number; path: Pt[]; mover: string; seq: number };
let pendingCapture: CaptureAnim | null = null;
let captureSeq = 0;

function makeHandle(wasm: any): GameHandle {
  const g = new wasm.SurakartaWasm();
  const base = moveHandle(g);
  pendingCapture = null;
  return {
    ...base,
    applyMove: (m: string) => {
      const dash = m.indexOf('-');
      const from = Number(m.slice(0, dash));
      const to = Number(m.slice(dash + 1));
      const board = g.get_board();
      const isCapture = dash > 0 && board[to] != null && board[to] !== ' ';
      const mover = dash > 0 ? board[from] : ' ';
      let arcPath: Pt[] = [];
      if (isCapture) {
        const s: string = g.canonical_arc(from, to);
        const arc = s ? s.split(',').map(Number) : [];
        arcPath = capturePath(arc);
      }
      const ok = base.applyMove(m);
      if (ok && isCapture && arcPath.length >= 2) {
        pendingCapture = { to, path: arcPath, mover, seq: ++captureSeq };
      }
      return ok;
    },
  };
}

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------
const CAPTURE_MS = 600;
const colorOf = (ch: string): string => (ch === 'X' ? 'var(--arc-p1)' : 'var(--arc-p2)');

function reduceMotion(): boolean {
  return typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

function SurakartaBoard({ board, interactive, legalMoves, lastCells, playerNames, onMove }: BoardProps) {
  const { t } = useT();
  const [sel, setSel] = useState<number | null>(null);
  const [anim, setAnim] = useState<CaptureAnim | null>(null);
  const seenSeq = useRef(0);
  const prevCount = useRef(Number.POSITIVE_INFINITY);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => setSel(null), [board]); // drop stale selection after any move

  // Pick up a capture stashed by the handle when the board string changes.
  // Only a real capture strictly REDUCES the piece count; an undo replays the
  // kept move prefix (re-arming module-scope `pendingCapture` with a fresh seq)
  // and lands on an earlier, fuller board — piece count jumps back UP. Gating on
  // "count went down" suppresses that phantom fly without touching the session.
  useEffect(() => {
    let count = 0;
    for (let i = 0; i < board.length; i++) if (board[i] !== ' ') count++;
    const captured = count < prevCount.current; // a piece just vanished
    prevCount.current = count;
    const pc = pendingCapture;
    if (captured && pc && pc.seq > seenSeq.current && board[pc.to] === pc.mover) {
      seenSeq.current = pc.seq;
      setAnim(pc);
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => setAnim(null), CAPTURE_MS + 40);
    }
    return () => window.clearTimeout(timer.current);
  }, [board]);

  const cells = Array.from({ length: N * N }, (_, i) => board[i] ?? ' ');
  const last = new Set(lastCells ?? []);

  // Legal "from -> to" targets for the tapped piece (empty steps + arc captures).
  const fromTo = new Map<number, Set<number>>();
  for (const m of legalMoves) {
    const dash = m.indexOf('-');
    if (dash < 0) continue;
    const f = Number(m.slice(0, dash));
    const to = Number(m.slice(dash + 1));
    if (!fromTo.has(f)) fromTo.set(f, new Set());
    fromTo.get(f)!.add(to);
  }
  const targets = sel != null ? (fromTo.get(sel) ?? new Set<number>()) : new Set<number>();

  const tap = (i: number) => {
    if (!interactive) return;
    if (sel != null && targets.has(i)) {
      onMove(`${sel}-${i}`);
      setSel(null);
      return;
    }
    if (fromTo.has(i)) {
      setSel(i !== sel ? i : null);
      return;
    }
    setSel(null);
  };

  const reduced = reduceMotion();
  const showFly = anim != null && !reduced;
  const showPath = anim != null && reduced;
  const seatName = (ch: string) => (playerNames?.[ch === 'X' ? 0 : 1] ?? (ch === 'X' ? 'Red' : 'Gold'));

  return (
    <div>
      <div className={styles.shiftCaption}>
        {sel == null ? t('Tap a piece to move') : t('Tap where to move')}
      </div>
      <div className={`${styles.mtWrap} ${styles.surWrap}`}>
        {/* board lines + the eight corner circuits */}
        <svg viewBox={VIEW_BOX} className={styles.mtLines} preserveAspectRatio="xMidYMid meet">
          {Array.from({ length: N }, (_, r) => (
            <line key={`h${r}`} x1={ORIGIN} y1={ORIGIN + r * STEP} x2={ORIGIN + (N - 1) * STEP} y2={ORIGIN + r * STEP} />
          ))}
          {Array.from({ length: N }, (_, c) => (
            <line key={`v${c}`} x1={ORIGIN + c * STEP} y1={ORIGIN} x2={ORIGIN + c * STEP} y2={ORIGIN + (N - 1) * STEP} />
          ))}
          {CIRCUIT_PATHS.map((d, i) => (
            <path key={`arc${i}`} d={d} className={styles.surCircuit} />
          ))}
        </svg>

        {/* point pieces */}
        {cells.map((ch, i) => {
          const p = ptOf(i);
          const isSel = i === sel;
          const isTarget = targets.has(i);
          const isCapTarget = isTarget && ch !== ' ';
          const hidden = showFly && anim!.to === i; // destination piece flies in
          const state = ch === 'X' ? seatName('X') : ch === 'O' ? seatName('O') : t('empty');
          return (
            <button
              key={i}
              className={`${styles.mtCell} ${styles.surCell} ${isSel ? styles.shiftSel : ''} ${
                isCapTarget ? styles.surCapTarget : isTarget ? styles.moveTarget : ''
              } ${last.has(i) ? styles.lastCell : ''}`}
              disabled={!interactive || !(fromTo.has(i) || isTarget)}
              onClick={() => tap(i)}
              style={{
                left: `${pct(p.x)}%`,
                top: `${pct(p.y)}%`,
                background: ch === ' ' ? 'var(--arc-soft)' : colorOf(ch),
                borderColor: ch === ' ' ? 'var(--arc-soft)' : 'rgba(0,0,0,0.28)',
                opacity: hidden ? 0 : 1,
              }}
              aria-label={t('Point {n}: {state}', { n: i + 1, state })}
            >
              {isTarget && ch === ' ' ? <span className={styles.moveDot} /> : ''}
            </button>
          );
        })}

        {/* capture animation overlay (same viewBox, so coords match the board) */}
        {anim != null && (
          <svg viewBox={VIEW_BOX} className={styles.surOverlay} preserveAspectRatio="xMidYMid meet" aria-hidden>
            {showPath && (
              // reduced motion: flash the road instead of moving the piece
              <path key={`p${anim.seq}`} d={dOf(anim.path)} data-capture-path="1" className={styles.surReducedPath} />
            )}
            {showFly && (
              <g key={`f${anim.seq}`} data-capture-anim="1">
                {/* CSS offset-path drives the motion (SMIL animateMotion begins
                    on the document timeline, so a mid-life insert would freeze
                    at the end — a teleport). offset-path starts on insertion. */}
                <circle
                  r={4.6}
                  fill={colorOf(anim.mover)}
                  stroke="rgba(0,0,0,0.28)"
                  strokeWidth={0.8}
                  className={styles.surFly}
                  style={{ offsetPath: `path('${dOf(anim.path)}')`, offsetRotate: '0deg' } as Record<string, string>}
                />
              </g>
            )}
          </svg>
        )}
      </div>
    </div>
  );
}

export const surakarta: GameDefinition = {
  id: 'surakarta',
  name: 'Surakarta',
  icon: '🌀',
  blurb: 'Slide a piece around the corner loops to snatch an enemy from across the board. Capture all twelve to win.',
  rules:
    'Each turn, step one piece to any empty neighbouring point (including diagonally), OR capture: slide a piece along its row or column, around at least one of the eight corner loops, and land on the first enemy piece it reaches — every point along the way must be empty. You win by capturing all twelve of the opponent’s pieces. If forty moves pass with no capture, the game is a draw.',
  // Calibrated by the self-play harness (treant-wasm/examples/calibrate.rs):
  // monotone ladder, still climbing at the top, so Hard is the max rung.
  difficulty: {
    // Easy promoted p8→p30 (the next measured rung: STD ladder {30, top_k 5,
    // temp 2}, 18% field on the 6-3 matrix). p8/t3 empirically never captured in
    // self-play — a guaranteed 40-ply pacifist draw that hid the signature arc
    // capture. p30 still sits clearly below Medium (p100) on the monotone ladder.
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2 },
  // The board (fixed 6×6) and the 40-ply draw cap are engine constants — no
  // knobs to expose, so this ships Classic-only (the Len Choa precedent).
  presets: [{ label: 'Classic', emoji: '⭐', params: { numPlayers: 2 } }],
  knobs: [],
  create: (wasm) => makeHandle(wasm),
  Board: SurakartaBoard,
  playerLabels: ['Red', 'Gold'],
  // No winning line lights up (a capture-all or a quiet draw), so narrate the
  // ending: the board was swept, or forty quiet moves ran out the clock.
  resultFlavor: ({ result, labels, seats, t }) => {
    const name = (seat: number) => (labels[seat] ? t(labels[seat]) : t('Player {n}', { n: seat + 1 }));
    const humans = seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
    const lone = humans.length === 1 ? humans[0] : null;
    if (result === 'Draw') {
      return t('🤝 Forty moves with no capture — it’s a draw.');
    }
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner) || winner < 0) return undefined;
    const loser = winner === 0 ? 1 : 0;
    if (lone === winner) return t('🏵️ You swept the board — you win! 🎉');
    if (lone === loser) return t('🏵️ {winner} swept the board — you lose.', { winner: name(winner) });
    return t('🏵️ {winner} swept the board!', { winner: name(winner) });
  },
};
