import { useState } from 'react';
import type { Difficulty, GameDefinition, GameParams, Mode } from './gameTypes';
import { buildShareUrl } from './shareLink';
import { GameIcon } from './icons';
import ModePicker from './controls/ModePicker';
import DifficultyPicker from './controls/DifficultyPicker';
import PresetChips from './controls/PresetChips';
import CustomKnobs from './controls/CustomKnobs';
import styles from './arcade.module.css';

export default function GameSetup({
  def,
  onStart,
  onBack,
  initialMode,
  initialDifficulty,
}: {
  def: GameDefinition;
  onStart: (cfg: { params: GameParams; mode: Mode; difficulty: Difficulty }) => void;
  onBack: () => void;
  initialMode?: Mode;
  initialDifficulty?: Difficulty;
}) {
  const [mode, setMode] = useState<Mode>(initialMode ?? (def.solo ? 'solo' : 'pvai'));
  const [difficulty, setDifficulty] = useState<Difficulty>(initialDifficulty ?? 'medium');
  const [params, setParams] = useState<GameParams>(def.defaultParams);
  const [showCustom, setShowCustom] = useState(false);
  const [shared, setShared] = useState('');

  const share = () => {
    const url = buildShareUrl(def.id, mode, difficulty, !def.solo && mode !== 'pvp');
    const clip = typeof navigator !== 'undefined' ? navigator.clipboard : undefined;
    if (clip?.writeText) {
      clip.writeText(url).then(() => setShared('Link copied!')).catch(() => setShared(url));
    } else {
      setShared(url);
    }
  };

  return (
    <div className={styles.setup}>
      <div className={styles.setupHeader}>
        <button className={styles.backBtn} onClick={onBack} aria-label="Back to arcade">
          ←
        </button>
        <h2>
          <GameIcon id={def.id} size={26} /> {def.name}
        </h2>
      </div>

      <div className={styles.setupLabel}>{def.solo ? 'Mode' : "Who's playing?"}</div>
      {def.solo ? (
        <div className={styles.seg}>
          <button
            className={`${styles.segBtn} ${mode === 'solo' ? styles.segOn : ''}`}
            onClick={() => setMode('solo')}
          >
            🙂 You play
          </button>
          <button
            className={`${styles.segBtn} ${mode === 'aivai' ? styles.segOn : ''}`}
            onClick={() => setMode('aivai')}
          >
            🤖 Watch AI
          </button>
        </div>
      ) : (
        <>
          <ModePicker value={mode} onChange={setMode} />
          {mode !== 'pvp' && (
            <>
              <div className={styles.setupLabel}>AI strength</div>
              <DifficultyPicker value={difficulty} onChange={setDifficulty} />
            </>
          )}
        </>
      )}

      {def.presets.length > 0 && (
        <>
          <div className={styles.setupLabel}>Board</div>
          <PresetChips presets={def.presets} active={params} onPick={setParams} />
        </>
      )}
      {def.knobs.length > 0 && (
        <>
          <button className={styles.customToggle} onClick={() => setShowCustom((s) => !s)}>
            {showCustom ? '▾' : '▸'} Customize
          </button>
          {showCustom && <CustomKnobs knobs={def.knobs} params={params} onChange={setParams} />}
        </>
      )}

      <button className={styles.playBtn} onClick={() => onStart({ params, mode, difficulty })}>
        ▶ Start game
      </button>

      <button className={styles.shareBtn} onClick={share}>
        🔗 Share this game
      </button>
      {shared && <div className={styles.sharedText}>{shared}</div>}
    </div>
  );
}
