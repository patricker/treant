import { useEffect, useRef } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';
import { usePrevBoard } from '../boardDiff';

const TILE_BG: Record<number, string> = {
  0: 'rgba(238,228,218,0.35)',
  2: '#eee4da',
  4: '#ede0c8',
  8: '#f2b179',
  16: '#f59563',
  32: '#f67c5f',
  64: '#f65e3b',
  128: '#edcf72',
  256: '#edcc61',
  512: '#edc850',
  1024: '#edc53f',
  2048: '#edc22e',
};
const tileColor = (v: number) => (v <= 4 ? '#776e65' : '#f9f6f2');

function makeHandle(wasm: any, _p: GameParams): GameHandle {
  const g = new wasm.Game2048Wasm();
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => (g.get_board() as number[]).join(','),
    currentPlayer: () => 0,
    isTerminal: () => g.is_terminal(),
    result: () => '',
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => ['Up', 'Down', 'Left', 'Right'],
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    statusText: () => `Score ${g.score()} · Best ${g.max_tile()}`,
    endText: () =>
      g.max_tile() >= 2048
        ? `🎉 You made ${g.max_tile()}!  Score ${g.score()}`
        : `Game over — score ${g.score()}, best tile ${g.max_tile()}`,
    free: () => g.free(),
  };
}

function Game2048Board({ board, interactive, onMove }: BoardProps) {
  const tiles = board.split(',').map(Number);
  const prevBoard = usePrevBoard(board);
  const prevTiles = prevBoard ? prevBoard.split(',').map(Number) : [];
  // Per-tile animation: 0 -> n is a spawn (pop), an increased value is a merge (bump).
  const tileAnim = (i: number): string => {
    const before = prevTiles[i] ?? 0;
    const now = tiles[i];
    if (prevTiles.length !== tiles.length) return '';
    if (before === 0 && now > 0) return styles.popIn;
    if (now > before && before > 0) return styles.mergeBump;
    return '';
  };
  const startRef = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    if (!interactive) return;
    const onKey = (e: KeyboardEvent) => {
      const map: Record<string, string> = {
        ArrowUp: 'Up',
        ArrowDown: 'Down',
        ArrowLeft: 'Left',
        ArrowRight: 'Right',
      };
      if (map[e.key]) {
        e.preventDefault();
        onMove(map[e.key]);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [interactive, onMove]);

  const onTouchStart = (e: React.TouchEvent) => {
    const t = e.touches[0];
    startRef.current = { x: t.clientX, y: t.clientY };
  };
  const onTouchEnd = (e: React.TouchEvent) => {
    const start = startRef.current;
    startRef.current = null;
    if (!start || !interactive) return;
    const t = e.changedTouches[0];
    const dx = t.clientX - start.x;
    const dy = t.clientY - start.y;
    if (Math.abs(dx) < 20 && Math.abs(dy) < 20) return;
    onMove(
      Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? 'Right' : 'Left') : dy > 0 ? 'Down' : 'Up',
    );
  };

  return (
    <div className={styles.g2048Wrap}>
      <div className={styles.g2048Grid} onTouchStart={onTouchStart} onTouchEnd={onTouchEnd}>
        {tiles.map((v, i) => (
          <div
            key={i}
            className={`${styles.g2048Tile} ${tileAnim(i)}`}
            style={{
              background: TILE_BG[v] ?? '#3c3a32',
              color: tileColor(v),
              fontSize: v >= 1024 ? '1.1rem' : '1.5rem',
            }}
          >
            {v > 0 ? v : ''}
          </div>
        ))}
      </div>
      <div className={styles.g2048Arrows}>
        <button
          className={styles.g2048Arrow}
          disabled={!interactive}
          onClick={() => onMove('Up')}
          aria-label="Up"
        >
          ⬆️
        </button>
        <div className={styles.g2048ArrowRow}>
          <button
            className={styles.g2048Arrow}
            disabled={!interactive}
            onClick={() => onMove('Left')}
            aria-label="Left"
          >
            ⬅️
          </button>
          <button
            className={styles.g2048Arrow}
            disabled={!interactive}
            onClick={() => onMove('Down')}
            aria-label="Down"
          >
            ⬇️
          </button>
          <button
            className={styles.g2048Arrow}
            disabled={!interactive}
            onClick={() => onMove('Right')}
            aria-label="Right"
          >
            ➡️
          </button>
        </div>
      </div>
    </div>
  );
}

export const game2048: GameDefinition = {
  id: '2048',
  name: '2048',
  icon: '🔢',
  blurb: 'Swipe to merge tiles. Reach 2048 — or watch the AI try.',
  solo: true,
  defaultParams: { numPlayers: 1 },
  presets: [],
  knobs: [],
  create: makeHandle,
  Board: Game2048Board,
  formatHint: (m) =>
    ({ Up: '⬆️ Up', Down: '⬇️ Down', Left: '⬅️ Left', Right: '➡️ Right' })[m] ?? m,
};
