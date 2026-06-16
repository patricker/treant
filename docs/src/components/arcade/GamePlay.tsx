import { useState } from 'react';
import type { GameDefinition, GameParams, PlayerKind } from './gameTypes';
import { PLAYER_LABEL } from './gameTypes';
import { useGameSession } from './useGameSession';
import { GameIcon } from './icons';
import { gameRules } from './rules';
import styles from './arcade.module.css';

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
  seats,
  onQuit,
  onChangeSetup,
}: {
  wasm: any;
  def: GameDefinition;
  params: GameParams;
  seats: PlayerKind[];
  onQuit: () => void;
  onChangeSetup: () => void;
}) {
  const s = useGameSession(wasm, def, params, seats);
  const [hint, setHint] = useState('');
  const [showRules, setShowRules] = useState(false);
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
        <button className={styles.navBtn} onClick={onQuit}>
          ← Games
        </button>
        <span className={styles.playTitle}>
          <GameIcon id={def.id} size={20} /> {def.name}
        </span>
        <div className={styles.playTopRight}>
          <button className={styles.navBtn} onClick={onChangeSetup} aria-label="New game / change setup">
            ↻ New
          </button>
          <button
            className={`${styles.navBtn} ${showRules ? styles.navBtnOn : ''}`}
            onClick={() => setShowRules((v) => !v)}
            aria-label="How to play"
          >
            ? Rules
          </button>
        </div>
      </div>

      {showRules && (
        <div className={styles.rulesPanel}>
          <strong>How to play {def.name}</strong>
          <p>{gameRules(def.id, def.rules, def.blurb)}</p>
        </div>
      )}

      {status && <div className={styles.turnBanner}>{status}</div>}

      <div className={styles.boardWrap}>
        <def.Board
          board={s.board}
          params={params}
          currentPlayer={s.current}
          interactive={interactive}
          legalMoves={s.legalMoves}
          onMove={s.onHumanMove}
        />

        {def.solo && s.phase === 'playing' && (
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
