import { useEffect, useRef, useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT, type I18n } from '../i18n';
import styles from '../arcade.module.css';

// Board string is single-sourced with the Rust engine (vanguard.rs `render`):
//   phase|w|h|scoutLong|current|winner|reason|army|myRemaining|mine|theirs
// `mine`/`theirs` are `;`-joined `cell.glyph.rev.moved.long` tokens. `theirs`
// carries glyph `?` for a still-hidden enemy piece (its rank never appears) and
// the REAL glyph only once combat has publicly revealed it (or at game over) —
// the load-bearing secrecy guarantee (vanguard.rs `wrong_seat_never_sees_hidden_ranks`).
//
// The handle appends `|LM=<summary>` (from the PUBLIC `last_move_summary_for`, identical
// for both seats) so the board can render the combat reveal beat + last-move rings
// inline without any extra plumbing. That suffix is public by construction — it
// carries no hidden rank of an unfought piece.

// ── piece kinds (index order matches the Rust `Kind` enum + ctor args) ─────────
interface KindMeta {
  glyph: string;
  name: string;
  rank: number | null; // combat rank (higher wins); null for the immobile Bomb/Standard
  emoji: string;
}
const KINDS: KindMeta[] = [
  { glyph: 'M', name: 'Marshal', rank: 6, emoji: '🎖️' },
  { glyph: 'C', name: 'Captain', rank: 5, emoji: '🪖' },
  { glyph: 'G', name: 'Sergeant', rank: 4, emoji: '🎗️' },
  { glyph: 'T', name: 'Trooper', rank: 3, emoji: '🔰' },
  { glyph: 'S', name: 'Scout', rank: 2, emoji: '🏃' },
  { glyph: 'P', name: 'Sapper', rank: 1, emoji: '🧰' },
  { glyph: 'Y', name: 'Spy', rank: 0, emoji: '🕵️' },
  { glyph: 'B', name: 'Bomb', rank: null, emoji: '💣' },
  { glyph: 'D', name: 'Standard', rank: null, emoji: '🚩' },
];
const GLYPH_TO_KIND: Record<string, number> = {};
KINDS.forEach((k, i) => (GLYPH_TO_KIND[k.glyph] = i));
const SCOUT_KIND = 4;

// ── parsed board ───────────────────────────────────────────────────────────────
interface VPiece {
  cell: number;
  kind: number | null; // null ⇒ a hidden enemy back of unknown rank
  hidden: boolean;
  revealed: boolean;
  moved: boolean;
  long: boolean; // has made a 2+ straight move ⇒ publicly a Scout
}
type Beat =
  | { t: 'q'; from: number; to: number; glyph: string; long: boolean }
  | { t: 'c'; from: number; to: number; ag: string; ar: string; dg: string; dr: string; outcome: string };
interface Parsed {
  phase: 'place' | 'play' | 'over';
  w: number;
  h: number;
  scoutLong: boolean;
  current: number;
  winner: string;
  reason: string;
  army: number[]; // clamped per-kind counts (Kind order)
  remaining: number[]; // remaining for the viewing seat to place (Kind order)
  mine: VPiece[];
  theirs: VPiece[];
  beat: Beat | null;
}

function parsePieces(s: string, enemy: boolean): VPiece[] {
  if (!s) return [];
  return s.split(';').map((tok) => {
    const [cell, glyph, rev, mv, ml] = tok.split('.');
    const hidden = enemy && glyph === '?';
    const long = ml === '1';
    // moved_long is public and pins the piece to Scout, even while hidden — so we
    // can name it a Scout without a combat reveal (a legitimate public inference).
    const kind = hidden ? (long ? SCOUT_KIND : null) : (GLYPH_TO_KIND[glyph] ?? null);
    return { cell: Number(cell), kind, hidden, revealed: rev === '1', moved: mv === '1', long };
  });
}

function parseBeat(s: string): Beat | null {
  if (!s) return null;
  const p = s.split(':');
  if (p[0] === 'q') return { t: 'q', from: Number(p[1]), to: Number(p[2]), glyph: p[3], long: p[4] === '1' };
  if (p[0] === 'c')
    return { t: 'c', from: Number(p[1]), to: Number(p[2]), ag: p[3], ar: p[4], dg: p[5], dr: p[6], outcome: p[7] };
  return null;
}

