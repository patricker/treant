import { useState } from 'react';
import type { GameDefinition, GameParams, PlayerKind } from './gameTypes';
import { PLAYER_LABEL } from './gameTypes';
import { useGameSession } from './useGameSession';
import { GameIcon } from './icons';
import { gameRules } from './rules';
import styles from './arcade.module.css';

function winnerLabel(result: string, labels: string[], seats: PlayerKind[]): string {
  if (result === 'Draw') return "It's a draw!";
  const n = Number(result);
  if (!Number.isFinite(n)) return `Player ${result} wins!`;
  const winnerSeat = n - 1;
  const name = labels[winnerSeat] ?? `Player ${n}`;
  // In a single-human game (vs AI), speak to the player directly so a loss is
  // unmistakable — "Hounds wins!" reads as a neutral status, not "you lost".
  const humanSeats = seats.map((s, i) => (s === 'human' ? i : -1)).filter((i) => i >= 0);
  if (humanSeats.length === 1) {
    return humanSeats[0] === winnerSeat ? 'You win! 🎉' : `You lost — ${name} wins.`;
  }
  return `${name} wins!`;
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
                {def.solo ? s.endText : winnerLabel(s.result, labels, s.seats)}
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
