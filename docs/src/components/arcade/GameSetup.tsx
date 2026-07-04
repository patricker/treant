import { useEffect, useState } from 'react';
import type { Difficulty, GameDefinition, GameParams, PlayerKind } from './gameTypes';
import { DIFFICULTY, PLAYER_LABEL, encodeSeats, seatsForMode } from './gameTypes';
import { gameRules } from './rules';
import { GameIcon } from './icons';
import { useT } from './i18n';
import PresetChips from './controls/PresetChips';
import CustomKnobs from './controls/CustomKnobs';
import styles from './arcade.module.css';

const SEAT_COLORS = ['var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)', 'var(--arc-p5)', 'var(--arc-p6)'];
const SEAT_OPTS: { kind: PlayerKind; emoji: string; label: string }[] = [
  { kind: 'human', emoji: '🧑', label: 'Human' },
  { kind: 'easy', emoji: '😊', label: 'Easy' },
  { kind: 'medium', emoji: '😎', label: 'Med' },
  { kind: 'hard', emoji: '🔥', label: 'Hard' },
];

/** Which quick mode (if any) the current seat line-up matches. */
function modeOf(seats: PlayerKind[]): 'pvp' | 'pvai' | 'aivai' | 'custom' {
  const humans = seats.filter((s) => s === 'human').length;
  if (humans === seats.length) return 'pvp';
  if (humans === 0) return 'aivai';
  if (humans === 1 && seats[0] === 'human') return 'pvai';
  return 'custom';
}

