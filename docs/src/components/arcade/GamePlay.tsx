import { useState } from 'react';
import type { Difficulty, GameDefinition, GameParams, Mode } from './gameTypes';
import { useGameSession } from './useGameSession';
import { GameIcon } from './icons';
import styles from './arcade.module.css';

const PLAYER_LABEL = ['Red', 'Yellow', 'Green', 'Purple'];

function winnerLabel(result: string, labels: string[]): string {
  if (result === 'Draw') return "It's a draw!";
  const n = Number(result);
  const name = Number.isFinite(n) ? labels[n - 1] : undefined;
  return `${name ?? `Player ${result}`} wins!`;
}

export default function GamePlay({
  wasm,
  def,
  params,
  mode,
  difficulty,
  onQuit,
  onChangeSetup,
}: {
  wasm: any;
  def: GameDefinition;
  params: GameParams;
  mode: Mode;
  difficulty: Difficulty;
  onQuit: () => void;
  onChangeSetup: () => void;
}) {
  const s = useGameSession(wasm, def, params, mode, difficulty);
  const [hint, setHint] = useState('');
  const labels = def.playerLabels ?? PLAYER_LABEL;
  const interactive = s.phase === 'playing' && s.seats[s.current] === 'human';
  const status = def.solo
    ? s.statusText
    : s.phase === 'thinking'
      ? '🤖 Thinking…'
      : s.phase === 'over'
        ? ''
        : `${labels[s.current] ?? `Player ${s.current + 1}`}'s turn`;

  return (
    <div className={styles.play}>
      <div className={styles.playTop}>
        <span>
          <GameIcon id={def.id} size={22} /> {def.name}
        </span>
        <button className={styles.quitBtn} onClick={onQuit} aria-label="Quit to arcade">
          ✕
        </button>
      </div>

      {status && <div className={styles.turnBanner}>{status}</div>}

      <div className={styles.boardWrap}>
        <def.Board
          board={s.board}
          params={params}
          currentPlayer={s.current}
          interactive={interactive}
          onMove={s.onHumanMove}
        />

        {def.solo && mode === 'solo' && s.phase === 'playing' && (
          <div className={styles.hintRow}>
            <button
              className={styles.hintBtn}
              onClick={() => {
                const m = s.getHint?.();
                setHint(m ? (def.formatHint ? def.formatHint(m) : m) : '');
              }}
            >
              💡 Hint
            </button>
            {hint && <span className={styles.hintText}>Try {hint}</span>}
          </div>
        )}

        {s.phase === 'over' && (
          <div className={styles.overlay}>
            <div className={`${styles.overlayCard} ${styles.overlayPop}`}>
              <div className={`${styles.overlayIcon} ${styles.celebrate}`}>
                {def.solo ? '🎮' : s.result === 'Draw' ? '🤝' : '🏆'}
              </div>
              <div className={styles.overlayText}>
                {def.solo ? s.endText : winnerLabel(s.result, labels)}
              </div>
              <button className={styles.playBtn} onClick={s.replay}>
                ↺ Play again
              </button>
              <button className={styles.secondaryBtn} onClick={onChangeSetup}>
                ⚙ Change setup
              </button>
              <button className={styles.ghostBtn} onClick={onQuit}>
                🏠 Arcade
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
