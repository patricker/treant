import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// The type of the resultFlavor callback's single context argument.
type FlavorCtx = Parameters<NonNullable<GameDefinition['resultFlavor']>>[0];

// A draughts piece: a round disc coloured by seat, with a gold crown for kings.
// Board chars are the arcade convention: 'x'/'X' = seat-0 man/king, 'o'/'O' =
// seat-1 man/king (UPPERCASE = crowned).
function DraughtPiece({ ch }: { ch: string }) {
  const seat0 = ch === 'x' || ch === 'X';
  const king = ch === 'X' || ch === 'O';
  const color = seat0 ? 'var(--arc-p1)' : 'var(--arc-p2)';
  return (
    <svg viewBox="0 0 24 24" width="80%" height="80%" style={{ display: 'block' }} aria-hidden>
      <circle cx="12" cy="12" r="9" fill={color} stroke="rgba(0,0,0,0.3)" strokeWidth="0.9" />
      <circle cx="12" cy="12" r="6.2" fill="none" stroke="rgba(0,0,0,0.18)" strokeWidth="0.8" />
      {king && (
        <path
          d="M6.5 14.5l-1-5 3.2 2.2L12 7l3.3 4.7 3.2-2.2-1 5z"
          fill="#ffd23f"
          stroke="rgba(0,0,0,0.4)"
          strokeWidth="0.6"
          strokeLinejoin="round"
        />
      )}
    </svg>
  );
}

