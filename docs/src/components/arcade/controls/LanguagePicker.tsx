import { LOCALES, useT, type LocaleId } from '../i18n';
import styles from '../arcade.module.css';

/** Compact language selector for the arcade chrome; switches copy live. */
export default function LanguagePicker() {
  const { locale, setLocale, t } = useT();
  return (
    <label className={styles.langPicker} title={t('Language')}>
      <select
        aria-label={t('Language')}
        value={locale}
        onChange={(e) => setLocale(e.target.value as LocaleId)}
      >
        {LOCALES.map((l) => (
          <option key={l.id} value={l.id}>
            {l.flag} {l.native}
          </option>
        ))}
      </select>
    </label>
  );
}
