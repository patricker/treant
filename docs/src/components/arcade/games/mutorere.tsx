import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

function pos(i: number): { x: number; y: number } {
  if (i === 0) return { x: 50, y: 50 };
  const ang = ((i - 1) * 45 - 90) * (Math.PI / 180);
  return { x: 50 + 42 * Math.cos(ang), y: 50 + 42 * Math.sin(ang) };
}

function MuTorereBoard({ board, interactive, legalMoves, onMove }: BoardProps) {
  const { t } = useT();
  const [sel, setSel] = useState<number | null>(null);
  useEffect(() => setSel(null), [board]); // drop stale selection after any move
  const cells = Array.from({ length: 9 }, (_, i) => board[i] ?? ' ');

  const fromTo = new Map<number, Set<number>>();
  for (const m of legalMoves) {
    const dash = m.indexOf('-');
    if (dash < 0) continue;
    const f = Number(m.slice(0, dash));
    const t = Number(m.slice(dash + 1));
    if (!fromTo.has(f)) fromTo.set(f, new Set());
    fromTo.get(f)!.add(t);
  }
  const targets = sel != null ? (fromTo.get(sel) ?? new Set<number>()) : new Set<number>();
  const tap = (i: number) => {
    if (!interactive) return;
    if (sel != null && targets.has(i)) {
      onMove(`${sel}-${i}`);
      setSel(null);
      return;
    }
    setSel(fromTo.has(i) && i !== sel ? i : null);
  };

  return (
    <div className={styles.mtWrap}>
      <svg viewBox="0 0 100 100" className={styles.mtLines}>
        {Array.from({ length: 8 }, (_, k) => {
          const a = pos(k + 1);
          const b = pos(k === 7 ? 1 : k + 2);
          return <line key={`r${k}`} x1={a.x} y1={a.y} x2={b.x} y2={b.y} />;
        })}
        {Array.from({ length: 8 }, (_, k) => {
          const a = pos(k + 1);
          return <line key={`s${k}`} x1="50" y1="50" x2={a.x} y2={a.y} />;
        })}
      </svg>
      {cells.map((ch, i) => {
        const p = pos(i);
        const isSel = i === sel;
        const isTarget = targets.has(i);
        return (
          <button
            key={i}
            className={`${styles.mtCell} ${isSel ? styles.shiftSel : ''} ${isTarget ? styles.moveTarget : ''}`}
            disabled={!interactive || (!fromTo.has(i) && !isTarget)}
            onClick={() => tap(i)}
            style={{
              left: `${p.x}%`,
              top: `${p.y}%`,
              background: ch === 'X' ? 'var(--arc-p1)' : ch === 'O' ? 'var(--arc-p2)' : 'var(--arc-soft)',
            }}
            aria-label={t('Point {n}: {state}', { n: i, state: ch === ' ' ? t('empty') : ch })}
          />
        );
      })}
    </div>
  );
}

export const muTorere: GameDefinition = {
  id: 'mu-torere',
  name: 'Mu Tōrere',
  icon: '✴️',
  blurb: 'Slide your pieces around the star. Block your opponent so they can’t move.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  defaultParams: { numPlayers: 2 },
  presets: [{ label: 'Classic', emoji: '⭐', params: { numPlayers: 2 } }],
  knobs: [],
  create: (wasm) => moveHandle(new wasm.MuTorereWasm()),
  Board: MuTorereBoard,
  playerLabels: ['Red', 'Yellow'],
};
