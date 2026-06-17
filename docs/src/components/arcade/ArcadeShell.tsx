import type { JSX } from 'react';
import { useEffect } from 'react';
import { useHistory, useLocation } from '@docusaurus/router';
import { gameById } from './games';
import type { Difficulty, GameParams, Mode, PlayerKind } from './gameTypes';
import { decodeSeats, encodeSeats, seatsForMode } from './gameTypes';
import Launcher from './Launcher';
import GameSetup from './GameSetup';
import GamePlay from './GamePlay';
import MuteToggle from './controls/MuteToggle';
import styles from './arcade.module.css';

type Screen =
  | { name: 'launcher' }
  | { name: 'setup'; gameId: string; initialSeats?: PlayerKind[] }
  | { name: 'playing'; gameId: string; params: GameParams; seats: PlayerKind[] };

const MODES: Mode[] = ['pvp', 'pvai', 'aivai', 'solo'];
const DIFFS: Difficulty[] = ['easy', 'medium', 'hard'];

// The arcade screen lives entirely in the URL search string, driven through
// Docusaurus's own router so the browser Back/Forward buttons move between
// arcade screens (launcher → setup → play) instead of jumping out to the docs
// home. Every game — and its exact per-seat line-up — is deep-linkable.

function screenToSearch(screen: Screen): string {
  if (screen.name === 'launcher') return '';
  const p = new URLSearchParams();
  p.set('game', screen.gameId);
  if (screen.name === 'setup') {
    if (screen.initialSeats) p.set('s', encodeSeats(screen.initialSeats));
  } else {
    p.set('play', '1');
    p.set('p', JSON.stringify(screen.params));
    p.set('s', encodeSeats(screen.seats));
  }
  return '?' + p.toString();
}

function screenFromSearch(search: string): Screen {
  const sp = new URLSearchParams(search);
  const gameId = sp.get('game');
  if (!gameId || !gameById(gameId)) return { name: 'launcher' };
  const def = gameById(gameId)!;
  const sCode = sp.get('s');
  // Back-compat with old ?mode=&difficulty= links.
  const rawMode = sp.get('mode');
  const rawDiff = sp.get('difficulty');
  const mode = rawMode && MODES.includes(rawMode as Mode) ? (rawMode as Mode) : undefined;
  const diff = rawDiff && DIFFS.includes(rawDiff as Difficulty) ? (rawDiff as Difficulty) : 'medium';

  if (sp.get('play') === '1') {
    let params: GameParams = def.defaultParams;
    try {
      const raw = sp.get('p');
      if (raw) params = { ...def.defaultParams, ...JSON.parse(raw) };
    } catch {
      /* fall back to defaults */
    }
    const np = params.numPlayers ?? 2;
    const seats = sCode
      ? decodeSeats(sCode, np)
      : seatsForMode(mode ?? (def.solo ? 'solo' : 'pvai'), np, diff);
    return { name: 'playing', gameId, params, seats };
  }
  const np = def.defaultParams.numPlayers ?? 2;
  const initialSeats = sCode ? decodeSeats(sCode, np) : mode ? seatsForMode(mode, np, diff) : undefined;
  return { name: 'setup', gameId, initialSeats };
}

export default function ArcadeShell({ wasm }: { wasm: any }) {
  const history = useHistory();
  const location = useLocation();
  // The GA4 (gtag) plugin's pageview hook runs on every client route change but
  // its script isn't injected in dev, so calls would throw "gtag is not a
  // function" on each in-arcade navigation. Production loads gtag, so this only
  // guards the dev/preview experience (and never overwrites a real gtag).
  if (typeof window !== 'undefined' && typeof (window as any).gtag !== 'function') {
    (window as any).gtag = () => {};
  }

  // The full Docusaurus footer (link columns) eats a screenful on mobile under
  // an interactive game. Tag <body> while the arcade is mounted so global CSS can
  // collapse the footer to just the copyright line on this route.
  useEffect(() => {
    document.body.classList.add('arcade-route');
    return () => document.body.classList.remove('arcade-route');
  }, []);

  const screen = screenFromSearch(location.search);
  const go = (next: Screen) => history.push('/arcade' + screenToSearch(next));

  let content: JSX.Element;
  if (screen.name === 'launcher') {
    content = <Launcher onPick={(gameId) => go({ name: 'setup', gameId })} />;
  } else {
    const def = gameById(screen.gameId)!;
    if (screen.name === 'setup') {
      content = (
        <GameSetup
          def={def}
          initialSeats={screen.initialSeats}
          onBack={() => go({ name: 'launcher' })}
          onStart={({ params, seats }) => go({ name: 'playing', gameId: screen.gameId, params, seats })}
        />
      );
    } else {
      content = (
        <GamePlay
          key={`${screen.gameId}-${JSON.stringify(screen.params)}-${encodeSeats(screen.seats)}`}
          wasm={wasm}
          def={def}
          params={screen.params}
          seats={screen.seats}
          onQuit={() => go({ name: 'launcher' })}
          onChangeSetup={() => go({ name: 'setup', gameId: screen.gameId })}
        />
      );
    }
  }

  return (
    <div className={styles.arcade}>
      <MuteToggle />
      {content}
    </div>
  );
}
