import { useState } from 'react';
import type { Difficulty, GameDefinition, GameParams, Mode } from './gameTypes';
import ModePicker from './controls/ModePicker';
import DifficultyPicker from './controls/DifficultyPicker';
import PresetChips from './controls/PresetChips';
import CustomKnobs from './controls/CustomKnobs';
import styles from './arcade.module.css';

export default function GameSetup({
  def,
  onStart,
  onBack,
}: {
  def: GameDefinition;
  onStart: (cfg: { params: GameParams; mode: Mode; difficulty: Difficulty }) => void;
  onBack: () => void;
}) {
  const [mode, setMode] = useState<Mode>('pvai');
  const [difficulty, setDifficulty] = useState<Difficulty>('medium');
  const [params, setParams] = useState<GameParams>(def.defaultParams);
  const [showCustom, setShowCustom] = useState(false);

  return (
    <div className={styles.setup}>
      <div className={styles.setupHeader}>
        <button className={styles.backBtn} onClick={onBack} aria-label="Back to arcade">
          ←
        </button>
        <h2>
          {def.icon} {def.name}
        </h2>
      </div>

      <div className={styles.setupLabel}>Who&apos;s playing?</div>
      <ModePicker value={mode} onChange={setMode} />

      {mode !== 'pvp' && (
        <>
          <div className={styles.setupLabel}>AI strength</div>
          <DifficultyPicker value={difficulty} onChange={setDifficulty} />
        </>
      )}

      <div className={styles.setupLabel}>Board</div>
      <PresetChips presets={def.presets} active={params} onPick={setParams} />
      <button className={styles.customToggle} onClick={() => setShowCustom((s) => !s)}>
        {showCustom ? '▾' : '▸'} Customize
      </button>
      {showCustom && <CustomKnobs knobs={def.knobs} params={params} onChange={setParams} />}

      <button className={styles.playBtn} onClick={() => onStart({ params, mode, difficulty })}>
        ▶ Start game
      </button>
    </div>
  );
}
