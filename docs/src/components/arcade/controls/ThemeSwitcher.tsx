import { ACCENTS, type AccentId } from '../arcadeAccent';
import styles from '../arcade.module.css';

/** A row of accent swatches that recolours the arcade live. */
export default function ThemeSwitcher({
  accent,
  onChange,
}: {
  accent: AccentId;
  onChange: (a: AccentId) => void;
}) {
  return (
    <div className={styles.themeSwitch} role="group" aria-label="Arcade colour theme">
      {ACCENTS.map((a) => (
        <button
          key={a.id}
          className={`${styles.themeSwatch} ${a.id === accent ? styles.themeSwatchOn : ''}`}
          style={{ ['--sw' as string]: a.color }}
          aria-label={a.label}
          aria-pressed={a.id === accent}
          title={a.label}
          onClick={() => onChange(a.id)}
        />
      ))}
    </div>
  );
}
