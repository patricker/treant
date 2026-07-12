import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import type { GameDefinition } from './gameTypes';
import { GAMES } from './games';
import { GameIcon } from './icons';
import { useT } from './i18n';
import { recentGames } from './recentGames';
import styles from './arcade.module.css';

// Game groupings for the launcher. Every game id must appear in exactly one
// category — there is no "More" fallback bucket, and scripts/check-categories.mjs
// fails the build if any id is left uncategorized. Variant children (games that
// declare `variantOf`) are the exception: they are HIDDEN from the grid and
// reached via their parent's family expansion, so they do NOT appear here.
const CATEGORIES: { name: string; ids: string[] }[] = [
  { name: 'Family classics', ids: ['connect-four', 'tic-tac-toe', 'draughts', 'reversi', 'salvo', 'vanguard', 'dots-and-boxes', 'mancala', 'oware', 'nim', 'quadline', 'world-threes', 'slimetrail'] },
  { name: 'Connect & line', ids: ['hex', 'y', 'gale', 'gomoku', 'pinch-five', 'connect-six', 'square-up', 'order-chaos', 'treblecross'] },
  { name: 'Move & capture', ids: ['frontline', 'pawn-duel', 'clobber', 'konane', 'nine-morris', 'amazons', 'fox-hounds', 'bagh-chal', 'len-choa', 'surakarta', 'first-capture', 'trails', 'shift'] },
  { name: 'Dice & solo', ids: ['pig', 'climb', '2048'] },
  { name: 'Brain-teasers', ids: ['bulls-cows', 'no-tac-toe', 'trap-three', 'toads-frogs', 'chomp', 'wythoff', 'subtract-square', 'euclid', 'mu-torere', 'domineering', 'nogo', 'col', 'sim'] },
];

// Variant children grouped by parent id (declared via `variantOf` on the child
// GameDefinition). A parent with children renders a "+N" family chip; the
// children live only inside that expansion, never as flat grid tiles.
const CHILDREN: Record<string, GameDefinition[]> = {};
for (const g of GAMES) {
  if (!g.variantOf) continue;
  (CHILDREN[g.variantOf] ??= []).push(g);
}

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

// A single game tile (icon + name). Shared by search/recent/category shelves
// and by family parents + children so every tile reads identically.
function Tile({
  g,
  onPick,
  t,
}: {
  g: GameDefinition;
  onPick: (id: string) => void;
  t: (key: string) => string;
}) {
  return (
    <button className={styles.stripTile} onClick={() => onPick(g.id)}>
      <span className={styles.stripIcon}>
        <GameIcon id={g.id} size={32} />
      </span>
      <span className={styles.stripName}>{t(g.name)}</span>
    </button>
  );
}

// The expansion renderer seam, dispatched ON CHILD COUNT: small families expand
// inline (a row of child tiles directly below the parent); large families will
// get a bottom sheet once one exists.
function renderFamily(
  parent: GameDefinition,
  kids: GameDefinition[],
  onPick: (id: string) => void,
  t: (key: string, params?: Record<string, string | number>) => string,
) {
  // The draughts family (5 children) exercises this inline fallback and reads
  // cleanly — Playwright-verified at 5 children on 390px (2·2·1 wrap, no kid
  // overflow, no horizontal page scroll). A bottom sheet remains an option if a
  // future, larger family (≥4 was the original trigger, up to 9 tiles) reads
  // poorly inline; until then the inline row is the shipped behaviour.
  return (
    <div className={styles.familyKids}>
      <div className={styles.familyCaption}>
        {t('More ways to play {name}', { name: t(parent.name) })}
      </div>
      <div className={styles.familyKidStrip}>
        {kids.map((k) => (
          <Tile key={k.id} g={k} onPick={onPick} t={t} />
        ))}
      </div>
    </div>
  );
}

// A parent tile plus a "+N" chip that expands its variant family. Tapping the
// tile body plays the parent (one tap, unchanged); tapping the chip toggles the
// expansion. The chip carries aria-expanded and a descriptive accessible name.
function FamilyTile({
  parent,
  kids,
  expanded,
  onToggle,
  onPick,
  t,
  tn,
}: {
  parent: GameDefinition;
  kids: GameDefinition[];
  expanded: boolean;
  onToggle: () => void;
  onPick: (id: string) => void;
  t: (key: string, params?: Record<string, string | number>) => string;
  tn: (
    n: number,
    singular: string,
    plural: string,
    params?: Record<string, string | number>,
  ) => string;
}) {
  const label = tn(
    kids.length,
    'Show 1 more way to play {name}',
    'Show {n} more ways to play {name}',
    { name: t(parent.name) },
  );
  return (
    <div className={expanded ? styles.familyOpen : styles.family}>
      <div className={styles.familyHead}>
        <Tile g={parent} onPick={onPick} t={t} />
        <button
          type="button"
          className={styles.familyChip}
          aria-expanded={expanded}
          aria-label={label}
          title={label}
          onClick={onToggle}
        >
          {expanded ? '×' : `+${kids.length} ▾`}
        </button>
      </div>
      {expanded && renderFamily(parent, kids, onPick, t)}
    </div>
  );
}

export default function Launcher({ onPick }: { onPick: (id: string) => void }) {
  const { t, tn } = useT();
  const [featured, setFeatured] = useState(0);
  const [recent, setRecent] = useState<string[]>([]);
  // Which variant family is expanded (only one at a time — simplest state).
  const [openFamily, setOpenFamily] = useState<string | null>(null);
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

  // Escape collapses an expanded family (second chip tap also collapses).
  useEffect(() => {
    if (!openFamily) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpenFamily(null);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [openFamily]);

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
              <Tile key={g.id} g={g} onPick={onPick} t={t} />
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
                <Tile key={g.id} g={g} onPick={onPick} t={t} />
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
                {games.map((g) => {
                  const kids = CHILDREN[g.id];
                  if (!kids || kids.length === 0) {
                    return <Tile key={g.id} g={g} onPick={onPick} t={t} />;
                  }
                  return (
                    <FamilyTile
                      key={g.id}
                      parent={g}
                      kids={kids}
                      expanded={openFamily === g.id}
                      onToggle={() =>
                        setOpenFamily((cur) => (cur === g.id ? null : g.id))
                      }
                      onPick={onPick}
                      t={t}
                      tn={tn}
                    />
                  );
                })}
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
