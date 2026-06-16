import { useState } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import styles from '../arcade.module.css';

// Domineering: each player places a domino, but player 0 (Vertical) places it
// downward and player 1 (Horizontal) places it rightward. A move is the anchor
// cell index; the engine's legal_moves() already lists the valid anchors for
// whoever is to move, so the board just renders them and previews the partner
// cell on hover.
function DomineeringBoard({ board, params, currentPlayer, interactive, legalMoves, onMove }: BoardProps) {
  const { cols, rows } = params;
  const [hover, setHover] = useState<number | null>(null);
  const legal = new Set(legalMoves.map(Number).filter((n) => !Number.isNaN(n)));
  const vertical = currentPlayer === 0;
  const partner = (i: number) => (vertical ? i + cols : i + 1);

  const ghost = new Set<number>();
  if (hover != null && legal.has(hover)) {
    ghost.add(hover);
    ghost.add(partner(hover));
  }
  const turnColor = vertical ? 'var(--arc-p1)' : 'var(--arc-p2)';

  return (
    <div>
      <div className={styles.domHint}>
        {vertical ? '↕ Vertical — place a domino downward' : '↔ Horizontal — place a domino rightward'}
      </div>
      <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)`, maxWidth: cols * 46 }}>
        {Array.from({ length: cols * rows }, (_, i) => {
          const ch = board[i] ?? ' ';
          const isLegal = interactive && legal.has(i);
          const isGhost = ghost.has(i);
          const bg =
            ch === 'X'
              ? 'var(--arc-p1)'
              : ch === 'O'
                ? 'var(--arc-p2)'
                : isGhost
                  ? `color-mix(in srgb, ${turnColor} 42%, transparent)`
                  : undefined;
          return (
            <button
              key={i}
              className={`${styles.tttCell} ${isLegal ? styles.domLegal : ''}`}
              disabled={!isLegal}
              onClick={() => onMove(String(i))}
              onMouseEnter={() => setHover(i)}
              onMouseLeave={() => setHover((h) => (h === i ? null : h))}
              style={{ background: bg }}
              aria-label={`Cell ${i + 1}: ${ch === ' ' ? 'empty' : ch}`}
            />
          );
        })}
      </div>
    </div>
  );
}

export const domineering: GameDefinition = {
  id: 'domineering',
  name: 'Domineering',
  icon: '🁢',
  blurb: 'Place dominoes — you go vertical, the foe goes horizontal. Last to fit one in wins.',
  defaultParams: { numPlayers: 2, cols: 6, rows: 6 },
  presets: [
    { label: 'Classic 6×6', emoji: '⭐', params: { numPlayers: 2, cols: 6, rows: 6 } },
    { label: 'Quick 5×5', emoji: '⚡', params: { numPlayers: 2, cols: 5, rows: 5 } },
    { label: 'Wide 8×8', emoji: '🔲', params: { numPlayers: 2, cols: 8, rows: 8 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 4, max: 8, step: 1 },
    { key: 'rows', label: 'Height', min: 4, max: 8, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.DomineeringWasm(p.cols, p.rows)),
  Board: DomineeringBoard,
  playerLabels: ['Vertical', 'Horizontal'],
};
