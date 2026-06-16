import type { JSX } from 'react';
import type { BoardProps, GameDefinition } from '../gameTypes';
import { moveHandle } from './frontline';
import styles from '../arcade.module.css';

const P0 = 'var(--arc-p1)';
const P1 = 'var(--arc-p2)';

function DotsBoxesBoard({ board, params, interactive, legalMoves, onMove }: BoardProps) {
  const C = params.cols;
  const R = params.rows;
  const [edgesStr = '', boxesStr = '', scoresStr = '0,0'] = board.split('|');
  const [s0, s1] = scoresStr.split(',').map(Number);
  const hoff = (R + 1) * C;
  const legal = new Set(legalMoves);

  const gridCols = 2 * C + 1;
  const gridRows = 2 * R + 1;
  const colTemplate = Array.from({ length: gridCols }, (_, i) => (i % 2 === 0 ? '13px' : '1fr')).join(' ');
  const rowTemplate = Array.from({ length: gridRows }, (_, i) => (i % 2 === 0 ? '13px' : '34px')).join(' ');

  const edgeBtn = (e: number, horizontal: boolean): JSX.Element => {
    const claimed = edgesStr[e] === '1';
    const isLegal = interactive && legal.has(String(e));
    return (
      <button
        key={`e${e}`}
        className={styles.dbEdge}
        disabled={!isLegal}
        onClick={() => onMove(String(e))}
        aria-label={`Edge ${e}${claimed ? ' claimed' : ''}`}
        style={{
          width: horizontal ? '70%' : '5px',
          height: horizontal ? '5px' : '70%',
          background: claimed ? 'var(--arc-ink)' : isLegal ? 'var(--arc-soft)' : 'transparent',
          cursor: isLegal ? 'pointer' : 'default',
        }}
      />
    );
  };

  const cells: JSX.Element[] = [];
  for (let gr = 0; gr < gridRows; gr++) {
    for (let gc = 0; gc < gridCols; gc++) {
      const er = gr % 2 === 0;
      const ec = gc % 2 === 0;
      if (er && ec) {
        cells.push(<div key={`d${gr}_${gc}`} className={styles.dbDot} />);
      } else if (er && !ec) {
        cells.push(<div key={`h${gr}_${gc}`} className={styles.dbSlot}>{edgeBtn((gr / 2) * C + (gc - 1) / 2, true)}</div>);
      } else if (!er && ec) {
        cells.push(
          <div key={`v${gr}_${gc}`} className={styles.dbSlot}>
            {edgeBtn(hoff + ((gr - 1) / 2) * (C + 1) + gc / 2, false)}
          </div>,
        );
      } else {
        const b = ((gr - 1) / 2) * C + (gc - 1) / 2;
        const owner = boxesStr[b];
        cells.push(
          <div
            key={`b${gr}_${gc}`}
            className={styles.dbBox}
            style={{ background: owner === '0' ? P0 : owner === '1' ? P1 : 'transparent', opacity: owner === '.' ? 1 : 0.8 }}
          />,
        );
      }
    }
  }

  return (
    <div>
      <div className={styles.reversiScores}>
        <span style={{ color: P0 }}>● {s0 || 0}</span>
        <span style={{ color: P1 }}>● {s1 || 0}</span>
      </div>
      <div
        className={styles.dbGrid}
        style={{ gridTemplateColumns: colTemplate, gridTemplateRows: rowTemplate }}
      >
        {cells}
      </div>
    </div>
  );
}

export const dotsBoxes: GameDefinition = {
  id: 'dots-and-boxes',
  name: 'Dots & Boxes',
  icon: '⬛',
  blurb: 'Claim a line; complete a box to claim it and go again. Most boxes wins.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 3, rows: 3 },
  presets: [
    { label: 'Classic 3×3', emoji: '⭐', params: { numPlayers: 2, cols: 3, rows: 3 } },
    { label: 'Tiny 2×2', emoji: '🔳', params: { numPlayers: 2, cols: 2, rows: 2 } },
    { label: 'Mega 5×5', emoji: '🤯', params: { numPlayers: 2, cols: 5, rows: 5 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 2, max: 5, step: 1 },
    { key: 'rows', label: 'Height', min: 2, max: 5, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.DotsBoxesWasm(p.cols, p.rows)),
  Board: DotsBoxesBoard,
  playerLabels: ['Red', 'Yellow'],
};