export default function GameSetup({
  def,
  wasm,
  onStart,
  onBack,
  initialSeats,
}: {
  def: GameDefinition;
  wasm?: any;
  onStart: (cfg: { params: GameParams; seats: PlayerKind[] }) => void;
  onBack: () => void;
  initialSeats?: PlayerKind[];
}) {
  const { t } = useT();
  const [params, setParams] = useState<GameParams>(def.defaultParams);
  // Live board preview: a throwaway engine renders the real board at the
  // chosen knob values, so "Mega 10×10" is something you SEE, not imagine.
  const [previewBoard, setPreviewBoard] = useState('');
  useEffect(() => {
    if (!wasm) return;
    try {
      const h = def.create(wasm, params);
      setPreviewBoard(h.getBoard());
      h.free();
    } catch {
      setPreviewBoard('');
    }
  }, [wasm, def, params]);
  const numPlayers = params.numPlayers ?? 2;
  const [aiStrength, setAiStrength] = useState<Difficulty>('medium');
  const [seats, setSeats] = useState<PlayerKind[]>(
    () => initialSeats ?? seatsForMode(def.solo ? 'solo' : 'pvai', numPlayers, 'medium'),
  );
  const [showPlayers, setShowPlayers] = useState(false);
  const [showCustom, setShowCustom] = useState(false);
  const [showRules, setShowRules] = useState(false);
  const [shared, setShared] = useState('');

  // Keep the seat list the same length as the player count as the knobs change.
  useEffect(() => {
    setSeats((prev) => {
      if (prev.length === numPlayers) return prev;
      const next = prev.slice(0, numPlayers);
      while (next.length < numPlayers) next.push(prev[prev.length - 1] ?? aiStrength);
      return next;
    });
  }, [numPlayers]); // eslint-disable-line react-hooks/exhaustive-deps

  const labels = def.playerLabels ?? PLAYER_LABEL;
  const mode = modeOf(seats);

  const applyMode = (m: 'pvp' | 'pvai' | 'aivai') => setSeats(seatsForMode(m, numPlayers, aiStrength));
  const applyStrength = (d: Difficulty) => {
    setAiStrength(d);
    setSeats((prev) => prev.map((s) => (s === 'human' ? 'human' : d)));
  };
  const setSeat = (i: number, kind: PlayerKind) => setSeats((prev) => prev.map((s, j) => (j === i ? kind : s)));

  const share = () => {
    if (typeof window === 'undefined') return;
    const url = `${window.location.origin}${window.location.pathname}?game=${def.id}&s=${encodeSeats(seats)}`;
    const clip = navigator?.clipboard;
    if (clip?.writeText) clip.writeText(url).then(() => setShared(t('Link copied!'))).catch(() => setShared(url));
    else setShared(url);
  };

  return (
    <div className={styles.setup}>
      <div className={styles.setupHeader}>
        <button className={styles.backBtn} onClick={onBack} aria-label={t('Back to arcade')}>
          ←
        </button>
        <h2>
          <GameIcon id={def.id} size={26} /> {t(def.name)}
        </h2>
      </div>

      <button
        className={`${styles.howToBtn} ${showRules ? styles.howToOn : ''}`}
        onClick={() => setShowRules((v) => !v)}
      >
        {t('📖 How to play')} {showRules ? '▾' : '▸'}
      </button>
      {showRules && <div className={styles.rulesPanel}>{t(gameRules(def.id, def.rules, def.blurb))}</div>}

      {def.solo ? (
        <>
          <div className={styles.setupLabel}>{t('Mode')}</div>
          <div className={styles.seg}>
            <button
              className={`${styles.segBtn} ${seats[0] === 'human' ? styles.segOn : ''}`}
              onClick={() => setSeats(['human'])}
            >
              {t('🙂 You play')}
            </button>
            <button
              className={`${styles.segBtn} ${seats[0] !== 'human' ? styles.segOn : ''}`}
              onClick={() => setSeats(['medium'])}
            >
              {t('🤖 Watch AI')}
            </button>
          </div>
        </>
      ) : (
        <>
          <div className={styles.setupLabel}>{t("Who's playing?")}</div>
          <div className={styles.seg}>
            <button className={`${styles.segBtn} ${mode === 'pvp' ? styles.segOn : ''}`} onClick={() => applyMode('pvp')}>
              {t('👥 Pass & play')}
            </button>
            <button className={`${styles.segBtn} ${mode === 'pvai' ? styles.segOn : ''}`} onClick={() => applyMode('pvai')}>
              {t('🤖 vs AI')}
            </button>
            <button className={`${styles.segBtn} ${mode === 'aivai' ? styles.segOn : ''}`} onClick={() => applyMode('aivai')}>
              {t('👀 Watch')}
            </button>
          </div>

          {seats.some((s) => s !== 'human') && (
            <>
              <div className={styles.setupLabel}>{t('AI strength')}</div>
              <div className={styles.seg}>
                {(Object.keys(DIFFICULTY) as Difficulty[]).map((d) => (
                  <button
                    key={d}
                    className={`${styles.segBtn} ${aiStrength === d ? styles.segOn : ''}`}
                    onClick={() => applyStrength(d)}
                  >
                    {DIFFICULTY[d].emoji} {t(DIFFICULTY[d].label)}
                  </button>
                ))}
              </div>
            </>
          )}

          <button
            className={styles.customToggle}
            onClick={() => setShowPlayers((v) => !v)}
          >
            {showPlayers ? '▾' : '▸'} {t('Customize players')} {mode === 'custom' ? t('• custom') : ''}
          </button>
          {showPlayers && (
            <div className={styles.playerList}>
              {seats.map((kind, i) => (
                <div key={i} className={styles.playerRow}>
                  <span className={styles.playerTag}>
                    <span className={styles.playerDot} style={{ background: SEAT_COLORS[i] }} />
                    {labels[i] ? t(labels[i]) : t('Player {n}', { n: i + 1 })}
                  </span>
                  <div className={styles.seatSeg}>
                    {SEAT_OPTS.map((o) => (
                      <button
                        key={o.kind}
                        className={`${styles.seatBtn} ${kind === o.kind ? styles.seatOn : ''}`}
                        onClick={() => setSeat(i, o.kind)}
                        title={o.kind === 'human' ? t('Human') : t('{level} AI', { level: t(DIFFICULTY[o.kind as Difficulty].label) })}
                        aria-label={o.kind === 'human' ? t('Human') : t('{level} AI', { level: t(DIFFICULTY[o.kind as Difficulty].label) })}
                        aria-pressed={kind === o.kind}
                      >
                        {o.emoji}
                      </button>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {def.presets.length > 0 && (
        <>
          <div className={styles.setupLabel}>{t('Board')}</div>
          <PresetChips presets={def.presets} active={params} onPick={setParams} />
        </>
      )}
      {previewBoard && (
        <div className={styles.setupPreview} aria-hidden="true">
          <def.Board
            board={previewBoard}
            params={params}
            currentPlayer={0}
            interactive={false}
            legalMoves={[]}
            onMove={() => {}}
          />
        </div>
      )}
      {def.knobs.length > 0 && (
        <>
          <button className={styles.customToggle} onClick={() => setShowCustom((s) => !s)}>
            {showCustom ? '▾' : '▸'} {t('Customize board')}
          </button>
          {showCustom && <CustomKnobs knobs={def.knobs} params={params} onChange={setParams} />}
        </>
      )}

      <button className={styles.playBtn} onClick={() => onStart({ params, seats })}>
        {t('▶ Start game')}
      </button>

      <button className={styles.shareBtn} onClick={share}>
        {t('🔗 Share this game')}
      </button>
      {shared && <div className={styles.sharedText}>{shared}</div>}
    </div>
  );
}
