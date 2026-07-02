import { useEffect, useMemo, useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import styles from '../arcade.module.css';
import { useT } from '../i18n';

const COLOR: Record<string, string> = {
  X: 'var(--arc-p1)',
  O: 'var(--arc-p2)',
};

// Amazons needs a three-step interaction: pick one of your amazons, move it
// like a queen, then shoot an arrow (also queen-wise) to burn a square. The
// engine's legal_moves() are "from-to-arrow" triples, so the board just indexes
// into them by the choices made so far.
function AmazonsBoard({ board, params, currentPlayer, interactive, legalMoves, onMove }: BoardProps) {
  const { t } = useT();
  const { cols, rows } = params;
  const [from, setFrom] = useState<number | null>(null);
  const [to, setTo] = useState<number | null>(null);
  // Reset the multi-step selection whenever the board changes (our move landed,
  // or the AI/opponent moved) so a stale pick/move can't carry into a new turn.
  useEffect(() => {
    setFrom(null);
    setTo(null);
  }, [board]);

  const { froms, tosByFrom, arrowsByFromTo } = useMemo(() => {
    const froms = new Set<number>();
    const tosByFrom = new Map<number, Set<number>>();
    const arrowsByFromTo = new Map<string, Set<number>>();
    for (const m of legalMoves) {
      const p = m.split('-').map(Number);
      if (p.length !== 3 || p.some(Number.isNaN)) continue;
      const [f, t, a] = p;
      froms.add(f);
      if (!tosByFrom.has(f)) tosByFrom.set(f, new Set());
      tosByFrom.get(f)!.add(t);
      const key = `${f}-${t}`;
      if (!arrowsByFromTo.has(key)) arrowsByFromTo.set(key, new Set());
      arrowsByFromTo.get(key)!.add(a);
    }
    return { froms, tosByFrom, arrowsByFromTo };
  }, [legalMoves]);

  const phase = from == null ? 'pick' : to == null ? 'move' : 'shoot';
  const tos = from != null ? (tosByFrom.get(from) ?? new Set<number>()) : new Set<number>();
  const arrows = from != null && to != null ? (arrowsByFromTo.get(`${from}-${to}`) ?? new Set<number>()) : new Set<number>();

  const tap = (i: number) => {
    if (!interactive) return;
    if (phase === 'pick') {
      if (froms.has(i)) setFrom(i);
    } else if (phase === 'move') {
      if (i === from) setFrom(null);
      else if (tos.has(i)) setTo(i);
      else if (froms.has(i)) setFrom(i);
    } else {
      if (arrows.has(i)) {
        onMove(`${from}-${to}-${i}`);
        setFrom(null);
        setTo(null);
      } else if (i === to) {
        setTo(null);
      }
    }
  };

  const caption =
    phase === 'pick' ? t('Tap one of your amazons') : phase === 'move' ? t('Tap where to move') : t('Tap where to shoot the arrow');
  const myGlyph = currentPlayer === 0 ? 'X' : 'O';

  return (
    <div>
      <div className={styles.shiftCaption}>{caption}</div>
      <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {Array.from({ length: cols * rows }, (_, i) => {
          let ch = board[i] ?? ' ';
          if (phase === 'shoot') {
            if (i === from) ch = ' ';
            if (i === to) ch = myGlyph;
          }
          const isSel = (phase === 'move' && i === from) || (phase === 'shoot' && i === to);
          const isMoveTarget = phase === 'move' && tos.has(i);
          const isArrow = phase === 'shoot' && arrows.has(i);
          const clickable =
            interactive &&
            ((phase === 'pick' && froms.has(i)) ||
              (phase === 'move' && (tos.has(i) || froms.has(i) || i === from)) ||
              (phase === 'shoot' && (arrows.has(i) || i === to)));
          return (
            <button
              key={i}
              className={`${styles.tttCell} ${isSel ? styles.shiftSel : ''} ${isMoveTarget ? styles.moveTarget : ''} ${isArrow ? styles.azArrow : ''}`}
              disabled={!clickable}
              onClick={() => tap(i)}
              style={{
                color: COLOR[ch],
                background: ch === '#' ? 'var(--arc-burnt, #4a3f5c)' : undefined,
                fontSize: cols > 6 ? '1rem' : undefined,
              }}
              aria-label={t('Cell {n}: {state}', { n: i + 1, state: ch === ' ' ? t('empty') : ch === '#' ? t('burnt') : ch })}
            >
              {ch === 'X' || ch === 'O' ? '♛' : ch === ' ' && isMoveTarget ? '•' : ''}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const amazons: GameDefinition = {
  id: 'amazons',
  name: 'Amazons',
  icon: '♛',
  blurb: 'Move an amazon like a queen, then shoot an arrow to burn a square. Trap your foe with no moves left.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 200, topK: 3, temp: 0.5 },
  },
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Quick 5×5', emoji: '⚡', params: { numPlayers: 2, cols: 5, rows: 5 } },
    { label: 'Mega 7×7', emoji: '🤯', params: { numPlayers: 2, cols: 7, rows: 7 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 5, max: 7, step: 1 },
    { key: 'rows', label: 'Height', min: 5, max: 7, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.AmazonsWasm(p.cols, p.rows)),
  Board: AmazonsBoard,
  playerLabels: ['Red', 'Yellow'],
};
