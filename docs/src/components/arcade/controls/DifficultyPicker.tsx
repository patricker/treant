import type { Difficulty } from '../gameTypes';
import { DIFFICULTY } from '../gameTypes';
import styles from '../arcade.module.css';

export default function DifficultyPicker({
  value,
  onChange,
}: {
  value: Difficulty;
  onChange: (d: Difficulty) => void;
}) {
  return (
    <div className={styles.seg}>
      {(Object.keys(DIFFICULTY) as Difficulty[]).map((d) => (
        <button
          key={d}
          className={`${styles.segBtn} ${value === d ? styles.segOn : ''}`}
          onClick={() => onChange(d)}
        >
          {DIFFICULTY[d].emoji} {DIFFICULTY[d].label}
        </button>
      ))}
    </div>
  );
}
