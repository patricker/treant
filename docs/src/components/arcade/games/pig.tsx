import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

const SEAT = ['var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)', 'var(--arc-p5)', 'var(--arc-p6)'];
const PIPS: Record<number, string> = { 1: '⚀', 2: '⚁', 3: '⚂', 4: '⚃', 5: '⚄', 6: '⚅' };

function makeHandle(wasm: any, p: GameParams, variant: number): GameHandle {
  const g = new wasm.PigWasm(p.numPlayers, p.target, variant);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => (g.is_terminal() ? [] : ['Roll', 'Hold']),
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

function PigBoard({ board, currentPlayer, interactive, onMove }: BoardProps) {
  const { t } = useT();
  // "s0,s1,...|turn|roll|target" — the two-dice variants append "|roll2".
  const [scoresStr, turnStr, rollStr, targetStr, roll2Str] = board.split('|');
  const scores = (scoresStr ?? '0').split(',').map(Number);
  const turn = Number(turnStr) || 0;
  const roll = Number(rollStr) || 0;
  const roll2 = Number(roll2Str) || 0;
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
        <div className={styles.pigDie}>
          {roll ? PIPS[roll] : '🎲'}
          {roll2 ? PIPS[roll2] : ''}
        </div>
        <div className={styles.pigTurn}>
          {t('Turn total {turn} · first to {target}', { turn, target })}
        </div>
      </div>

      <div className={styles.pigButtons}>
        <button className={styles.pigRoll} disabled={!interactive} onClick={() => onMove('Roll')}>
          {t('🎲 Roll')}
        </button>
        <button className={styles.pigHold} disabled={!interactive || turn === 0} onClick={() => onMove('Hold')}>
          {t('✋ Hold ({turn})', { turn })}
        </button>
      </div>
    </div>
  );
}

export const pig: GameDefinition = {
  id: 'pig',
  noUndo: true, // dice game — replay-based undo would reroll the rolls
  name: 'Pig',
  icon: '🎲',
  blurb: 'Roll to build points — but a 1 wipes your turn. Bank before you bust!',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, target: 100 },
  presets: [
    { label: 'Classic (100)', emoji: '⭐', params: { numPlayers: 2, target: 100 } },
    { label: 'Quick (50)', emoji: '⚡', params: { numPlayers: 2, target: 50 } },
    { label: '4-Player', emoji: '🎉', params: { numPlayers: 4, target: 100 } },
    { label: '6-Player Frenzy', emoji: '🎊', params: { numPlayers: 6, target: 100 } },
    { label: 'Sprint (20)', emoji: '🏎️', params: { numPlayers: 2, target: 20 } },
    { label: 'Marathon (200)', emoji: '🤯', params: { numPlayers: 2, target: 200 } },
  ],
  knobs: [
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
    { key: 'target', label: 'Target', min: 20, max: 200, step: 5 },
  ],
  create: (wasm, p) => makeHandle(wasm, p, 0),
  Board: PigBoard,
  playerLabels: ['Player 1', 'Player 2', 'Player 3', 'Player 4'],
};

export const twoDicePig: GameDefinition = {
  id: 'two-dice-pig',
  noUndo: true, // dice game — replay-based undo would reroll the rolls
  name: 'Two-Dice Pig',
  icon: '🎲',
  blurb: 'Two dice, double the pace — but snake eyes eat your whole score.',
  difficulty: pig.difficulty, // same press-your-luck shape as classic Pig (see plans/ai-calibration-results.md)
  defaultParams: { numPlayers: 2, target: 100 },
  presets: [
    { label: 'Classic (100)', emoji: '⭐', params: { numPlayers: 2, target: 100 } },
    { label: 'Quick (50)', emoji: '⚡', params: { numPlayers: 2, target: 50 } },
    { label: '4-Player', emoji: '🎉', params: { numPlayers: 4, target: 100 } },
    { label: '6-Player Frenzy', emoji: '🎊', params: { numPlayers: 6, target: 100 } },
    { label: 'Sprint (20)', emoji: '🏎️', params: { numPlayers: 2, target: 20 } },
    { label: 'Marathon (200)', emoji: '🤯', params: { numPlayers: 2, target: 200 } },
  ],
  knobs: [
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
    { key: 'target', label: 'Target', min: 20, max: 200, step: 5 },
  ],
  create: (wasm, p) => makeHandle(wasm, p, 1),
  Board: PigBoard,
  playerLabels: ['Player 1', 'Player 2', 'Player 3', 'Player 4'],
};

export const bigPig: GameDefinition = {
  id: 'big-pig',
  noUndo: true, // dice game — replay-based undo would reroll the rolls
  name: 'Big Pig',
  icon: '🐷',
  blurb: 'Doubles pay double and snake eyes pay 25. Greed, rewarded. Mostly.',
  difficulty: pig.difficulty, // same press-your-luck shape as classic Pig (see plans/ai-calibration-results.md)
  defaultParams: { numPlayers: 2, target: 100 },
  presets: [
    { label: 'Classic (100)', emoji: '⭐', params: { numPlayers: 2, target: 100 } },
    { label: 'Quick (50)', emoji: '⚡', params: { numPlayers: 2, target: 50 } },
    { label: '4-Player', emoji: '🎉', params: { numPlayers: 4, target: 100 } },
    { label: '6-Player Frenzy', emoji: '🎊', params: { numPlayers: 6, target: 100 } },
    { label: 'Sprint (20)', emoji: '🏎️', params: { numPlayers: 2, target: 20 } },
    { label: 'Marathon (200)', emoji: '🤯', params: { numPlayers: 2, target: 200 } },
  ],
  knobs: [
    { key: 'numPlayers', label: 'Players', min: 2, max: 6, step: 1 },
    { key: 'target', label: 'Target', min: 20, max: 200, step: 5 },
  ],
  create: (wasm, p) => makeHandle(wasm, p, 2),
  Board: PigBoard,
  playerLabels: ['Player 1', 'Player 2', 'Player 3', 'Player 4'],
};