// Checkerboard board with step-by-step forced-capture CHAIN input. A turn is
// built one landing square at a time: tap a piece, then tap each legal landing;
// when the chain can go no further the full dash-joined path ("12-19-26") is
// submitted atomically. Captures are compulsory, so whenever any jump exists the
// engine only offers capturing pieces — the UI enforces nothing itself. Kings
// wear a crown; the whole chain (origin, every captured square, the landing) is
// ringed as the last move via lastCells. Precedent: Strand's two-phase board.
function DraughtsBoard({ board, params, interactive, legalMoves, lastCells, playerNames, onMove }: BoardProps) {
  const { t } = useT();
  // Giveaway (misère) looks identical to normal draughts, so a fixed chip keeps
  // the inverted objective in view on every turn — not just during selection.
  const misere = !!params?.misere;
  const n = Math.max(1, Math.round(Math.sqrt(board.length)));
  const cells = Array.from({ length: n * n }, (_, i) => board[i] ?? ' ');
  const last = new Set(lastCells ?? []);

  // The chain being built, as cell indices; path[0] is the selected piece.
  const [path, setPath] = useState<number[]>([]);
  // Drop the in-progress chain whenever the board changes (move committed, undo,
  // or the opponent replied) so a stale half-chain never lingers.
  useEffect(() => setPath([]), [board]);

  // Parse the engine's legal moves into index paths. No returned move is a prefix
  // of another (captures are MAXIMAL chains), so a completed prefix is unambiguous.
  const moves = legalMoves.filter(Boolean).map((m) => m.split('-').map(Number));
  const starts = new Set(moves.map((mv) => mv[0]));

  // Moves whose path still matches the chain built so far (prefix match).
  const matching = path.length
    ? moves.filter((mv) => mv.length >= path.length && path.every((p, k) => mv[k] === p))
    : [];
  // The distinct next landing squares that extend the current chain.
  const nextCells = new Set<number>();
  for (const mv of matching) if (mv.length > path.length) nextCells.add(mv[path.length]);

  // Forced-capture affordance. Captures are compulsory, so when a jump exists the
  // engine offers ONLY jumping pieces — every other piece the mover owns is
  // silently disabled. Detect that state so the tappable pieces can be ringed and
  // the caption can say "you must capture".
  //
  // The mover's seat is read off any movable piece and single-sourced here, so
  // both the capture test and the owned-piece count agree on whose pieces are
  // whose ('x'/'X' = seat-0, 'o'/'O' = seat-1; UPPERCASE = king).
  let moverSeat0 = true;
  if (starts.size) {
    const firstCh = cells[moves[0][0]];
    moverSeat0 = firstCh === 'x' || firstCh === 'X';
  }
  const isEnemy = (ch: string) =>
    moverSeat0 ? ch === 'o' || ch === 'O' : ch === 'x' || ch === 'X';
  // A move is a capture iff one of its diagonal steps slides over an enemy piece.
  // Determined by BOARD CONTENT, not distance: a man jump has the enemy at the
  // step's midpoint; a flying-king capture has the enemy somewhere along the slid
  // diagonal — while a flying king's QUIET slide (same ≥2-row span) crosses only
  // empty squares. A distance heuristic can't tell those apart; walking the
  // in-between cells can. Any capturing segment in a chain flags the whole move.
  const isCapture = (mv: number[]) => {
    for (let k = 0; k + 1 < mv.length; k++) {
      const ra = Math.floor(mv[k] / n), ca = mv[k] % n;
      const rb = Math.floor(mv[k + 1] / n), cb = mv[k + 1] % n;
      const dr = Math.sign(rb - ra), dc = Math.sign(cb - ca);
      for (let r = ra + dr, c = ca + dc; r !== rb || c !== cb; r += dr, c += dc) {
        if (isEnemy(cells[r * n + c])) return true;
      }
    }
    return false;
  };
  const anyCapture = moves.some(isCapture);
  // Count the mover's pieces to know whether the legal-to-move set is a strict
  // subset — i.e. force-capture is hiding pieces.
  let ownedCount = 0;
  if (starts.size) {
    for (const ch of cells) {
      const s0 = ch === 'x' || ch === 'X';
      const s1 = ch === 'o' || ch === 'O';
      if ((moverSeat0 && s0) || (!moverSeat0 && s1)) ownedCount++;
    }
  }
  const forced = interactive && anyCapture && starts.size > 0 && starts.size < ownedCount;

  const tap = (i: number) => {
    if (!interactive) return;
    if (path.length === 0) {
      if (starts.has(i)) setPath([i]);
      return;
    }
    // Tapping the origin piece cancels the whole in-progress chain.
    if (i === path[0]) {
      setPath([]);
      return;
    }
    if (!nextCells.has(i)) return;
    const next = [...path, i];
    // Does any legal move extend beyond this landing? If so keep building the
    // chain; otherwise the chain is complete — submit the full path atomically.
    const extendable = moves.some((mv) => mv.length > next.length && next.every((p, k) => mv[k] === p));
    if (extendable) setPath(next);
    else {
      onMove(next.join('-'));
      setPath([]);
    }
  };

  const caption =
    path.length === 0
      ? forced
        ? t('You must capture — tap a highlighted piece')
        : t('Tap a piece')
      : path.length === 1
        ? t('Tap where to move')
        : t('Keep jumping — tap the next square');

  const pieceName = (ch: string) =>
    ch === ' '
      ? t('empty')
      : ch === 'x' || ch === 'X'
        ? playerNames?.[0] ?? ch
        : playerNames?.[1] ?? ch;

  const inChain = path.slice(1); // landing squares chosen so far (trail markers)

  return (
    <div>
      {misere && <div className={styles.draughtsGoalChip}>{t('🙃 Goal: lose everything!')}</div>}
      <div className={styles.shiftCaption}>{caption}</div>
      <div
        className={styles.draughtsGrid}
        style={{ gridTemplateColumns: `repeat(${n}, minmax(0, 1fr))` }}
      >
        {cells.map((ch, i) => {
          const r = Math.floor(i / n);
          const c = i % n;
          const dark = (r + c) % 2 === 1; // playing squares are the dark squares
          const isSel = path[0] === i;
          const isNext = interactive && nextCells.has(i);
          const isTrail = inChain.includes(i);
          const tappable = interactive && (path.length === 0 ? starts.has(i) : isNext || i === path[0]);
          // Ring the pieces that can still move only while nothing is selected yet
          // (path empty), so this affordance never overlaps the selected/target rings.
          const isForced = forced && path.length === 0 && starts.has(i);
          const cls = [
            styles.draughtsCell,
            dark ? styles.draughtsDark : styles.draughtsLight,
            isSel ? styles.draughtsSel : '',
            isNext ? styles.draughtsTarget : '',
            isForced ? styles.draughtsForced : '',
            last.has(i) ? styles.lastCell : '',
          ]
            .filter(Boolean)
            .join(' ');
          return (
            <button
              key={i}
              className={cls}
              disabled={!tappable}
              onClick={() => tap(i)}
              aria-label={t('Cell {n}: {state}', { n: i + 1, state: pieceName(ch) })}
            >
              {ch !== ' ' ? (
                <DraughtPiece ch={ch} />
              ) : isNext ? (
                <span className={styles.draughtsDot} />
              ) : isTrail ? (
                <span className={styles.draughtsTrailDot} />
              ) : null}
            </button>
          );
        })}
      </div>
      {interactive && path.length >= 1 && (
        <button
          type="button"
          className={styles.draughtsCancel}
          onClick={() => setPath([])}
        >
          {path.length >= 2 ? t('✕ Cancel this jump') : t('✕ Deselect')}
        </button>
      )}
    </div>
  );
}

