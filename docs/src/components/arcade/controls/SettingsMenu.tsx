import { useState } from 'react';
import type { AccentId } from '../arcadeAccent';
import { useT } from '../i18n';
import ThemeSwitcher from './ThemeSwitcher';
import LanguagePicker from './LanguagePicker';
import MuteToggle from './MuteToggle';
import styles from '../arcade.module.css';

/**
 * One small ⚙ button holding every rarely-touched setting (accent theme,
 * language, sound) in a popover, so the arcade screens carry no permanent
 * settings chrome at all.
 */
export default function SettingsMenu({
  accent,
  onAccent,
}: {
  accent: AccentId;
  onAccent: (a: AccentId) => void;
}) {
  const { t } = useT();
  const [open, setOpen] = useState(false);
  return (
    <div className={styles.settingsWrap}>
      <button
        className={styles.settingsBtn}
        aria-label={t('Settings')}
        aria-expanded={open}
        title={t('Settings')}
        onClick={() => setOpen((v) => !v)}
      >
        ⚙
      </button>
      {open && (
        <>
          <div className={styles.settingsBackdrop} onClick={() => setOpen(false)} />
          <div className={styles.settingsPanel} role="dialog" aria-label={t('Settings')}>
            <div className={styles.settingsRow}>
              <span className={styles.settingsLabel}>{t('Theme')}</span>
              <ThemeSwitcher accent={accent} onChange={onAccent} />
            </div>
            <div className={styles.settingsRow}>
              <span className={styles.settingsLabel}>{t('Language')}</span>
              <LanguagePicker />
            </div>
            <div className={styles.settingsRow}>
              <span className={styles.settingsLabel}>{t('Sound')}</span>
              <MuteToggle />
            </div>
            <div className={styles.settingsFoot}>
              <a href="/">Treant · MIT</a>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
