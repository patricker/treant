import { useState } from 'react';
import type { GameDefinition, GameParams, PlayerKind } from './gameTypes';
import { PLAYER_LABEL } from './gameTypes';
import { useGameSession } from './useGameSession';
import { GameIcon } from './icons';
import { gameRules } from './rules';
import { useT, type I18n } from './i18n';
import styles from './arcade.module.css';

function winnerLabel(t: I18n['t'], result: string, labels: string[], seats: PlayerKind[]): string {
  if (result === 'Draw') return t("It's a draw!");
  const n = Number(result);
  if (!Number.isFinite(n)) return t('Player {n} wins!', { n: result });
  const winnerSeat = n - 1;
  const name = labels[winnerSeat] ? t(labels[winnerSeat]) : t('Player {n}', { n });
  // In a single-human game (vs AI), speak to the player directly so a loss is
  // unmistakable — "Hounds wins!" reads as a neutral status, not "you lost".
  const humanSeats = seats.map((s, i) => (s === 'human' ? i : -1)).filter((i) => i >= 0);
  if (humanSeats.length === 1) {
    return humanSeats[0] === winnerSeat ? t('You win! 🎉') : t('You lost — {name} wins.', { name });
  }
  return t('{name} wins!', { name });
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
  const { t } = useT();
  const s = useGameSession(wasm, def, params, seats);
  const [hint, setHint] = useState('');
  const [showRules, setShowRules] = useState(false);
  const labels = def.playerLabels ?? PLAYER_LABEL;
  const interactive = s.phase === 'playing' && s.seats[s.current] === 'human';
  const status = def.solo
    ? s.statusText
    : s.phase === 'thinking'
      ? t('🤖 Thinking…')
      : s.phase === 'over'
        ? ''
        : (() => {
            const name = labels[s.current] ? t(labels[s.current]) : t('Player {n}', { n: s.current + 1 });
            // English possessive: "Goats' turn", not "Goats's turn".
            return name.endsWith('s') ? t("{name}' turn", { name }) : t("{name}'s turn", { name });
          })();

  return (
    <div className={styles.play}>
      <div className={styles.playTop}>
        <button className={styles.navBtn} onClick={onQuit}>
          {t('← Games')}
        </button>
        <span className={styles.playTitle}>
          <GameIcon id={def.id} size={20} /> {t(def.name)}
        </span>
        <div className={styles.playTopRight}>
          <button className={styles.navBtn} onClick={onChangeSetup} aria-label={t('New game / change setup')}>
            {t('↻ New')}
          </button>
          <button
            className={`${styles.navBtn} ${showRules ? styles.navBtnOn : ''}`}
            onClick={() => setShowRules((v) => !v)}
            aria-label={t('How to play')}
          >
            {t('? Rules')}
          </button>
        </div>
      </div>

      {showRules && (
        <div className={styles.rulesPanel}>
          <strong>{t('How to play {name}', { name: t(def.name) })}</strong>
          <p>{t(gameRules(def.id, def.rules, def.blurb))}</p>
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
              {t('💡 Hint')}
            </button>
            {hint && <span className={styles.hintText}>{t('Try {hint}', { hint })}</span>}
          </div>
        )}

        {s.phase === 'over' && (
          <div className={styles.overlay}>
            <div className={`${styles.overlayCard} ${styles.overlayPop}`}>
              <div className={`${styles.overlayIcon} ${styles.celebrate}`}>
                {def.solo ? '🎮' : s.result === 'Draw' ? '🤝' : '🏆'}
              </div>
              <div className={styles.overlayText}>
                {def.solo ? s.endText : winnerLabel(t, s.result, labels, s.seats)}
              </div>
              <button className={styles.playBtn} onClick={s.replay}>
                {t('↺ Play again')}
              </button>
              <button className={styles.secondaryBtn} onClick={onChangeSetup}>
                {t('⚙ Change setup')}
              </button>
              <button className={styles.ghostBtn} onClick={onQuit}>
                {t('🏠 Arcade')}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
