import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.HexWasm(p.size);
  const n = p.size;
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (k) => g.playout_n(k),
    legalMoves: () => {
      const b = g.get_board();
      const out: string[] = [];
      for (let i = 0; i < n * n; i++) if ((b[i] ?? ' ') === ' ') out.push(String(i));
      return out;
    },
    free: () => g.free(),
  };
}

function HexBoard({ board, params, interactive, onMove }: BoardProps) {
  const n = params.size;
  const size = Math.max(16, Math.floor(300 / (n * 1.35)));
  return (
    <div>
      <div className={styles.shiftCaption}>
        <span style={{ color: 'var(--arc-p1)' }}>X</span> links top↔bottom ·{' '}
        <span style={{ color: 'var(--arc-p2)' }}>O</span> links left↔right
      </div>
      <div className={styles.hexWrap}>
        {Array.from({ length: n }, (_, r) => (
          <div key={r} className={styles.hexRow} style={{ marginLeft: r * (size * 0.52), gap: size * 0.14 }}>
            {Array.from({ length: n }, (_, c) => {
              const i = r * n + c;
              const ch = board[i] ?? ' ';
              return (
                <button
                  key={c}
                  className={styles.hexCell}
                  disabled={!interactive || ch !== ' '}
                  onClick={() => onMove(String(i))}
                  style={{
                    width: size,
                    height: size,
                    background:
                      ch === 'X' ? 'var(--arc-p1)' : ch === 'O' ? 'var(--arc-p2)' : 'var(--arc-soft)',
                  }}
                  aria-label={`Cell ${i + 1}: ${ch === ' ' ? 'empty' : ch}`}
                />
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}

export const hex: GameDefinition = {
  id: 'hex',
  name: 'Hex',
  icon: '⬡',
  blurb: 'Connect your two sides with one chain of stones. It can never be a draw.',
  defaultParams: { numPlayers: 2, size: 7 },
  presets: [
    { label: 'Classic 7×7', emoji: '⭐', params: { numPlayers: 2, size: 7 } },
    { label: 'Small 5×5', emoji: '🔳', params: { numPlayers: 2, size: 5 } },
    { label: 'Big 9×9', emoji: '🔲', params: { numPlayers: 2, size: 9 } },
  ],
  knobs: [{ key: 'size', label: 'Size', min: 5, max: 11, step: 1 }],
  create: makeHandle,
  Board: HexBoard,
  playerLabels: ['X', 'O'],
};
