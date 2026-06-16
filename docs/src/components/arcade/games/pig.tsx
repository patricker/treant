import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';

const SEAT = ['var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)', 'var(--arc-p5)', 'var(--arc-p6)'];
const PIPS: Record<number, string> = { 1: '⚀', 2: '⚁', 3: '⚂', 4: '⚃', 5: '⚄', 6: '⚅' };

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.PigWasm(p.numPlayers, p.target);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => (g.is_terminal() ? [] : ['Roll', 'Hold']),
    free: () => g.free(),
  };
}

function PigBoard({ board, currentPlayer, interactive, onMove }: BoardProps) {
  // "s0,s1,...|turn|roll|target"
  const [scoresStr, turnStr, rollStr, targetStr] = board.split('|');
  const scores = (scoresStr ?? '0').split(',').map(Number);
  const turn = Number(turnStr) || 0;
  const roll = Number(rollStr) || 0;
  const target = Number(targetStr) || 100;

  return (
    <div className={styles.pigWrap}>
      <div className={styles.pigScores}>
        {scores.map((sc, i) => (
          <div key={i} className={`${styles.pigRow} ${i === currentPlayer ? styles.pigRowActive : ''}`}>
            <span className={styles.pigName} style={{ color: SEAT[i] }}>
              P{i + 1}
            </span>
            <div className={styles.pigBar}>
              <div
                className={styles.pigBarFill}
                style={{ width: `${Math.min(100, (sc / target) * 100)}%`, background: SEAT[i] }}
              />
            </div>
            <span className={styles.pigVal}>{sc}</span>
          </div>
        ))}
      </div>

      <div className={styles.pigCenter}>
        <div className={styles.pigDie}>{roll ? PIPS[roll] : '🎲'}</div>
        <div className={styles.pigTurn}>
          Turn total <strong>{turn}</strong> · first to {target}
        </div>
      </div>

      <div className={styles.pigButtons}>
        <button className={styles.pigRoll} disabled={!interactive} onClick={() => onMove('Roll')}>
          🎲 Roll
        </button>
        <button className={styles.pigHold} disabled={!interactive || turn === 0} onClick={() => onMove('Hold')}>
          ✋ Hold ({turn})
        </button>
      </div>
    </div>
  );
}

export const pig: GameDefinition = {
  id: 'pig',
  name: 'Pig',
  icon: '🎲',
  blurb: 'Roll to build points — but a 1 wipes your turn. Bank before you bust!',
  defaultParams: { numPlayers: 2, target: 100 },
  presets: [
    { label: 'Classic (100)', emoji: '⭐', params: { numPlayers: 2, target: 100 } },
    { label: 'Quick (50)', emoji: '⚡', params: { numPlayers: 2, target: 50 } },
    { label: '4-Player', emoji: '🎉', params: { numPlayers: 4, target: 100 } },
  ],
  knobs: [
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
    { key: 'target', label: 'Target', min: 50, max: 150, step: 10 },
  ],
  create: makeHandle,
  Board: PigBoard,
  playerLabels: ['Player 1', 'Player 2', 'Player 3', 'Player 4'],
};