// Narrate the (glow-less) blocked / wiped-out ending. `misere` inverts the
// framing for Giveaway: there the player who ran out of pieces or moves WINS.
function draughtsFlavor(misere: boolean): GameDefinition['resultFlavor'] {
  return ({ result, labels, seats, t }: FlavorCtx) => {
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner) || winner < 0) return undefined; // draw → default banner
    const loser = winner === 0 ? 1 : 0;
    const name = (s: number) => (labels[s] ? t(labels[s]) : t('Player {n}', { n: s + 1 }));
    const humans = seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
    const lone = humans.length === 1 ? humans[0] : -1;
    if (misere) {
      if (lone === winner) return t('🙃 You gave everything away first — you win! 🎉');
      if (lone === loser) return t('🙃 {winner} gave everything away first — you lose.', { winner: name(winner) });
      return t('🙃 {winner} gave everything away first!', { winner: name(winner) });
    }
    if (lone === loser) return t('No moves left — {winner} wins.', { winner: name(winner) });
    if (lone === winner) return t('{loser} has no moves left — you win! 🎉', { loser: name(loser) });
    return t('No moves — {winner} wins.', { winner: name(winner) });
  };
}

// ─────────────────────────────── Parent tile ───────────────────────────────
// American / English draughts is BOTH the family parent and a real variant: the
// familiar 8×8 checkers with forced (but free-choice) captures and step-1,
// non-flying kings. This is the "go crazy" surface — it exposes EVERY rule flag
// so a curious player can build any draughts they like (flip on flying kings,
// backward jumps, max-capture, mid-chain crowning, the giveaway win condition,
// grow the board to 10×10 / 12×12, or — the wave-B additions — forbid men from
// jumping kings and pick a Spanish/Italian capture priority). Ctor: (size,
// men_rows, flying, men_back, max_capture, promote_mid, misere,
// men_cannot_capture_kings, capture_priority).
export const draughts: GameDefinition = {
  id: 'draughts',
  name: 'Draughts',
  icon: '⛀',
  blurb: 'Classic checkers: march your men, jump diagonally — jumps are forced — and crown a king at the far row.',
  // Solver-off (long, cycle-prone games); strength is a real material+mobility
  // evaluator. Hard = max rung per CALIBRATION.md policy (the self-play plateau
  // measures relative self-play strength, not absolute strength vs humans).
  // Easy/Medium measured via scripts/calibrate.sh draughts 12 on 2026-07-06
  // (field 3→18→58→74→72→75; seat-0 47%).
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, size: 8, men_rows: 3, flying: 0, men_back: 0, max_capture: 0, promote_mid: 0, misere: 0, men_no_king: 0, capture_priority: 0 },
  presets: [
    { label: 'American ⭐ 8×8', emoji: '⭐', params: { numPlayers: 2, size: 8, men_rows: 3, flying: 0, men_back: 0, max_capture: 0, promote_mid: 0, misere: 0 } },
    { label: 'Flying Kings', emoji: '👑', params: { numPlayers: 2, size: 8, men_rows: 3, flying: 1, men_back: 1, max_capture: 0, promote_mid: 0, misere: 0 } },
    { label: 'Big Board 10×10', emoji: '🔲', params: { numPlayers: 2, size: 10, men_rows: 4, flying: 1, men_back: 1, max_capture: 1, promote_mid: 0, misere: 0 } },
    { label: 'Anything Goes 12×12', emoji: '🤯', params: { numPlayers: 2, size: 12, men_rows: 4, flying: 1, men_back: 1, max_capture: 1, promote_mid: 1, misere: 0 } },
  ],
  knobs: [
    { key: 'size', label: 'Board size', min: 8, max: 12, step: 2 },
    { key: 'men_rows', label: 'Rows of men', min: 2, max: 4, step: 1 },
    { key: 'flying', label: 'Flying kings', min: 0, max: 1, step: 1, valueLabels: ['No', 'Yes'] },
    { key: 'men_back', label: 'Men jump back', min: 0, max: 1, step: 1, valueLabels: ['No', 'Yes'] },
    { key: 'max_capture', label: 'Force max capture', min: 0, max: 1, step: 1, valueLabels: ['No', 'Yes'] },
    { key: 'promote_mid', label: 'Crown mid-jump', min: 0, max: 1, step: 1, valueLabels: ['No', 'Yes'] },
    { key: 'misere', label: 'Giveaway (misère)', min: 0, max: 1, step: 1, valueLabels: ['No', 'Yes'] },
    { key: 'men_no_king', label: 'Men can\'t jump kings', min: 0, max: 1, step: 1, valueLabels: ['No', 'Yes'], help: 'Italian rule: a plain man may never capture a king.' },
    { key: 'capture_priority', label: 'Capture priority', min: 0, max: 2, step: 1, valueLabels: ['None', 'Most kings', 'Italian rules'], help: 'Tie-break between longest captures. Anything but None forces maximum-capture on.' },
  ],
  create: (wasm, p) =>
    moveHandle(new wasm.DraughtsWasm(p.size, p.men_rows, p.flying, p.men_back, p.max_capture, p.promote_mid, p.misere, p.men_no_king ?? 0, p.capture_priority ?? 0)),
  Board: DraughtsBoard,
  playerLabels: ['Red', 'Gold'],
  resultFlavor: draughtsFlavor(false),
};

