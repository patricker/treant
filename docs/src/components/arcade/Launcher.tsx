import { GAMES } from './games';
import styles from './arcade.module.css';

export default function Launcher({ onPick }: { onPick: (id: string) => void }) {
  return (
    <div className={styles.launcher}>
      <h1 className={styles.arcadeTitle}>🌳 Treant Arcade</h1>
      <p className={styles.arcadeSub}>
        Pass-and-play or take on the AI. Crank the knobs and make it weird.
      </p>
      <div className={styles.launcherGrid}>
        {GAMES.map((g) => (
          <button key={g.id} className={styles.gameTile} onClick={() => onPick(g.id)}>
            <span className={styles.tileIcon}>{g.icon}</span>
            <span className={styles.tileName}>{g.name}</span>
            <span className={styles.tileBlurb}>{g.blurb}</span>
          </button>
        ))}
        <div className={`${styles.gameTile} ${styles.tileSoon}`}>
          <span className={styles.tileIcon}>🚧</span>
          <span className={styles.tileName}>More soon</span>
        </div>
      </div>
    </div>
  );
}
