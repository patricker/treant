import type { GameParams, Knob } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

export default function CustomKnobs({
  knobs,
  params,
  onChange,
}: {
  knobs: Knob[];
  params: GameParams;
  onChange: (p: GameParams) => void;
}) {
  const { t } = useT();
  return (
    <div className={styles.knobs}>
      {knobs.map((kn) => {
        const v = params[kn.key];
        const set = (nv: number) =>
          onChange({ ...params, [kn.key]: Math.max(kn.min, Math.min(kn.max, nv)) });
        return (
          <div key={kn.key} className={styles.knob}>
            <span className={styles.knobLabel}>{t(kn.label)}</span>
            <div className={styles.stepper}>
              <button onClick={() => set(v - kn.step)} disabled={v <= kn.min} aria-label={t('Decrease {label}', { label: t(kn.label) })}>
                −
              </button>
              <span className={styles.knobVal}>{v}</span>
              <button onClick={() => set(v + kn.step)} disabled={v >= kn.max} aria-label={t('Increase {label}', { label: t(kn.label) })}>
                +
              </button>
            </div>
          </div>
        );
      })}
    </div>
  );
}