function parseBoard(board: string): Parsed {
  const [core, lm] = board.split('|LM=');
  const p = core.split('|');
  const nums = (s: string): number[] => (s ? s.split(',').map(Number) : []);
  return {
    phase: (p[0] as Parsed['phase']) || 'place',
    w: Number(p[1]) || 8,
    h: Number(p[2]) || 8,
    scoutLong: p[3] === '1',
    current: Number(p[4]) || 0,
    winner: p[5] || '',
    reason: p[6] || '',
    army: nums(p[7] || ''),
    remaining: nums(p[8] || ''),
    mine: parsePieces(p[9] || '', false),
    theirs: parsePieces(p[10] || '', true),
    beat: parseBeat(lm || ''),
  };
}

// ── geometry (mirrors vanguard.rs; the engine is the final validator) ──────────
interface Geom {
  zoneRows: number;
  lakes: Set<number>;
  zoneOf(seat: number): (cell: number) => boolean;
}
function geom(w: number, h: number): Geom {
  const mini = w === 6 && h === 7;
  const zoneRows = mini ? 2 : 3;
  const lakes = new Set<number>(mini ? [20, 21] : [26, 29, 34, 37]);
  return {
    zoneRows,
    lakes,
    zoneOf: (seat: number) => (cell: number) => {
      const r = Math.floor(cell / w);
      return seat === 0 ? r >= h - zoneRows : r < zoneRows;
    },
  };
}

