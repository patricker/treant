import { useEffect, useRef, useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT, type I18n } from '../i18n';
import styles from '../arcade.module.css';

// Board string is single-sourced with the Rust engine (salvo.rs `render`):
//   phase|size|touch|salvo|current|volleyLeft|volleySize|winner|fleet|myships|myincoming|myshots|oppsunk|oppfull
// Fields are relative to the VIEWING seat: `myships`/`myshots`/`myincoming` are
// the seat's own; `oppsunk` lists ONLY opponent ships this seat has already sunk
// (an attested reveal); `oppfull` is populated ONLY at game over. Un-sunk
// opponent ship cells are structurally absent — the load-bearing secrecy
// guarantee (see salvo.rs `wrong_seat_never_sees_fleet`).

type Orient = 'h' | 'v';
interface Ship {
  cell: number;
  orient: Orient;
  len: number;
  sunk: boolean;
}
interface Shot {
  cell: number;
  res: 'm' | 'h' | 'k'; // miss / hit / sunk
  len?: number; // sunk ship length (res === 'k')
}
interface SunkShip {
  cells: number[];
  len: number;
}
interface Parsed {
  phase: 'place' | 'fire' | 'over';
  size: number;
  touch: boolean;
  salvo: boolean;
  current: number;
  volleyLeft: number;
  volleySize: number;
  winner: string;
  fleet: number[];
  myships: Ship[];
  myincoming: Shot[];
  myshots: Shot[];
  oppsunk: SunkShip[];
  oppfull: Ship[];
}

function parseShips(s: string): Ship[] {
  if (!s) return [];
  return s.split(';').map((tok) => {
    const [cell, o, len, sunk] = tok.split('.');
    return { cell: Number(cell), orient: o === 'v' ? 'v' : 'h', len: Number(len), sunk: sunk === '1' };
  });
}
function parseShots(s: string): Shot[] {
  if (!s) return [];
  return s.split(';').map((tok) => {
    const [cell, res] = tok.split(':');
    if (res && res[0] === 'k') return { cell: Number(cell), res: 'k' as const, len: Number(res.slice(1)) };
    return { cell: Number(cell), res: (res === 'h' ? 'h' : 'm') as 'h' | 'm' };
  });
}
function parseSunk(s: string): SunkShip[] {
  if (!s) return [];
  return s.split(';').map((tok) => {
    const parts = tok.split('.').map(Number);
    const len = parts.pop() ?? 0;
    return { cells: parts, len };
  });
}
function parseBoard(board: string): Parsed {
  const p = board.split('|');
  return {
    phase: (p[0] as Parsed['phase']) || 'place',
    size: Number(p[1]) || 10,
    touch: p[2] === '1',
    salvo: p[3] === '1',
    current: Number(p[4]) || 0,
    volleyLeft: Number(p[5]) || 0,
    volleySize: Number(p[6]) || 0,
    winner: p[7] || '',
    fleet: p[8] ? p[8].split(',').map(Number) : [],
    myships: parseShips(p[9] || ''),
    myincoming: parseShots(p[10] || ''),
    myshots: parseShots(p[11] || ''),
    oppsunk: parseSunk(p[12] || ''),
    oppfull: parseShips(p[13] || ''),
  };
}

