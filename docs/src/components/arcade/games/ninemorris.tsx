import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// The 24 points on a 0..6 grid: three concentric squares with midpoint spokes.
const GRID: [number, number][] = [
  [0, 0], [3, 0], [6, 0],
  [1, 1], [3, 1], [5, 1],
  [2, 2], [3, 2], [4, 2],
  [0, 3], [1, 3], [2, 3], [4, 3], [5, 3], [6, 3],
  [2, 4], [3, 4], [4, 4],
  [1, 5], [3, 5], [5, 5],
  [0, 6], [3, 6], [6, 6],
];
// The drawn board segments (each square side is split by its midpoint, plus the
// four connector spokes) — mirrors the engine's adjacency.
const EDGES: [number, number][] = [
  [0, 1], [1, 2], [3, 4], [4, 5], [6, 7], [7, 8],
  [9, 10], [10, 11], [12, 13], [13, 14], [15, 16], [16, 17],
  [18, 19], [19, 20], [21, 22], [22, 23],
  [0, 9], [9, 21], [3, 10], [10, 18], [6, 11], [11, 15],
  [1, 4], [4, 7], [16, 19], [19, 22], [8, 12], [12, 17],
  [5, 13], [13, 20], [2, 14], [14, 23],
];
// Grid coord → viewBox percent (8% margin, 84% span).
const px = (g: number) => 8 + (g / 6) * 84;

type ParsedMove = { from: number | 'p'; to: number; victim: number | null; raw: string };
function parse(m: string): ParsedMove | null {
  const [main, vic] = m.split('x');
  const victim = vic !== undefined ? Number(vic) : null;
  if (main.startsWith('p')) {
    return { from: 'p', to: Number(main.slice(1)), victim, raw: m };
  }
  const dash = main.indexOf('-');
  if (dash < 0) return null;
  return { from: Number(main.slice(0, dash)), to: Number(main.slice(dash + 1)), victim, raw: m };
}

