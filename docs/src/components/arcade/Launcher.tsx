import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { GAMES } from './games';
import { GameIcon } from './icons';
import { useT } from './i18n';
import { recentGames } from './recentGames';
import styles from './arcade.module.css';

// Game groupings for the launcher (any game not listed falls under "More").
const CATEGORIES: { name: string; ids: string[] }[] = [
  { name: 'Family classics', ids: ['connect-four', 'tic-tac-toe', 'reversi', 'anti-reversi', 'dots-and-boxes', 'mancala', 'oware', 'nim', 'quadline'] },
  { name: 'Connect & line', ids: ['hex', 'y', 'gomoku', 'pinch-five', 'connect-six', 'square-up', 'order-chaos', 'treblecross'] },
  { name: 'Move & capture', ids: ['frontline', 'clobber', 'konane', 'nine-morris', 'lasker-morris', 'amazons', 'fox-hounds', 'bagh-chal', 'first-capture', 'trails', 'joust', 'shift'] },
  { name: 'Dice & solo', ids: ['pig', 'climb', '2048'] },
  { name: 'Brain-teasers', ids: ['no-tac-toe', 'trap-three', 'chomp', 'wythoff', 'subtract-square', 'euclid', 'mu-torere', 'domineering', 'cram', 'nogo', 'col', 'snort', 'sim'] },
];

// Hero gradient + a tiny decorative board motif per game.
const HERO_BG: Record<string, string> = {
  'connect-four': 'linear-gradient(135deg,#ff5e7e,#ffb347)',
  'tic-tac-toe': 'linear-gradient(135deg,#f7b733,#fc4a1a)',
  nim: 'linear-gradient(135deg,#2f9e6f,#7bd389)',
  mancala: 'linear-gradient(135deg,#7c5cff,#b388ff)',
  shift: 'linear-gradient(135deg,#ff5e7e,#ff9a6b)',
  '2048': 'linear-gradient(135deg,#f59563,#edc22e)',
};

// The Treant mascot (same little guy as the site logo), drawn in currentColor
// so he inherits the accent tint + glow of wherever he stands.
function TreantMascot({ size = 30 }: { size?: number }) {
  return (
    <svg viewBox="0 0 64 64" width={size} height={size} className={styles.mascot} aria-hidden="true">
      <circle cx="32" cy="24" r="20" fill="none" stroke="currentColor" strokeWidth="3.5" />
      <rect x="27" y="42" width="10" height="14" fill="currentColor" />
      <line x1="32" y1="56" x2="20" y2="60" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
      <line x1="32" y1="56" x2="44" y2="60" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
      <circle cx="25" cy="22" r="3" fill="currentColor" />
      <circle cx="39" cy="22" r="3" fill="currentColor" />
    </svg>
  );
}

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
      // Most games have no bespoke motif — show their SVG glyph large rather
      // than an empty box.
      return (
        <span style={{ color: 'rgba(255,255,255,0.92)' }}>
          <GameIcon id={id} size={54} />
        </span>
      );
  }
}