// ───────────────────────── Variant children (config-only) ─────────────────────
// Each national variant pins its identity flags in the constructor (a 12×12
// "Pool" isn't Pool), so the children expose NO knobs and a single canonical
// preset — no wave-A variant attests an alternate board size, so there is no
// size knob to give. The parent Draughts tile is the go-crazy surface.

export const internationalDraughts: GameDefinition = {
  id: 'international-draughts',
  variantOf: 'draughts',
  name: 'International Draughts',
  icon: '⛀',
  blurb: 'Draughts on a big 10×10 board: flying kings, forward-and-backward jumps, and you must always take the MOST pieces.',
  // Calibrated via scripts/calibrate.sh international-draughts 12 (LIGHT ladder,
  // field 6→19→58→79→88; seat-0 46%). Bigger board keeps climbing → higher Hard.
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 80, topK: 4, temp: 1 },
    hard: { playouts: 500, topK: 2, temp: 0.2 },
  },
  defaultParams: { numPlayers: 2, size: 10, men_rows: 4, flying: 1, men_back: 1, max_capture: 1, promote_mid: 0, misere: 0 },
  presets: [{ label: 'International 10×10', emoji: '⭐', params: { numPlayers: 2, size: 10, men_rows: 4, flying: 1, men_back: 1, max_capture: 1, promote_mid: 0, misere: 0 } }],
  knobs: [],
  create: (wasm) => moveHandle(new wasm.DraughtsWasm(10, 4, 1, 1, 1, 0, 0, 0, 0)),
  Board: DraughtsBoard,
  playerLabels: ['Red', 'Gold'],
  resultFlavor: draughtsFlavor(false),
};