// ---- geometry (mirrors salvo.rs exactly; the engine is the final validator) ---
function shipCells(cell: number, orient: Orient, len: number, size: number): number[] | null {
  const r = Math.floor(cell / size);
  const c = cell % size;
  const cells: number[] = [];
  for (let i = 0; i < len; i++) {
    const rr = orient === 'h' ? r : r + i;
    const cc = orient === 'h' ? c + i : c;
    if (rr >= size || cc >= size) return null;
    cells.push(rr * size + cc);
  }
  return cells;
}
// Can `cells` be placed given `occ` (cell → ship idx for OTHER ships)? Mirrors
// placement_ok: no overlap, and when !touch no 8-adjacency to a different ship.
function placementOk(occ: Map<number, number>, size: number, cells: number[], touch: boolean): boolean {
  const own = new Set(cells);
  for (const c of cells) if (occ.has(c)) return false;
  if (!touch) {
    for (const c of cells) {
      const r = Math.floor(c / size);
      const col = c % size;
      for (let dr = -1; dr <= 1; dr++) {
        for (let dc = -1; dc <= 1; dc++) {
          const nr = r + dr;
          const ncol = col + dc;
          if (nr < 0 || ncol < 0 || nr >= size || ncol >= size) continue;
          const nc = nr * size + ncol;
          if (occ.has(nc) && !own.has(nc)) return false;
        }
      }
    }
  }
  return true;
}

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.SalvoWasm(
    p.size ?? 10,
    p.s1 ?? 0,
    p.s2 ?? 0,
    p.s3 ?? 0,
    p.s4 ?? 0,
    p.s5 ?? 0,
    p.touch ?? 1,
    p.salvo ?? 0,
  );
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    getBoardFor: (seat) => g.get_board_for(seat),
    // Secrecy-safe handoff summary: the outgoing seat's OWN fired volley, read
    // from its own view (get_board_for(seat) — never any opponent secret). The
    // last volley = the trailing `volleySize` of my shots; own surviving-ship
    // count doesn't change during my own volley, so it recovers the volley size.
    lastMoveSummaryFor: (seat) => {
      const st = parseBoard(g.get_board_for(seat));
      if (st.myshots.length === 0) return undefined; // placement move — nothing to show
      const surviving = st.myships.filter((s) => !s.sunk).length;
      const volleyN = st.salvo ? Math.min(Math.max(surviving, 1), st.myshots.length) : 1;
      const vol = st.myshots.slice(st.myshots.length - volleyN);
      return vol.map((s) => (s.res === 'k' ? `k${s.len}` : s.res)).join(';');
    },
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

// Friendly, trademark-free per-length ship names (index = length - 1).
const SHIP_NAME = ['Dinghy', 'Boat', 'Cruiser', 'Warship', 'Carrier'];
function shipName(len: number): string {
  return SHIP_NAME[len - 1] ?? `Length-${len}`;
}

// A tiny ship silhouette (len squares) for the tray chips.
function ShipGlyph({ len, orient = 'h' }: { len: number; orient?: Orient }) {
  return (
    <span className={`${styles.salvoGlyph} ${orient === 'v' ? styles.salvoGlyphV : ''}`} aria-hidden="true">
      {Array.from({ length: len }, (_, i) => (
        <span key={i} className={styles.salvoGlyphCell} />
      ))}
    </span>
  );
}

// ---- a small local random placer for "🎲 Place for me" (stages a valid fleet
// the player can review/adjust before locking; the engine still validates). ----
function randomFleet(fleet: number[], size: number, touch: boolean): Record<number, { origin: number; orient: Orient }> {
  const n2 = size * size;
  for (let attempt = 0; attempt < 60; attempt++) {
    const placed: Record<number, { origin: number; orient: Orient }> = {};
    const occ = new Map<number, number>();
    let ok = true;
    for (let idx = 0; idx < fleet.length; idx++) {
      const len = fleet[idx];
      const tries: { origin: number; orient: Orient; cells: number[] }[] = [];
      const orients: Orient[] = len === 1 ? ['h'] : ['h', 'v'];
      for (const o of orients) {
        for (let cell = 0; cell < n2; cell++) {
          const cells = shipCells(cell, o, len, size);
          if (cells && placementOk(occ, size, cells, touch)) tries.push({ origin: cell, orient: o, cells });
        }
      }
      if (tries.length === 0) {
        ok = false;
        break;
      }
      const pick = tries[Math.floor(Math.random() * tries.length)];
      placed[idx] = { origin: pick.origin, orient: pick.orient };
      pick.cells.forEach((c) => occ.set(c, idx));
    }
    if (ok) return placed;
  }
  return {};
}

