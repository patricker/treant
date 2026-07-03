import { useEffect, useState } from 'react';
import type { GameDefinition, GameParams, PlayerKind } from './gameTypes';
import { PLAYER_LABEL } from './gameTypes';
import { recordRecentGame } from './recentGames';
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
  // Families want to study the final board ("how did I win?") — the game-over
  // card can be dismissed (scrim tap / ✕ / Escape) and reopens on replay.
  const [overlayDismissed, setOverlayDismissed] = useState(false);
  useEffect(() => recordRecentGame(def.id), [def.id]);
  useEffect(() => {
    if (s.phase !== 'over') setOverlayDismissed(false);
  }, [s.phase]);
  useEffect(() => {
    if (s.phase !== 'over' || overlayDismissed) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOverlayDismissed(true);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [s.phase, overlayDismissed]);
  const labels = def.playerLabels ?? PLAYER_LABEL;
  const interactive = s.phase === 'playing' && s.seats[s.current] === 'human';
  const status = def.solo
    ? s.statusText
    : s.phase === 'thinking'
      ? t('🤖 Thinking…')
      : s.phase === 'over'
        ? ''
        : (() => {
            const en = labels[s.current] ?? `Player ${s.current + 1}`;
            const name = labels[s.current] ? t(labels[s.current]) : t('Player {n}', { n: s.current + 1 });
            // Pick the key by the ENGLISH label so the branch is stable across
            // locales (non-English dicts translate both keys identically); the
            // apostrophe variant only matters for the English possessive:
            // "Goats' turn", not "Goats's turn".
            return en.endsWith('s') ? t("{name}' turn", { name }) : t("{name}'s turn", { name });
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
          {!def.solo && !def.noUndo && (
            <button
              className={styles.navBtn}
              onClick={s.undo}
              disabled={!s.canUndo}
              aria-label={t('Undo move')}
              title={t('Undo move')}
            >
              ↶
            </button>
          )}
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

      {status && (
        <div className={styles.turnBanner}>
          {!def.solo && s.phase === 'playing' && (
            <span
              className={styles.turnDot}
              style={{ background: `var(--arc-p${s.current + 1})` }}
              aria-hidden="true"
            />
          )}
          {status}
        </div>
      )}

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

        {s.phase === 'over' && !overlayDismissed && (
          <div className={styles.overlay} onClick={() => setOverlayDismissed(true)}>
            <div
              className={`${styles.overlayCard} ${styles.overlayPop}`}
              onClick={(e) => e.stopPropagation()}
            >
              <button
                className={styles.overlayClose}
                aria-label={t('View board')}
                title={t('View board')}
                onClick={() => setOverlayDismissed(true)}
              >
                ✕
              </button>
              <div className={`${styles.overlayIcon} ${styles.celebrate}`}>
                {(() => {
                  if (def.solo) return '🎮';
                  if (s.result === 'Draw') return '🤝';
                  // A lone human who lost shouldn't get a trophy.
                  const humans = s.seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
                  const winner = Number(s.result) - 1;
                  const humanLost = humans.length === 1 && Number.isFinite(winner) && humans[0] !== winner;
                  return humanLost ? '🤖' : '🏆';
                })()}
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
        {s.phase === 'over' && overlayDismissed && (
          <div className={styles.resultPill}>
            <span>{def.solo ? s.endText : winnerLabel(t, s.result, labels, s.seats)}</span>
            <button className={styles.playBtn} onClick={s.replay}>
              {t('↺ Play again')}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