export const brazilianDraughts: GameDefinition = {
  id: 'brazilian-draughts',
  variantOf: 'draughts',
  name: 'Brazilian Draughts',
  icon: '⛀',
  blurb: 'International rules on a compact 8×8 board: flying kings, backward captures, and maximum-capture is forced.',
  // Hard = max rung per CALIBRATION.md policy (the self-play plateau measures
  // relative self-play strength, not absolute strength vs humans). Easy/Medium
  // measured via scripts/calibrate.sh brazilian-draughts 12 on 2026-07-06
  // (field 5→15→47→70→79→84; seat-0 49%).
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, size: 8, men_rows: 3, flying: 1, men_back: 1, max_capture: 1, promote_mid: 0, misere: 0 },
  presets: [{ label: 'Brazilian 8×8', emoji: '⭐', params: { numPlayers: 2, size: 8, men_rows: 3, flying: 1, men_back: 1, max_capture: 1, promote_mid: 0, misere: 0 } }],
  knobs: [],
  create: (wasm) => moveHandle(new wasm.DraughtsWasm(8, 3, 1, 1, 1, 0, 0, 0, 0)),
  Board: DraughtsBoard,
  playerLabels: ['Red', 'Gold'],
  resultFlavor: draughtsFlavor(false),
};

export const poolCheckers: GameDefinition = {
  id: 'pool-checkers',
  variantOf: 'draughts',
  name: 'Pool Checkers',
  icon: '⛀',
  blurb: '8×8 checkers with flying kings and backward jumps — but take ANY capture you like, not necessarily the longest.',
  // Hard = max rung per CALIBRATION.md policy (the self-play plateau measures
  // relative self-play strength, not absolute strength vs humans). Easy/Medium
  // measured via scripts/calibrate.sh pool-checkers 12 on 2026-07-06
  // (field 8→12→44→73→79→83; seat-0 47%).
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, size: 8, men_rows: 3, flying: 1, men_back: 1, max_capture: 0, promote_mid: 0, misere: 0 },
  presets: [{ label: 'Pool 8×8', emoji: '⭐', params: { numPlayers: 2, size: 8, men_rows: 3, flying: 1, men_back: 1, max_capture: 0, promote_mid: 0, misere: 0 } }],
  knobs: [],
  create: (wasm) => moveHandle(new wasm.DraughtsWasm(8, 3, 1, 1, 0, 0, 0, 0, 0)),
  Board: DraughtsBoard,
  playerLabels: ['Red', 'Gold'],
  resultFlavor: draughtsFlavor(false),
};

export const russianDraughts: GameDefinition = {
  id: 'russian-draughts',
  variantOf: 'draughts',
  name: 'Russian Draughts',
  icon: '⛀',
  blurb: '8×8 with flying kings and backward jumps — and a man that reaches the back row mid-jump is crowned at once and keeps going as a king.',
  // Hard = max rung per CALIBRATION.md policy (the self-play plateau measures
  // relative self-play strength, not absolute strength vs humans). Easy/Medium
  // measured via scripts/calibrate.sh russian-draughts 12 on 2026-07-06
  // (field 0→20→45→76→78→81; seat-0 49%).
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, size: 8, men_rows: 3, flying: 1, men_back: 1, max_capture: 0, promote_mid: 1, misere: 0 },
  presets: [{ label: 'Russian 8×8', emoji: '⭐', params: { numPlayers: 2, size: 8, men_rows: 3, flying: 1, men_back: 1, max_capture: 0, promote_mid: 1, misere: 0 } }],
  knobs: [],
  create: (wasm) => moveHandle(new wasm.DraughtsWasm(8, 3, 1, 1, 0, 1, 0, 0, 0)),
  Board: DraughtsBoard,
  playerLabels: ['Red', 'Gold'],
  resultFlavor: draughtsFlavor(false),
};

