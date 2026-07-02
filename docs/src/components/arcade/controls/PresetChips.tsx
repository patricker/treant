import type { GameParams, Preset } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

function same(a: GameParams, b: GameParams) {
  return a.cols === b.cols && a.rows === b.rows && a.k === b.k && a.numPlayers === b.numPlayers;
}

export default function PresetChips({
  presets,
  active,
  onPick,
}: {
  presets: Preset[];
  active: GameParams;
  onPick: (p: GameParams) => void;
}) {
  const { t } = useT();
  return (
    <div className={styles.chipRow}>
      {presets.map((p) => (
        <button
          key={p.label}
          className={`${styles.chip} ${same(active, p.params) ? styles.chipOn : ''}`}
          onClick={() => onPick(p.params)}
        >
          {p.emoji} {t(p.label)}
        </button>
      ))}
    </div>
  );
}