// ── handle ─────────────────────────────────────────────────────────────────────
function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.VanguardWasm(
    p.preset ?? 1,
    p.marshal ?? 1,
    p.captain ?? 1,
    p.sergeant ?? 2,
    p.trooper ?? 2,
    p.scout ?? 1,
    p.sapper ?? 2,
    p.spy ?? 1,
    p.bomb ?? 1,
    1, // Standard: always exactly one (the engine forces this)
    p.scoutLong ?? 1,
  );
  // Append the PUBLIC last-move summary so the board can render the combat beat +
  // last-move rings. `last_move_summary_for` is identical for both seats and never
  // carries a hidden rank — appending it leaks nothing (see header note).
  const withBeat = (base: string): string => {
    const lm = g.last_move_summary_for(0);
    return lm ? `${base}|LM=${lm}` : base;
  };
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    getBoardFor: (seat) => withBeat(g.get_board_for(seat)),
    // Secrecy-safe handoff summary: the outgoing seat's own most-recent move beat,
    // which is public (combat reveals are public knowledge). Empty during placement
    // ⇒ undefined (nothing to show beyond the normal blackout).
    lastMoveSummaryFor: (seat) => g.last_move_summary_for(seat) || undefined,
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const s: string = g.legal_moves();
      return s ? s.split(',') : [];
    },
    weakMove: (pl, k, t, s) => g.weak_move(pl, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

// ── piece glyph ────────────────────────────────────────────────────────────────
// Movable pieces show their combat rank digit (matches the summary's ranks);
// Bomb/Standard/Spy show a distinctive emoji so they read at a glance.
function pieceFace(kind: number | null): { text: string; emoji: boolean } {
  if (kind == null) return { text: '?', emoji: false };
  const m = KINDS[kind];
  if (kind === 7 || kind === 8 || kind === 6) return { text: m.emoji, emoji: true }; // Bomb / Standard / Spy
  return { text: String(m.rank), emoji: false };
}

// A small piece face for the rank-palette tray chips.
function PieceChip({ kind }: { kind: number }) {
  const face = pieceFace(kind);
  return (
    <span className={styles.vgChipFace} aria-hidden="true">
      {face.text}
    </span>
  );
}

// ── placement view ─────────────────────────────────────────────────────────────
function PlacementView({ st, interactive, onMove }: { st: Parsed; interactive: boolean; onMove: (m: string) => void }) {
  const { t } = useT();
  const { w, h, army, current, params } = st as Parsed & { params?: GameParams };
  const G = geom(w, h);
  const inZone = G.zoneOf(current);
  // Staged pieces: cell -> kind index. Submitted to the engine (which validates
  // each) only on Lock — placement moves are append-only, so we stage like Salvo.
  const [placed, setPlaced] = useState<Record<number, number>>({});
  const [sel, setSel] = useState<number | null>(null); // selected kind to drop
  const [shake, setShake] = useState<number | null>(null);
  const [locking, setLocking] = useState(false);
  const shakeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset staging whenever the underlying board changes (locked, turn passed, new
  // game). During a seat's own placement run the phase/current don't change, so a
  // multi-piece stage survives until it's submitted.
  useEffect(() => {
    setPlaced({});
    setSel(null);
    setLocking(false);
  }, [st.phase, st.current, w, h]);

  const stagedCount = (k: number): number => Object.values(placed).filter((v) => v === k).length;
  const remainingOf = (k: number): number => army[k] - stagedCount(k);
  const totalArmy = army.reduce((a, b) => a + b, 0);
  const allPlaced = Object.keys(placed).length === totalArmy && totalArmy > 0;

  // Engine-held pieces for this seat (after Lock, or while the AI deploys) — shown
  // read-only so a locked army never blinks out. Inert during interactive staging.
  const engineCells = new Set<number>(st.mine.map((p) => p.cell));

  // Honest clamp surfacing: compare the requested composition against the clamped
  // army the engine actually built (the `army` field is post-clamp).
  const requested = params
    ? (params.marshal ?? 1) +
      (params.captain ?? 1) +
      (params.sergeant ?? 2) +
      (params.trooper ?? 2) +
      (params.scout ?? 1) +
      (params.sapper ?? 2) +
      (params.spy ?? 1) +
      (params.bomb ?? 1) +
      1
    : totalArmy;
  const clamped = requested > totalArmy;

  const nextUnplaced = (): number | null => {
    for (let k = 0; k < KINDS.length; k++) if (army[k] > 0 && remainingOf(k) > 0) return k;
    return null;
  };
  const effSel = sel != null && army[sel] > 0 && remainingOf(sel) > 0 ? sel : nextUnplaced();

  const doShake = (cell: number) => {
    setShake(cell);
    if (shakeTimer.current) clearTimeout(shakeTimer.current);
    shakeTimer.current = setTimeout(() => setShake(null), 320);
  };

  const tapCell = (cell: number) => {
    if (!interactive || locking) return;
    if (placed[cell] !== undefined) {
      // Pick a staged piece back up.
      setPlaced((p) => {
        const n = { ...p };
        delete n[cell];
        return n;
      });
      return;
    }
    if (!inZone(cell) || G.lakes.has(cell)) {
      doShake(cell);
      return;
    }
    if (effSel == null || remainingOf(effSel) <= 0) {
      doShake(cell);
      return;
    }
    setPlaced((p) => ({ ...p, [cell]: effSel }));
  };

  // 🎲 auto-fill: a random valid layout (every zone cell is a legal placement, and
  // army ≤ zone capacity, so this can't fail). Mirrors Salvo's local placer; the
  // engine still validates each move on Lock.
  const deployForMe = () => {
    if (!interactive || locking) return;
    const cells: number[] = [];
    for (let c = 0; c < w * h; c++) if (inZone(c) && !G.lakes.has(c)) cells.push(c);
    for (let i = cells.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [cells[i], cells[j]] = [cells[j], cells[i]];
    }
    const next: Record<number, number> = {};
    let idx = 0;
    for (let k = 0; k < KINDS.length; k++) {
      for (let n = 0; n < army[k]; n++) {
        if (idx >= cells.length) break;
        next[cells[idx++]] = k;
      }
    }
    setPlaced(next);
    setSel(null);
  };

  const lockIn = () => {
    if (!interactive || locking || !allPlaced) return;
    setLocking(true);
    for (const [cell, kind] of Object.entries(placed)) onMove(`p:${kind}:${cell}`);
  };

  return (
    <div className={styles.vgWrap}>
      <div className={styles.vgInstruction}>
        {interactive
          ? effSel != null
            ? t('Deploy your {name} — tap a square in your zone. Tap a piece to pick it up.', {
                name: t(KINDS[effSel].name),
              })
            : t('Army deployed — lock it in!')
          : t('Deploying army…')}
      </div>

      {interactive && clamped && (
        <div className={styles.vgClamp}>{t('⚠️ Army trimmed to fit the board.')}</div>
      )}

      {/* Rank palette: one chip per kind you must deploy, with a rank number and a
          placed/total badge. Tapping a chip selects that kind to drop next. */}
      <div className={styles.vgTray}>
        {KINDS.map((meta, k) => {
          if (army[k] === 0) return null;
          const done = stagedCount(k);
          const active = effSel === k;
          return (
            <button
              key={k}
              className={`${styles.vgChip} ${active ? styles.vgChipActive : ''} ${done === army[k] ? styles.vgChipDone : ''}`}
              disabled={!interactive || done === army[k]}
              onClick={() => setSel(k)}
            >
              <PieceChip kind={k} />
              <span className={styles.vgChipName}>{t(meta.name)}</span>
              <span className={styles.vgBadge}>
                {done}/{army[k]}
              </span>
            </button>
          );
        })}
      </div>

      <VanguardGrid
        st={st}
        cells={Array.from({ length: w * h }, (_, cell) => {
          const stagedKind = placed[cell];
          const staged = stagedKind !== undefined;
          const isZone = inZone(cell);
          const lake = G.lakes.has(cell);
          return {
            cell,
            lake,
            dimZone: !isZone && !lake, // outside your zone during setup
            own: staged || engineCells.has(cell),
            face: staged ? pieceFace(stagedKind) : null,
            faceName: staged ? KINDS[stagedKind].name : '',
            shake: shake === cell,
            onTap: () => tapCell(cell),
            tappable: interactive && !locking,
          };
        })}
      />

      {interactive && (
        <>
          <div className={styles.vgControls}>
            <button className={styles.secondaryBtn} onClick={deployForMe}>
              {t('🎲 Deploy for me')}
            </button>
          </div>
          <button className={styles.playBtn} disabled={!allPlaced || locking} onClick={lockIn}>
            {t('Lock in army 🔒')}
          </button>
        </>
      )}
    </div>
  );
}

// ── shared grid renderer ────────────────────────────────────────────────────────
interface CellSpec {
  cell: number;
  lake: boolean;
  dimZone?: boolean;
  own: boolean;
  enemy?: boolean;
  hidden?: boolean;
  revealed?: boolean;
  scout?: boolean;
  moved?: boolean;
  face: { text: string; emoji: boolean } | null;
  faceName: string;
  sel?: boolean;
  target?: boolean;
  last?: boolean;
  shake?: boolean;
  onTap?: () => void;
  tappable?: boolean;
}
function VanguardGrid({ st, cells }: { st: Parsed; cells: CellSpec[] }) {
  const { t } = useT();
  return (
    <div className={styles.vgGrid} style={{ gridTemplateColumns: `repeat(${st.w}, 1fr)` }} role="grid">
      {cells.map((c) => {
        let cls = styles.vgCell;
        if (c.lake) cls += ` ${styles.vgLake}`;
        if (c.dimZone) cls += ` ${styles.vgDim}`;
        if (c.own) cls += ` ${styles.vgOwn}`;
        if (c.enemy && c.hidden) cls += ` ${styles.vgHidden}`;
        if (c.enemy && !c.hidden) cls += ` ${styles.vgEnemy}`;
        if (c.revealed && c.enemy) cls += ` ${styles.vgRevealed}`;
        if (c.sel) cls += ` ${styles.vgSel}`;
        if (c.target) cls += ` ${styles.vgTarget}`;
        if (c.last) cls += ` ${styles.vgLast}`;
        if (c.shake) cls += ` ${styles.vgShake}`;
        const label = c.lake
          ? t('Lake (impassable)')
          : c.faceName
            ? c.enemy
              ? t('Enemy {name}', { name: t(c.faceName) })
              : t('Your {name}', { name: t(c.faceName) })
            : c.hidden
              ? c.scout
                ? t('Enemy Scout (hidden rank)')
                : t('Enemy piece (hidden)')
              : t('Empty');
        return (
          <button
            key={c.cell}
            className={cls}
            disabled={!c.tappable}
            onClick={c.onTap}
            aria-label={label}
          >
            {c.face ? (
              <span className={c.face.emoji ? styles.vgFaceEmoji : styles.vgFaceRank} aria-hidden="true">
                {c.face.text}
              </span>
            ) : c.hidden ? (
              <span className={styles.vgBack} aria-hidden="true">
                {c.scout ? '🏃' : '◈'}
              </span>
            ) : null}
            {c.scout && (c.own || !c.hidden) && <span className={styles.vgScoutTag} aria-hidden="true">⇢</span>}
            {c.moved && !c.scout && <span className={styles.vgMovedDot} aria-hidden="true" />}
          </button>
        );
      })}
    </div>
  );
}

// ── play view ───────────────────────────────────────────────────────────────────
function PlayView({
  board,
  st,
  interactive,
  legalMoves,
  onMove,
}: {
  board: string;
  st: Parsed;
  interactive: boolean;
  legalMoves: string[];
  onMove: (m: string) => void;
}) {
  const { t } = useT();
  const { w, h } = st;
  const G = geom(w, h);
  const over = st.phase === 'over';
  const [from, setFrom] = useState<number | null>(null);
  // Reset the selection whenever the board changes (our move landed, the turn
  // passed, or — critically — the pass-and-play handoff) so no stale highlight
  // leaks into the next view.
  useEffect(() => setFrom(null), [board]);

  const mineAt = new Map<number, VPiece>();
  st.mine.forEach((p) => mineAt.set(p.cell, p));
  const theirsAt = new Map<number, VPiece>();
  st.theirs.forEach((p) => theirsAt.set(p.cell, p));

  // Targets for the selected piece, from the engine's own legal move list.
  const targets = new Set<number>();
  if (from != null)
    for (const m of legalMoves) {
      const [tag, f, to] = m.split(':');
      if (tag === 'm' && Number(f) === from) targets.add(Number(to));
    }
  // Which of my pieces can move at all (so only those invite a tap).
  const movable = new Set<number>();
  for (const m of legalMoves) {
    const [tag, f] = m.split(':');
    if (tag === 'm') movable.add(Number(f));
  }

  const lastCells = new Set<number>();
  if (st.beat) {
    lastCells.add(st.beat.from);
    lastCells.add(st.beat.to);
  }

  const tapCell = (cell: number) => {
    if (!interactive || over) return;
    if (from != null && targets.has(cell)) {
      onMove(`m:${from}:${cell}`);
      setFrom(null);
      return;
    }
    if (mineAt.has(cell) && movable.has(cell)) {
      setFrom((f) => (f === cell ? null : cell));
      return;
    }
    setFrom(null);
  };

  const cells: CellSpec[] = Array.from({ length: w * h }, (_, cell) => {
    const lake = G.lakes.has(cell);
    const mine = mineAt.get(cell);
    const enemy = theirsAt.get(cell);
    let face: CellSpec['face'] = null;
    let faceName = '';
    let scout = false;
    let moved = false;
    if (mine) {
      face = pieceFace(mine.kind);
      faceName = mine.kind != null ? KINDS[mine.kind].name : '';
      scout = mine.kind === SCOUT_KIND;
      moved = mine.moved;
    } else if (enemy) {
      scout = enemy.kind === SCOUT_KIND && (enemy.revealed || enemy.long);
      moved = enemy.moved;
      if (!enemy.hidden) {
        face = pieceFace(enemy.kind);
        faceName = enemy.kind != null ? KINDS[enemy.kind].name : '';
      }
    }
    return {
      cell,
      lake,
      own: !!mine,
      enemy: !!enemy,
      hidden: !!enemy?.hidden,
      revealed: !!enemy && !enemy.hidden,
      scout,
      moved,
      face,
      faceName,
      sel: from === cell,
      target: targets.has(cell),
      last: lastCells.has(cell),
      onTap: () => tapCell(cell),
      tappable: interactive && !over && (targets.has(cell) || (!!mine && movable.has(cell))),
    };
  });

  return (
    <div className={styles.vgWrap}>
      {st.beat && st.beat.t === 'c' && <CombatBeat beat={st.beat} />}
      {interactive && !over && (
        <div className={styles.vgInstruction}>
          {from != null ? t('Tap a highlighted square to move.') : t('Tap one of your pieces to move it.')}
        </div>
      )}
      <VanguardGrid st={st} cells={cells} />
      <div className={styles.vgLegend}>
        <span>◈ {t('hidden')}</span>
        <span>⇢ {t('Scout')}</span>
        <span>💣 {t('Bomb')}</span>
        <span>🚩 {t('Standard')}</span>
      </div>
    </div>
  );
}

// ── combat reveal beat (public — safe inline for vs-AI and post-handoff) ────────
function beatLine(beat: Beat, t: I18n['t']): string {
  if (beat.t === 'q') {
    if (beat.long) return t('🏃 A Scout dashed across the field.');
    const name = beat.glyph !== '?' ? KINDS[GLYPH_TO_KIND[beat.glyph]]?.name : '';
    return name ? t('{name} advanced.', { name: t(name) }) : t('A piece advanced.');
  }
  const an = t(KINDS[GLYPH_TO_KIND[beat.ag]]?.name ?? 'piece');
  const dn = t(KINDS[GLYPH_TO_KIND[beat.dg]]?.name ?? 'piece');
  switch (beat.outcome) {
    case 'sc':
      return t('⚔️ {att} captured the Standard 🚩 — game over!', { att: an });
    case 'bd':
      return t('⚔️ The Sapper defused a Bomb 💣.');
    case 'bh':
      return t('💥 A Bomb held — the {att} was destroyed.', { att: an });
    case 'aw':
      return t('⚔️ {att} (rank {ar}) captured {def} (rank {dr}).', { att: an, ar: beat.ar, def: dn, dr: beat.dr });
    case 'dw':
      return t('🛡️ {def} (rank {dr}) held off {att} (rank {ar}).', { def: dn, dr: beat.dr, att: an, ar: beat.ar });
    case 'mm':
      return t('⚔️ {att} and {def} were both lost (equal rank).', { att: an, def: dn });
    default:
      return t('⚔️ Combat!');
  }
}
function CombatBeat({ beat }: { beat: Beat }) {
  const { t } = useT();
  return <div className={styles.vgBeat}>{beatLine(beat, t)}</div>;
}

// ── board dispatcher ────────────────────────────────────────────────────────────
function VanguardBoard({ board, params, interactive, legalMoves, onMove }: BoardProps) {
  const st = parseBoard(board);
  // Thread params through to the placement view for honest clamp surfacing.
  (st as Parsed & { params?: GameParams }).params = params;
  if (st.phase === 'place') return <PlacementView st={st} interactive={interactive} onMove={onMove} />;
  return <PlayView board={board} st={st} interactive={interactive} legalMoves={legalMoves} onMove={onMove} />;
}

// ── presets ─────────────────────────────────────────────────────────────────────
// Ctor arg order: preset, marshal, captain, sergeant, trooper, scout, sapper, spy,
// bomb, (standard=1), scoutLong. Standard is always exactly one — the flag you hide.
const STANDARD: GameParams = {
  numPlayers: 2,
  preset: 1,
  marshal: 1,
  captain: 1,
  sergeant: 2,
  trooper: 2,
  scout: 1,
  sapper: 2,
  spy: 1,
  bomb: 1,
  scoutLong: 1,
};

export const vanguard: GameDefinition = {
  id: 'vanguard',
  name: 'Vanguard',
  icon: '🚩',
  blurb:
    'Deploy a secret army of ranked pieces behind your lines, then manoeuvre. Ranks stay hidden until pieces clash — win by capturing the enemy Standard, or leaving them with nowhere to move.',
  // Hand-set ladder (nim / Bulls & Cows / Salvo precedent): hidden info + a
  // first-mover edge make win-rate self-play calibration meaningless. These map to
  // the determinized-PIMC opponent's (K samples × search depth) via the engine's
  // `ladder()`: Easy samples 2 worlds at depth 1 with a wide, noisy pick; Hard
  // argmaxes 16 worlds at depth 2. Wired from the Task 7-2 report's ladder table.
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3.0 },
    medium: { playouts: 60, topK: 4, temp: 1.0 },
    hard: { playouts: 800, topK: 1, temp: 0.0 },
  },
  hiddenInfo: true,
  noUndo: true,
  defaultParams: STANDARD,
  presets: [
    { label: 'Standard', emoji: '⭐', params: STANDARD },
    // Mini 6×7, a compact 8-piece army (one of each fighter that matters + your
    // Standard) for a quick teaching game.
    {
      label: 'Mini',
      emoji: '⚡',
      params: {
        numPlayers: 2,
        preset: 0,
        marshal: 1,
        captain: 1,
        sergeant: 0,
        trooper: 1,
        scout: 1,
        sapper: 1,
        spy: 1,
        bomb: 1,
        scoutLong: 1,
      },
    },
    // Bomb Garden: six bombs, no sappers to defuse them — a minefield with a
    // handful of soldiers threading through. The composition-editor gag.
    {
      label: 'Bomb Garden',
      emoji: '🤯',
      params: {
        numPlayers: 2,
        preset: 1,
        marshal: 1,
        captain: 1,
        sergeant: 0,
        trooper: 2,
        scout: 0,
        sapper: 0,
        spy: 1,
        bomb: 6,
        scoutLong: 1,
      },
    },
    // Lockstep: the standard army, but Scouts march one square like everyone else
    // (scout_long off) — a slower, more deliberate game that showcases the toggle.
    { label: 'Lockstep', emoji: '🎩', params: { ...STANDARD, scoutLong: 0 } },
  ],
  knobs: [
    { key: 'preset', label: 'Board size', min: 0, max: 1, step: 1, valueLabels: ['Mini (6×7)', 'Standard (8×8)'] },
    { key: 'marshal', label: 'Marshals (rank 6)', min: 0, max: 6, step: 1 },
    { key: 'captain', label: 'Captains (rank 5)', min: 0, max: 6, step: 1 },
    { key: 'sergeant', label: 'Sergeants (rank 4)', min: 0, max: 6, step: 1 },
    { key: 'trooper', label: 'Troopers (rank 3)', min: 0, max: 6, step: 1 },
    { key: 'scout', label: 'Scouts (rank 2)', min: 0, max: 6, step: 1 },
    { key: 'sapper', label: 'Sappers (rank 1)', min: 0, max: 6, step: 1 },
    { key: 'spy', label: 'Spies (rank 0)', min: 0, max: 6, step: 1 },
    { key: 'bomb', label: 'Bombs 💣', min: 0, max: 6, step: 1 },
    { key: 'scoutLong', label: 'Scouts move far', min: 0, max: 1, step: 1, valueLabels: ['No', 'Yes'] },
  ],
  create: makeHandle,
  Board: VanguardBoard,
  // The pre-handoff reveal beat (pass-and-play): render the outgoing seat's public
  // move summary — the same combat beat the board shows inline in vs-AI.
  formatHandoffSummary: (raw, t) => {
    const beat = parseBeat(raw);
    return beat ? beatLine(beat, t) : '';
  },
  // No winningCells() on the handle, so resultFlavor fires on every terminal.
  // Narrate WHY the game ended (Standard capture / immobilisation / draw), honestly
  // for the lone human ("you") and by seat name otherwise.
  resultFlavor: ({ result, board, labels, seats, t }) => {
    const reason = parseBoard(board).reason;
    if (reason === 'draw' || result === 'Draw') return t('🏳️ Stalemate — the armies ground to a halt.');
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner)) return undefined;
    const humans = seats.map((s, i) => (s === 'human' ? i : -1)).filter((i) => i >= 0);
    const lone = humans.length === 1;
    const won = lone && humans[0] === winner;
    if (reason === 'immobilised') {
      if (lone)
        return won
          ? t('🎉 The enemy is boxed in — you win!')
          : t('😵 You are boxed in — no moves left.');
      const name = labels[winner] ? t(labels[winner]) : t('Player {n}', { n: winner + 1 });
      return t('🪤 {name} wins — the enemy is boxed in!', { name });
    }
    // Standard capture (or any other decisive terminal).
    if (lone)
      return won ? t('🚩 You captured the enemy Standard! 🎉') : t('🚩 Your Standard was captured.');
    const name = labels[winner] ? t(labels[winner]) : t('Player {n}', { n: winner + 1 });
    return t('🚩 {name} captured the enemy Standard!', { name });
  },
  playerLabels: ['Player 1', 'Player 2'],
};
