import type { GameParams, Preset } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// Compare the FULL param set: presets are keyed on target/stones/men/size and
// (soon) rule flags, not just grid dims — two presets must never both light up.
function same(a: GameParams, b: GameParams) {
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  return [...keys].every((k) => (a[k] ?? 0) === (b[k] ?? 0));
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