export const giveawayCheckers: GameDefinition = {
  id: 'giveaway-checkers',
  variantOf: 'draughts',
  name: 'Giveaway Checkers',
  icon: '🙃',
  // American-based framing (matches the shipped flags: no flying, no backward,
  // no max-capture). The engine header notes a Russian-based alternative; user
  // text never mentions it.
  blurb: 'Checkers upside-down: the first player left with no move — because they gave every piece away, or got boxed in — WINS.',
  // Misère flips who is favoured, but the ladder is NON-DEGENERATE: the material
  // sign-flip means deeper search genuinely plays the giveaway better. Hard = max
  // rung per CALIBRATION.md policy (the self-play plateau measures relative
  // self-play strength, not absolute strength vs humans). Easy/Medium measured via
  // scripts/calibrate.sh giveaway-checkers 12 on 2026-07-06 (field 3→17→48→73→78→81
  // monotonic; seat-0 52%).
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, size: 8, men_rows: 3, flying: 0, men_back: 0, max_capture: 0, promote_mid: 0, misere: 1 },
  presets: [{ label: 'Giveaway 8×8', emoji: '🙃', params: { numPlayers: 2, size: 8, men_rows: 3, flying: 0, men_back: 0, max_capture: 0, promote_mid: 0, misere: 1 } }],
  knobs: [],
  create: (wasm) => moveHandle(new wasm.DraughtsWasm(8, 3, 0, 0, 0, 0, 1, 0, 0)),
  Board: DraughtsBoard,
  playerLabels: ['Red', 'Gold'],
  resultFlavor: draughtsFlavor(true),
};

export const spanishDraughts: GameDefinition = {
  id: 'spanish-draughts',
  variantOf: 'draughts',
  name: 'Spanish Draughts',
  icon: '⛀',
  blurb: '8×8 with flying kings but forward-only men — and when captures tie for length you must take the one that grabs the most kings.',
  // Hard = max rung per CALIBRATION.md policy (the self-play plateau measures
  // relative self-play strength, not absolute strength vs humans; harness capped
  // at p300, we ship the max rung). Easy/Medium measured via calibrate example
  // spanish-draughts 20 on 2026-07-11 (field 7→14→47→76→77→79; seat-0 49%).
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, size: 8, men_rows: 3, flying: 1, men_back: 0, max_capture: 1, promote_mid: 0, misere: 0, men_no_king: 0, capture_priority: 1 },
  presets: [{ label: 'Spanish 8×8', emoji: '⭐', params: { numPlayers: 2, size: 8, men_rows: 3, flying: 1, men_back: 0, max_capture: 1, promote_mid: 0, misere: 0, men_no_king: 0, capture_priority: 1 } }],
  knobs: [],
  create: (wasm) => moveHandle(new wasm.DraughtsWasm(8, 3, 1, 0, 1, 0, 0, 0, 1)),
  Board: DraughtsBoard,
  playerLabels: ['Red', 'Gold'],
  resultFlavor: draughtsFlavor(false),
};

export const italianDraughts: GameDefinition = {
  id: 'italian-draughts',
  variantOf: 'draughts',
  name: 'Italian Draughts',
  icon: '⛀',
  blurb: '8×8 with step-at-a-time (non-flying) kings where a plain man may never jump a king — and a strict order for which longest capture you must take.',
  // Hard = max rung per CALIBRATION.md policy (the self-play plateau measures
  // relative self-play strength, not absolute strength vs humans; harness capped
  // at p100 on a flat top, we ship the max rung). Easy/Medium measured via
  // calibrate example italian-draughts 20 on 2026-07-11 (field 8→14→68→70→68→72;
  // seat-0 47%; auto-picked Medium {30,5,2}).
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, size: 8, men_rows: 3, flying: 0, men_back: 0, max_capture: 1, promote_mid: 0, misere: 0, men_no_king: 1, capture_priority: 2 },
  presets: [{ label: 'Italian 8×8', emoji: '⭐', params: { numPlayers: 2, size: 8, men_rows: 3, flying: 0, men_back: 0, max_capture: 1, promote_mid: 0, misere: 0, men_no_king: 1, capture_priority: 2 } }],
  knobs: [],
  create: (wasm) => moveHandle(new wasm.DraughtsWasm(8, 3, 0, 0, 1, 0, 0, 1, 2)),
  Board: DraughtsBoard,
  playerLabels: ['Red', 'Gold'],
  resultFlavor: draughtsFlavor(false),
};