export default function Launcher({ onPick }: { onPick: (id: string) => void }) {
  const { t } = useT();
  const [featured, setFeatured] = useState(0);
  const [recent, setRecent] = useState<string[]>([]);
  // Auto-rotation stops FOR GOOD on the first interaction with the hero
  // (moving targets are hostile once someone is reading), and never runs for
  // users who ask the OS for reduced motion.
  const pausedRef = useRef(false);

  useEffect(() => {
    setRecent(recentGames());
    if (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) {
      pausedRef.current = true;
    }
    const id = setInterval(() => {
      if (!pausedRef.current) setFeatured((f) => (f + 1) % GAMES.length);
    }, 6000);
    return () => clearInterval(id);
  }, []);

  const stopRotation = () => (pausedRef.current = true);
  const step = (d: number) => {
    stopRotation();
    setFeatured((f) => (f + d + GAMES.length) % GAMES.length);
  };

  const game = GAMES[featured];
  const recentDefs = recent
    .map((id) => GAMES.find((g) => g.id === id))
    .filter(Boolean) as typeof GAMES;

  const [query, setQuery] = useState('');
  const q = query.trim().toLowerCase();
  // Match the translated name AND the English name (players may search either).
  const matches = q
    ? GAMES.filter(
        (g) => t(g.name).toLowerCase().includes(q) || g.name.toLowerCase().includes(q),
      )
    : null;

  return (
    <div className={styles.launcher}>
      <h1 className={styles.arcadeTitle}>
        <TreantMascot /> Treant Arcade
      </h1>
      <p className={styles.arcadeSub}>
        {t('Pass-and-play or take on the AI. Crank the knobs and make it weird.')}
      </p>

      <div className={styles.launcherBody}>
      <div
        className={styles.heroCard}
        style={{ background: HERO_BG[game.id] ?? 'linear-gradient(135deg,#ff5e7e,#ffb347)' }}
        onMouseEnter={stopRotation}
        onTouchStart={stopRotation}
      >
        <div className={styles.heroLabel}>{t('✨ Featured')}</div>
        <div className={styles.heroName}>
          <GameIcon id={game.id} size={30} /> {t(game.name)}
        </div>
        <div className={styles.heroArt}>
          <button className={`${styles.heroNav} ${styles.heroNavPrev}`} aria-label={t('Previous game')} onClick={() => step(-1)}>
            ‹
          </button>
          <HeroArt id={game.id} />
          <button className={`${styles.heroNav} ${styles.heroNavNext}`} aria-label={t('Next game')} onClick={() => step(1)}>
            ›
          </button>
        </div>
        <div className={styles.heroBlurb}>{t(game.blurb)}</div>
        <button className={styles.heroPlay} onClick={() => onPick(game.id)}>
          {t('▶ Play')}
        </button>
      </div>

      <input
        type="search"
        className={styles.searchBox}
        placeholder={t('Search games…')}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        aria-label={t('Search games…')}
      />

      {matches ? (
        <div className={styles.launcherGames}>
          <div className={styles.moreLabel}>
            {matches.length ? t('Search results') : t('No games match')}
          </div>
          <div className={styles.gameStrip}>
            {matches.map((g) => (
              <button key={g.id} className={styles.stripTile} onClick={() => onPick(g.id)}>
                <span className={styles.stripIcon}>
                  <GameIcon id={g.id} size={32} />
                </span>
                <span className={styles.stripName}>{t(g.name)}</span>
              </button>
            ))}
          </div>
        </div>
      ) : (
      <div className={styles.launcherGames}>
        {recentDefs.length > 0 && (
          <div>
            <div className={styles.moreLabel}>{t('Recently played')}</div>
            <div className={styles.gameStrip}>
              {recentDefs.map((g) => (
                <button key={g.id} className={styles.stripTile} onClick={() => onPick(g.id)}>
                  <span className={styles.stripIcon}>
                    <GameIcon id={g.id} size={32} />
                  </span>
                  <span className={styles.stripName}>{t(g.name)}</span>
                </button>
              ))}
            </div>
          </div>
        )}
        {CATEGORIES.map((cat) => {
          const games = cat.ids.map((id) => GAMES.find((g) => g.id === id)).filter(Boolean) as typeof GAMES;
          if (games.length === 0) return null;
          return (
            <div key={cat.name}>
              <div className={styles.moreLabel}>{t(cat.name)}</div>
              <div className={styles.gameStrip}>
                {games.map((g) => (
                  <button key={g.id} className={styles.stripTile} onClick={() => onPick(g.id)}>
                    <span className={styles.stripIcon}>
                      <GameIcon id={g.id} size={32} />
                    </span>
                    <span className={styles.stripName}>{t(g.name)}</span>
                  </button>
                ))}
              </div>
            </div>
          );
        })}
      </div>
      )}
      </div>
    </div>
  );
}
