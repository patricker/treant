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
  // While a hidden-info handoff blackout is up, the board underneath must be
  // inert (the scrim blocks pointer events; this also blocks keyboard).
  const interactive = s.phase === 'playing' && s.seats[s.current] === 'human' && s.handoff == null;
  const handoffName =
    s.handoff != null ? (labels[s.handoff] ? t(labels[s.handoff]) : t('Player {n}', { n: s.handoff + 1 })) : '';
  // The end-of-game line, shared by the overlay card and the dismissed pill so
  // both tell the same story. A game may override the default winner line via
  // resultFlavor (e.g. a box-in "Snail trapped!" message); it falls back to the
  // standard "{winner} wins!" when the hook is absent or returns undefined.
  const endLine =
    def.solo
      ? s.endText
      : (s.phase === 'over' &&
          def.resultFlavor?.({
            result: s.result,
            winCells: s.winCells,
            board: s.board,
            labels,
            seats: s.seats,
            t,
          })) ||
        winnerLabel(t, s.result, labels, s.seats);
  const status = def.solo
    ? s.statusText
    : s.phase === 'thinking'
      ? t('🤖 Thinking…')
      : s.phase === 'over'
        ? ''
        : (() => {
            // A lone human vs the AI hears "Your turn" — "Red's turn" reads
            // like a status about someone else and kids lose track of which
            // side they are.
            const humans = s.seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
            if (humans.length === 1 && humans[0] === s.current) return t('Your turn');
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
      {s.handoff != null && (
        // Full-viewport blackout between turns so the next player picks up the
        // phone without seeing the last player's secret view. Covers the entire
        // screen (board + counters + history). Only the button dismisses it —
        // Escape does nothing, on purpose (no accidental peeking).
        <div
          className={styles.blackout}
          role="dialog"
          aria-modal="true"
          aria-label={t('Hand the phone to {name}', { name: handoffName })}
        >
          <div className={styles.blackoutInner}>
            <div className={styles.blackoutEyes} aria-hidden="true">
              👀
            </div>
            <div className={styles.blackoutTitle}>{t('Pass the phone')}</div>
            <div className={styles.blackoutText}>
              {t('Hand the phone to {name} — nobody else look!', { name: handoffName })}
            </div>
            <button className={styles.playBtn} onClick={s.dismissHandoff} autoFocus>
              {t("I'm {name} — show my board", { name: handoffName })}
            </button>
          </div>
        </div>
      )}
      <div className={styles.playTop}>
        <button className={styles.navBtn} onClick={onQuit} aria-label={t('Back to games')}>
          <span aria-hidden="true">←</span> <span className={styles.navLabel}>{t('Games')}</span>
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
            <span aria-hidden="true">↻</span> <span className={styles.navLabel}>{t('New')}</span>
          </button>
          <button
            className={`${styles.navBtn} ${showRules ? styles.navBtnOn : ''}`}
            onClick={() => setShowRules((v) => !v)}
            aria-label={t('How to play')}
          >
            <span aria-hidden="true">?</span> <span className={styles.navLabel}>{t('Rules')}</span>
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
          {!def.solo && (s.phase === 'playing' || s.phase === 'thinking') && (
            <span
              className={`${styles.turnDot} ${s.phase === 'thinking' ? styles.turnDotThinking : ''}`}
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
          winCells={s.winCells}
          lastCells={s.lastCells}
          terminal={s.phase === 'over'}
          playerNames={labels.map((l) => t(l))}
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
              role="dialog"
              aria-label={endLine}
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
              <div className={styles.overlayText}>{endLine}</div>
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
            <span>{endLine}</span>
            <button className={styles.playBtn} onClick={s.replay}>
              {t('↺ Play again')}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
