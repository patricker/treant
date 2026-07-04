import { useState } from 'react';
import { isMuted, setMuted, unlockAudio } from '../sound';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

export default function MuteToggle() {
  const { t } = useT();
  const [muted, setMutedState] = useState(() => isMuted());
  return (
    <button
      className={styles.muteBtn}
      aria-label={muted ? t('Unmute') : t('Mute')}
      aria-pressed={!muted}
      onClick={() => {
        unlockAudio();
        const next = !muted;
        setMuted(next);
        setMutedState(next);
      }}
    >
      {/* State word beside the icon — a lone 🔊 doesn't say whether it shows
          the current state or the action. */}
      {muted ? <>🔇 {t('Off')}</> : <>🔊 {t('On')}</>}
    </button>
  );
}