// ─────────────────────────── Placement view ───────────────────────────────
function PlacementView({ st, interactive, onMove }: { st: Parsed; interactive: boolean; onMove: (m: string) => void }) {
  const { t } = useT();
  const { size, fleet, touch } = st;
  const [placed, setPlaced] = useState<Record<number, { origin: number; orient: Orient }>>({});
  const [orient, setOrient] = useState<Orient>('h');
  const [sel, setSel] = useState<number | null>(null);
  const [shake, setShake] = useState<number | null>(null);
  const [locking, setLocking] = useState(false);
  const shakeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset staging whenever the underlying board changes (our fleet locked, the
  // turn passed, or a new game started).
  useEffect(() => {
    setPlaced({});
    setSel(null);
    setOrient('h');
    setLocking(false);
  }, [st.phase, st.current, size, fleet.length]);

  // Cells occupied by staged ships → owning idx.
  const occ = new Map<number, number>();
  const cellsOf = (idx: number): number[] => {
    const pl = placed[idx];
    return pl ? shipCells(pl.origin, pl.orient, fleet[idx], size) ?? [] : [];
  };
  for (let idx = 0; idx < fleet.length; idx++) cellsOf(idx).forEach((c) => occ.set(c, idx));
  // Ships the engine already holds for this seat (after Lock, or while the AI
  // opponent places its fleet) — shown read-only so a locked fleet never blinks
  // out. During interactive placement `myships` is empty, so this is inert.
  const engineCells = new Set<number>();
  st.myships.forEach((sh) => (shipCells(sh.cell, sh.orient, sh.len, size) ?? []).forEach((c) => engineCells.add(c)));

  const nextUnplaced = (() => {
    for (let idx = 0; idx < fleet.length; idx++) if (!placed[idx]) return idx;
    return null;
  })();
  const effSel = sel != null && !placed[sel] ? sel : nextUnplaced;
  const allPlaced = Object.keys(placed).length === fleet.length && fleet.length > 0;

  const doShake = (cell: number) => {
    setShake(cell);
    if (shakeTimer.current) clearTimeout(shakeTimer.current);
    shakeTimer.current = setTimeout(() => setShake(null), 360);
  };

  const occExcept = (idx: number): Map<number, number> => {
    const m = new Map(occ);
    cellsOf(idx).forEach((c) => m.delete(c));
    return m;
  };

  const tapCell = (cell: number) => {
    if (!interactive || locking) return;
    const owner = occ.get(cell);
    if (owner !== undefined) {
      const pl = placed[owner];
      const len = fleet[owner];
      // Tap the ship's ORIGIN → rotate it in place; tap its body, or a length-1
      // ship, → pick it up (back to the tray).
      if (len > 1 && cell === pl.origin) {
        const no: Orient = pl.orient === 'h' ? 'v' : 'h';
        const cells = shipCells(pl.origin, no, len, size);
        if (cells && placementOk(occExcept(owner), size, cells, touch)) {
          setPlaced((p) => ({ ...p, [owner]: { origin: pl.origin, orient: no } }));
          return;
        }
        doShake(cell); // can't rotate here
        return;
      }
      setPlaced((p) => {
        const n = { ...p };
        delete n[owner];
        return n;
      });
      setSel(owner);
      return;
    }
    // Empty cell → place the selected ship.
    if (effSel == null) return;
    const len = fleet[effSel];
    const orients: Orient[] = len === 1 ? ['h'] : orient === 'h' ? ['h', 'v'] : ['v', 'h'];
    for (const o of orients) {
      const cells = shipCells(cell, o, len, size);
      if (cells && placementOk(occ, size, cells, touch)) {
        setPlaced((p) => ({ ...p, [effSel]: { origin: cell, orient: o } }));
        setSel(null);
        return;
      }
    }
    doShake(cell);
  };

  const placeForMe = () => {
    if (!interactive || locking) return;
    const r = randomFleet(fleet, size, touch);
    if (Object.keys(r).length === fleet.length) {
      setPlaced(r);
      setSel(null);
    }
  };

  const lockIn = () => {
    if (!interactive || locking || !allPlaced) return;
    setLocking(true);
    for (let idx = 0; idx < fleet.length; idx++) {
      const pl = placed[idx];
      onMove(`p:${idx}:${pl.origin}:${pl.orient}`);
    }
  };

  // Tray: fleet grouped by length (descending), each group a chip with a
  // placed/total badge. Tapping a group selects its next unplaced slot.
  const lengths = Array.from(new Set(fleet)).sort((a, b) => b - a);
  const selLen = effSel != null ? fleet[effSel] : null;

  return (
    <div className={styles.salvoWrap}>
      <div className={styles.salvoInstruction}>
        {interactive
          ? effSel != null
            ? t('Tap the sea to drop your {name}. Tap a ship to rotate or move it.', { name: t(shipName(fleet[effSel])) })
            : t('Fleet ready — lock it in!')
          : t('Placing fleet…')}
      </div>

      {/* Pass-and-play cadence tip. Shown only when salvo mode is selected and
          the view is non-interactive — i.e. in the setup-screen board preview
          (a human placing their own fleet is `interactive` and won't see it),
          so this is guidance at choose-your-board time, not mid-game clutter. */}
      {st.salvo && !interactive && (
        <div className={styles.salvoHint}>{t('💡 Salvo volleys = fewer phone passes')}</div>
      )}

      <div className={styles.salvoTray}>
        {lengths.map((L) => {
          const slots = fleet.map((l, i) => (l === L ? i : -1)).filter((i) => i >= 0);
          const done = slots.filter((i) => placed[i]).length;
          const active = selLen === L && effSel != null && !placed[effSel];
          return (
            <button
              key={L}
              className={`${styles.salvoChip} ${active ? styles.salvoChipActive : ''} ${done === slots.length ? styles.salvoChipDone : ''}`}
              disabled={!interactive || done === slots.length}
              onClick={() => {
                const nxt = slots.find((i) => !placed[i]);
                if (nxt != null) setSel(nxt);
              }}
            >
              <ShipGlyph len={L} />
              <span className={styles.salvoChipName}>{t(shipName(L))}</span>
              <span className={styles.salvoBadge}>
                {done}/{slots.length}
              </span>
            </button>
          );
        })}
      </div>

      <div
        className={styles.salvoGrid}
        style={{ gridTemplateColumns: `repeat(${size}, 1fr)` }}
        role="grid"
        aria-label={t('Your waters — place your fleet')}
      >
        {Array.from({ length: size * size }, (_, cell) => {
          const owner = occ.get(cell);
          const mine = owner !== undefined || engineCells.has(cell);
          const isOrigin = owner !== undefined && placed[owner].origin === cell;
          return (
            <button
              key={cell}
              className={`${styles.salvoCell} ${mine ? styles.salvoShip : ''} ${isOrigin ? styles.salvoShipOrigin : ''} ${shake === cell ? styles.salvoShake : ''}`}
              disabled={!interactive || locking}
              onClick={() => tapCell(cell)}
              aria-label={mine ? t('Your ship') : t('Open sea')}
            />
          );
        })}
      </div>

      {interactive && (
        <div className={styles.salvoControls}>
          <button className={styles.secondaryBtn} onClick={() => setOrient((o) => (o === 'h' ? 'v' : 'h'))}>
            {orient === 'h' ? t('⟳ Rotate (↔)') : t('⟳ Rotate (↕)')}
          </button>
          <button className={styles.secondaryBtn} onClick={placeForMe}>
            {t('🎲 Place for me')}
          </button>
        </div>
      )}
      {interactive && (
        <button className={styles.playBtn} disabled={!allPlaced || locking} onClick={lockIn}>
          {t('Lock in fleet 🔒')}
        </button>
      )}
    </div>
  );
}

// Human-readable cell label ("C4"): column letter + 1-indexed row. Used by the
// tap-to-aim confirm affordance so the highlighted target has a spoken name.
function coordLabel(cell: number, size: number): string {
  const r = Math.floor(cell / size);
  const c = cell % size;
  return `${String.fromCharCode(65 + c)}${r + 1}`;
}

// ─────────────────────────── Fire / over view ─────────────────────────────
function FireView({
  board,
  st,
  interactive,
  onMove,
}: {
  board: string;
  st: Parsed;
  interactive: boolean;
  onMove: (m: string) => void;
}) {
  const { t } = useT();
  const { size } = st;
  const over = st.phase === 'over';
  const [view, setView] = useState<'their' | 'yours'>('their');
  // Tap-to-aim / tap-again-to-fire. On a noUndo game with sub-40px cells an
  // immediate-fire tap is an irreversible misfire — so the FIRST tap only aims
  // (highlights the cell + names its coordinate); a SECOND tap on the SAME cell
  // fires; tapping a different cell moves the aim. Applied at EVERY board size:
  // one extra tap is cheap and it removes the whole class of fat-finger misfires.
  // The aim MUST reset whenever the board string changes — that covers firing a
  // shot, the turn passing, and (critically) the pass-and-play handoff, so no
  // stale crosshair leaks into the next player's view.
  const [aim, setAim] = useState<number | null>(null);
  useEffect(() => setAim(null), [board]);

  // Their waters: my shot map + any opponent ships I've sunk (revealed), plus
  // the full opponent fleet at game over.
  const myShotAt = new Map<number, Shot>();
  st.myshots.forEach((s) => myShotAt.set(s.cell, s));
  const sunkCells = new Set<number>();
  st.oppsunk.forEach((sh) => sh.cells.forEach((c) => sunkCells.add(c)));
  const oppShipCells = new Set<number>();
  if (over) st.oppfull.forEach((sh) => (shipCells(sh.cell, sh.orient, sh.len, size) ?? []).forEach((c) => oppShipCells.add(c)));

  // Your fleet: my ships + opponent's incoming shots at me.
  const myShipAt = new Map<number, Ship>();
  st.myships.forEach((sh) => (shipCells(sh.cell, sh.orient, sh.len, size) ?? []).forEach((c) => myShipAt.set(c, sh)));
  const incomingAt = new Map<number, Shot>();
  st.myincoming.forEach((s) => incomingAt.set(s.cell, s));

  const theirGrid = (
    <div
      className={styles.salvoGrid}
      style={{ gridTemplateColumns: `repeat(${size}, 1fr)` }}
      role="grid"
      aria-label={t('Enemy waters — tap to fire')}
    >
      {Array.from({ length: size * size }, (_, cell) => {
        const shot = myShotAt.get(cell);
        const wreck = sunkCells.has(cell);
        const revealed = oppShipCells.has(cell) && !shot;
        const canFire = interactive && !shot;
        const aimed = canFire && aim === cell;
        let cls = styles.salvoCell;
        let glyph = '';
        if (wreck) {
          cls += ` ${styles.salvoSunk}`;
          glyph = '☠';
        } else if (shot?.res === 'h' || shot?.res === 'k') {
          cls += ` ${styles.salvoHit}`;
          glyph = '💥';
        } else if (shot?.res === 'm') {
          cls += ` ${styles.salvoMiss}`;
          glyph = '·';
        } else if (aimed) {
          cls += ` ${styles.salvoAim}`;
          glyph = '⌖';
        } else if (revealed) {
          cls += ` ${styles.salvoReveal}`;
        }
        return (
          <button
            key={cell}
            className={cls}
            disabled={!canFire}
            // First tap aims; a second tap on the same cell fires.
            onClick={() => canFire && (aim === cell ? onMove(`s:${cell}`) : setAim(cell))}
            aria-label={
              shot
                ? shot.res === 'm'
                  ? t('Miss')
                  : t('Hit')
                : aimed
                  ? t('Aiming {coord} — tap again to fire', { coord: coordLabel(cell, size) })
                  : t('Aim {coord}', { coord: coordLabel(cell, size) })
            }
          >
            <span aria-hidden="true">{glyph}</span>
          </button>
        );
      })}
    </div>
  );

  const yourGrid = (
    <div
      className={styles.salvoGrid}
      style={{ gridTemplateColumns: `repeat(${size}, 1fr)` }}
      role="grid"
      aria-label={t('Your fleet')}
    >
      {Array.from({ length: size * size }, (_, cell) => {
        const ship = myShipAt.get(cell);
        const inc = incomingAt.get(cell);
        let cls = styles.salvoCell;
        let glyph = '';
        if (ship) {
          cls += ` ${styles.salvoOwn} ${ship.sunk ? styles.salvoSunk : ''}`;
          if (inc && (inc.res === 'h' || inc.res === 'k')) glyph = '💥';
        } else if (inc?.res === 'm') {
          cls += ` ${styles.salvoMiss}`;
          glyph = '·';
        }
        return (
          <div key={cell} className={cls} aria-hidden="true">
            <span>{glyph}</span>
          </div>
        );
      })}
    </div>
  );

  return (
    <div className={styles.salvoWrap}>
      <div className={styles.seg}>
        <button className={`${styles.segBtn} ${view === 'their' ? styles.segOn : ''}`} onClick={() => setView('their')}>
          {t('🎯 Enemy waters')}
        </button>
        <button className={`${styles.segBtn} ${view === 'yours' ? styles.segOn : ''}`} onClick={() => setView('yours')}>
          {t('🚢 Your fleet')}
        </button>
      </div>

      {interactive && st.salvo && (
        <div className={styles.salvoCounter}>
          {t('Volley — {n} shot(s) left', { n: st.volleyLeft })}
        </div>
      )}
      {interactive && !st.salvo && !over && (
        <div className={styles.salvoCounter}>{t('Take your shot')}</div>
      )}
      {interactive && !over && view === 'their' && aim != null && (
        <div className={styles.salvoAimLine}>
          {t('🎯 Aiming {coord} — tap again to fire', { coord: coordLabel(aim, size) })}
        </div>
      )}

      {view === 'their' ? theirGrid : yourGrid}

      <div className={styles.salvoLegend}>
        <span>💥 {t('hit')}</span>
        <span>· {t('miss')}</span>
        <span>☠ {t('sunk')}</span>
      </div>
    </div>
  );
}

function SalvoBoard({ board, interactive, onMove }: BoardProps) {
  const st = parseBoard(board);
  if (st.phase === 'place') return <PlacementView st={st} interactive={interactive} onMove={onMove} />;
  return <FireView board={board} st={st} interactive={interactive} onMove={onMove} />;
}

function formatHandoffSummary(raw: string, t: I18n['t']): string {
  const shots = raw.split(';').filter(Boolean);
  const glyphs = shots.map((r) => (r === 'm' ? '·' : r[0] === 'k' ? '☠' : '💥')).join(' ');
  const sunk = shots.filter((r) => r[0] === 'k').map((r) => Number(r.slice(1)));
  // Neutral, shooter-agnostic label (not "Your fire"): this line sits atop the
  // handoff blackout, which the INCOMING player also reads — so it must not
  // address them as the shooter. The outgoing seat's name isn't available at
  // this seam (formatHandoffSummary receives only the raw summary + t), and
  // plumbing it through the shared GamePlay blackout for a cosmetic label isn't
  // worth it, so a shooter-agnostic label is the right call.
  //
  // Count-aware wording: a non-salvo turn is ONE shot, for which "volley" reads
  // oddly, so a single shot says "Last shot:" and a multi-shot volley says
  // "Last volley:". `t` (not `tn`) is all this seam receives, so the two forms
  // are separate full-sentence keys picked by count.
  const single = shots.length === 1;
  if (sunk.length > 0) {
    const name = t(shipName(sunk[sunk.length - 1]));
    return single
      ? t('Last shot: {glyphs} — sank a {name}!', { glyphs, name })
      : t('Last volley: {glyphs} — sank a {name}!', { glyphs, name });
  }
  return single ? t('Last shot: {glyphs}', { glyphs }) : t('Last volley: {glyphs}', { glyphs });
}

// Classic fleet counts s1..s5 = 0,1,2,1,1 → lengths {5,4,3,3,2}, 17 cells — the
// attested 1990 Milton-Bradley-adjacent paper set (Carrier 5, one length-4, two
// Cruiser 3, one Boat 2). See salvo.rs rules-provenance table.
const CLASSIC: GameParams = { numPlayers: 2, size: 10, s1: 0, s2: 1, s3: 2, s4: 1, s5: 1, touch: 1, salvo: 0 };

export const salvo: GameDefinition = {
  id: 'salvo',
  name: 'Salvo',
  icon: '🚢',
  blurb: 'Hide your fleet, then hunt the enemy’s. Call your shots on the open sea — a hit, a miss, or a ship going down — and sink every last one before they sink you.',
  // Hand-set ladder (nim/Bulls & Cows precedent): hidden info + a first-mover
  // firing edge make win-rate self-play calibration meaningless. These map to the
  // determinized gunner's sample count K + top-K/temperature over its heat map:
  // Easy fires from a few noisy samples with a wide, hot pick (misses a lot, weak
  // hunting); Hard argmaxes a sharp heat map with parity search. Derived by the
  // 5B-1 engine implementer; Hard pinned by hard_gunner_sinks_..._within_bound.
  difficulty: {
    easy: { playouts: 6, topK: 8, temp: 3.0 },
    medium: { playouts: 60, topK: 4, temp: 1.0 },
    hard: { playouts: 160, topK: 1, temp: 0.0 },
  },
  hiddenInfo: true,
  noUndo: true,
  defaultParams: CLASSIC,
  presets: [
    { label: 'Classic', emoji: '⭐', params: CLASSIC },
    { label: 'Quick', emoji: '⚡', params: { numPlayers: 2, size: 6, s1: 0, s2: 2, s3: 1, s4: 0, s5: 0, touch: 1, salvo: 0 } },
    // Salvo ON: 6 one-cell dinghies on 12×12 is ~72 blind shots one-at-a-time —
    // a slog. Volleys (6 shots/turn, shrinking as dinghies sink) turn the blind
    // hunt fast and silly; that swingy volley IS the gag.
    { label: 'Dinghy Swarm', emoji: '🤯', params: { numPlayers: 2, size: 12, s1: 6, s2: 0, s3: 0, s4: 0, s5: 0, touch: 1, salvo: 1 } },
    { label: 'True Salvo', emoji: '🎩', params: { numPlayers: 2, size: 10, s1: 0, s2: 1, s3: 2, s4: 1, s5: 1, touch: 1, salvo: 1 } },
  ],
  // Five fleet count-knobs, one per ship length, labelled by a friendly
  // (trademark-free) ship name; plus board size and the two rule flags. The
  // engine clamps an infeasible fleet to a placeable one — the preview shows the
  // clamped fleet honestly.
  knobs: [
    { key: 'size', label: 'Board size', min: 6, max: 15, step: 1 },
    { key: 's5', label: 'Carriers (5)', min: 0, max: 6, step: 1 },
    { key: 's4', label: 'Warships (4)', min: 0, max: 6, step: 1 },
    { key: 's3', label: 'Cruisers (3)', min: 0, max: 6, step: 1 },
    { key: 's2', label: 'Boats (2)', min: 0, max: 6, step: 1 },
    { key: 's1', label: 'Dinghies (1)', min: 0, max: 6, step: 1 },
    { key: 'touch', label: 'Ships may touch', min: 0, max: 1, step: 1 },
    { key: 'salvo', label: 'Salvo (volley) mode', min: 0, max: 1, step: 1 },
  ],
  create: makeHandle,
  Board: SalvoBoard,
  formatHandoffSummary,
  // No winCells (this isn't a line game), so resultFlavor fires on every terminal
  // (bullscows/board-line precedent). Narrate the sinking honestly for the lone
  // human ("you"/"the enemy") and by seat name otherwise.
  resultFlavor: ({ result, labels, seats, t }) => {
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner)) return undefined;
    const humans = seats.map((s, i) => (s === 'human' ? i : -1)).filter((i) => i >= 0);
    if (humans.length === 1) {
      return humans[0] === winner
        ? t('⚓ You sank the enemy fleet! 🎉')
        : t('⚓ The enemy sank your whole fleet.');
    }
    const name = labels[winner] ? t(labels[winner]) : t('Player {n}', { n: winner + 1 });
    return t('⚓ {name} sank the whole fleet!', { name });
  },
  playerLabels: ['Player 1', 'Player 2'],
};
