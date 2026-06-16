import type { Difficulty, GameDefinition, GameParams, Mode } from './gameTypes';
import { useGameSession } from './useGameSession';
import styles from './arcade.module.css';

const PLAYER_LABEL = ['Red', 'Yellow', 'Green', 'Purple'];

function winnerLabel(result: string): string {
  if (result === 'Draw') return "It's a draw!";
  const n = Number(result);
  const name = Number.isFinite(n) ? PLAYER_LABEL[n - 1] : undefined;
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
  const interactive = s.phase === 'playing' && s.seats[s.current] === 'human';
  const status =
    s.phase === 'thinking'
      ? '🤖 Thinking…'
      : s.phase === 'over'
        ? ''
        : `${PLAYER_LABEL[s.current] ?? `Player ${s.current + 1}`}'s turn`;

  return (
    <div className={styles.play}>
      <div className={styles.playTop}>
        <span>
          {def.icon} {def.name}
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

        {s.phase === 'over' && (
          <div className={styles.overlay}>
            <div className={styles.overlayCard}>
              <div className={styles.overlayIcon}>{s.result === 'Draw' ? '🤝' : '🏆'}</div>
              <div className={styles.overlayText}>{winnerLabel(s.result)}</div>
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
