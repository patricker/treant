import { useState } from 'react';
import { isMuted, setMuted, unlockAudio } from '../sound';
import styles from '../arcade.module.css';

export default function MuteToggle() {
  const [muted, setMutedState] = useState(() => isMuted());
  return (
    <button
      className={styles.muteBtn}
      aria-label={muted ? 'Unmute' : 'Mute'}
      onClick={() => {
        unlockAudio();
        const next = !muted;
        setMuted(next);
        setMutedState(next);
      }}
    >
      {muted ? '🔇' : '🔊'}
    </button>
  );
}