function NineMorrisBoard({ board, interactive, legalMoves, onMove }: BoardProps) {
  const { t } = useT();
  const [sel, setSel] = useState<number | null>(null);
  const [pending, setPending] = useState<{ prefix: string; victims: Set<number> } | null>(null);
  useEffect(() => {
    setSel(null);
    setPending(null);
  }, [board]); // drop any in-progress selection once a move lands

  const [cellStr, phase = ''] = board.split('|');
  const cells = Array.from({ length: 24 }, (_, i) => cellStr[i] ?? ' ');

  const moves = legalMoves.map(parse).filter(Boolean) as ParsedMove[];
  const placing = moves.some((m) => m.from === 'p');

  // placement target → its variants; slide source → target → variants
  const placeMap = new Map<number, ParsedMove[]>();
  const slideMap = new Map<number, Map<number, ParsedMove[]>>();
  for (const m of moves) {
    if (m.from === 'p') {
      (placeMap.get(m.to) ?? placeMap.set(m.to, []).get(m.to)!).push(m);
    } else {
      const f = m.from as number;
      const tgts = slideMap.get(f) ?? slideMap.set(f, new Map()).get(f)!;
      (tgts.get(m.to) ?? tgts.set(m.to, []).get(m.to)!).push(m);
    }
  }
  const targets: Map<number, ParsedMove[]> = sel != null ? (slideMap.get(sel) ?? new Map()) : new Map();

  const commit = (variants: ParsedMove[], prefix: string) => {
    if (variants.length === 1 && variants[0].victim == null) {
      onMove(variants[0].raw);
    } else {
      setPending({ prefix, victims: new Set(variants.map((v) => v.victim!).filter((v) => v != null)) });
    }
  };

  const tap = (i: number) => {
    if (!interactive) return;
    if (pending) {
      if (pending.victims.has(i)) {
        onMove(`${pending.prefix}x${i}`);
        setPending(null);
      }
      return; // in removal mode only enemy victims respond
    }
    if (placing) {
      const variants = placeMap.get(i);
      if (variants) commit(variants, `p${i}`);
      return;
    }
    if (sel != null && targets.has(i)) {
      commit(targets.get(i)!, `${sel}-${i}`);
      return;
    }
    setSel(slideMap.has(i) && i !== sel ? i : null);
  };

  const isVictim = (i: number) => pending?.victims.has(i) ?? false;
  const isTarget = (i: number) => sel != null && targets.has(i);
  const tappable = (i: number) => {
    if (pending) return isVictim(i);
    if (placing) return placeMap.has(i);
    return slideMap.has(i) || isTarget(i);
  };

  const caption = pending
    ? t('Mill! Tap an enemy piece to remove')
    : placing
      ? t('Tap an empty point to place')
      : sel == null
        ? t('Tap a piece')
        : t('Tap where to move');

  // Remaining-to-place pips (parsed from the "place:r0,r1" phase suffix).
  let toPlace: [number, number] | null = null;
  if (phase.startsWith('place:')) {
    const [a, b] = phase.slice(6).split(',').map(Number);
    toPlace = [a, b];
  }

  return (
    <div>
      <div className={styles.shiftCaption}>{caption}</div>
      {toPlace && (
        <div style={{ display: 'flex', gap: 14, justifyContent: 'center', alignItems: 'center', paddingBottom: 8, fontWeight: 800 }}>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}>
            <span style={{ width: 12, height: 12, borderRadius: '50%', background: 'var(--arc-p1)' }} />
            {toPlace[0]}
          </span>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}>
            <span style={{ width: 12, height: 12, borderRadius: '50%', background: 'var(--arc-p2)' }} />
            {toPlace[1]}
          </span>
          <span style={{ fontWeight: 600, opacity: 0.7 }}>{t('to place')}</span>
        </div>
      )}
      <div className={styles.mtWrap} style={{ maxWidth: 360 }}>
        <svg viewBox="0 0 100 100" className={styles.mtLines}>
          {EDGES.map(([a, b], k) => (
            <line key={k} x1={px(GRID[a][0])} y1={px(GRID[a][1])} x2={px(GRID[b][0])} y2={px(GRID[b][1])} />
          ))}
        </svg>
        {cells.map((ch, i) => {
          const [gx, gy] = GRID[i];
          const isSel = i === sel;
          const victim = isVictim(i);
          const target = isTarget(i);
          const stateWord = ch === 'X' ? t('Red') : ch === 'O' ? t('Yellow') : t('empty');
          return (
            <button
              key={i}
              className={`${styles.mtCell} ${isSel ? styles.shiftSel : ''} ${target ? styles.moveTarget : ''}`}
              disabled={!interactive || !tappable(i)}
              onClick={() => tap(i)}
              style={{
                left: `${px(gx)}%`,
                top: `${px(gy)}%`,
                width: '10%',
                height: '10%',
                background: ch === 'X' ? 'var(--arc-p1)' : ch === 'O' ? 'var(--arc-p2)' : 'var(--arc-soft)',
                outline: victim ? '3px solid var(--arc-p1)' : undefined,
                boxShadow: victim ? '0 0 0 4px rgba(255,59,92,0.25)' : undefined,
              }}
              aria-label={t('Point {n}: {state}', { n: i, state: stateWord })}
            />
          );
        })}
      </div>
    </div>
  );
}

export const nineMorris: GameDefinition = {
  id: 'nine-morris',
  name: "Nine Men's Morris",
  icon: '🔗',
  blurb: 'Place nine men, then slide them along the lines. Make a mill of three to snatch an enemy man. Grind them down to two.',
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, men: 9, flying: 0 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, men: 9, flying: 0 } },
    { label: 'Six Men', emoji: '⚡', params: { numPlayers: 2, men: 6, flying: 0 } },
    { label: 'Twelve Men', emoji: '🤯', params: { numPlayers: 2, men: 12, flying: 0 } },
    { label: 'Flying Nine', emoji: '🪽', params: { numPlayers: 2, men: 9, flying: 1 } },
  ],
  knobs: [
    { key: 'men', label: 'Men', min: 3, max: 12, step: 1 },
    { key: 'flying', label: 'Flying', min: 0, max: 1, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.NineMorrisWasm(p.men, p.flying)),
  Board: NineMorrisBoard,
  playerLabels: ['Red', 'Yellow'],
};
