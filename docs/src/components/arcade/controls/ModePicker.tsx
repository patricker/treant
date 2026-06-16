import type { Mode } from '../gameTypes';
import styles from '../arcade.module.css';

const OPTS: { id: Mode; label: string; emoji: string }[] = [
  { id: 'pvp', label: '2 of us', emoji: '👥' },
  { id: 'pvai', label: 'vs AI', emoji: '🤖' },
  { id: 'aivai', label: 'Watch', emoji: '👀' },
];

export default function ModePicker({ value, onChange }: { value: Mode; onChange: (m: Mode) => void }) {
  return (
    <div className={styles.seg}>
      {OPTS.map((o) => (
        <button
          key={o.id}
          className={`${styles.segBtn} ${value === o.id ? styles.segOn : ''}`}
          onClick={() => onChange(o.id)}
        >
          {o.emoji} {o.label}
        </button>
      ))}
    </div>
  );
}
