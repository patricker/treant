import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { GAMES } from './games';
import styles from './arcade.module.css';

// Hero gradient + a tiny decorative board motif per game.
const HERO_BG: Record<string, string> = {
  'connect-four': 'linear-gradient(135deg,#ff5e7e,#ffb347)',
  'tic-tac-toe': 'linear-gradient(135deg,#f7b733,#fc4a1a)',
  nim: 'linear-gradient(135deg,#2f9e6f,#7bd389)',
  mancala: 'linear-gradient(135deg,#7c5cff,#b388ff)',
  shift: 'linear-gradient(135deg,#ff5e7e,#ff9a6b)',
  '2048': 'linear-gradient(135deg,#f59563,#edc22e)',
};

function HeroArt({ id }: { id: string }) {
  const disc = (bg: string) => (
    <span style={{ width: 18, height: 18, borderRadius: '50%', background: bg, display: 'inline-block' }} />
  );
  const tile = (t: string, bg: string) => (
    <span
      style={{
        width: 26,
        height: 26,
        borderRadius: 5,
        background: bg,
        color: '#776e65',
        fontWeight: 800,
        fontSize: 13,
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      {t}
    </span>
  );
  const wrap = (children: ReactNode) => (
    <div style={{ display: 'flex', gap: 6, alignItems: 'center', justifyContent: 'center' }}>{children}</div>
  );
  switch (id) {
    case 'connect-four':
      return wrap([disc('#ffd23f'), disc('#ff3b5c'), disc('#ffd23f'), disc('#ff3b5c')].map((d, i) => <span key={i}>{d}</span>));
    case 'tic-tac-toe':
      return <div style={{ fontSize: 22, fontWeight: 900, letterSpacing: 4, color: '#fff' }}>X O X</div>;
    case 'nim':
      return wrap([0, 1, 2, 3, 4].map((i) => <span key={i}>{disc('rgba(255,255,255,.85)')}</span>));
    case 'mancala':
      return wrap([3, 1, 4, 2].map((n, i) => (
        <span key={i} style={{ width: 24, height: 24, borderRadius: '50%', background: 'rgba(255,255,255,.85)', color: '#7c5cff', fontWeight: 800, fontSize: 12, display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}>{n}</span>
      )));
    case 'shift':
      return <div style={{ fontSize: 22, fontWeight: 900, letterSpacing: 4, color: '#fff' }}>X · O</div>;
    case '2048':
      return wrap([tile('2', '#eee4da'), tile('4', '#ede0c8'), tile('8', '#f2b179')].map((t, i) => <span key={i}>{t}</span>));
    default:
      return null;
  }
}

export default function Launcher({ onPick }: { onPick: (id: string) => void }) {
  const [featured, setFeatured] = useState(0);
  const pausedRef = useRef(false);

  useEffect(() => {
    const id = setInterval(() => {
      if (!pausedRef.current) setFeatured((f) => (f + 1) % GAMES.length);
    }, 5000);
    return () => clearInterval(id);
  }, []);

  const game = GAMES[featured];

  return (
    <div className={styles.launcher}>
      <h1 className={styles.arcadeTitle}>🌳 Treant Arcade</h1>
      <p className={styles.arcadeSub}>
        Pass-and-play or take on the AI. Crank the knobs and make it weird.
      </p>

      <div className={styles.launcherBody}>
      <div
        className={styles.heroCard}
        style={{ background: HERO_BG[game.id] ?? 'linear-gradient(135deg,#ff5e7e,#ffb347)' }}
        onMouseEnter={() => (pausedRef.current = true)}
        onMouseLeave={() => (pausedRef.current = false)}
      >
        <div className={styles.heroLabel}>✨ Featured</div>
        <div className={styles.heroName}>
          {game.icon} {game.name}
        </div>
        <div className={styles.heroArt}>
          <HeroArt id={game.id} />
        </div>
        <div className={styles.heroBlurb}>{game.blurb}</div>
        <button className={styles.heroPlay} onClick={() => onPick(game.id)}>
          ▶ Play
        </button>
        <div className={styles.heroDots}>
          {GAMES.map((g, i) => (
            <button
              key={g.id}
              className={`${styles.heroDot} ${i === featured ? styles.heroDotOn : ''}`}
              onClick={() => setFeatured(i)}
              aria-label={`Feature ${g.name}`}
            />
          ))}
        </div>
      </div>

      <div className={styles.launcherGames}>
        <div className={styles.moreLabel}>All games</div>
        <div className={styles.gameStrip}>
          {GAMES.map((g) => (
            <button key={g.id} className={styles.stripTile} onClick={() => onPick(g.id)}>
              <span className={styles.stripIcon}>{g.icon}</span>
              <span className={styles.stripName}>{g.name}</span>
            </button>
          ))}
        </div>
      </div>
      </div>
    </div>
  );
}
