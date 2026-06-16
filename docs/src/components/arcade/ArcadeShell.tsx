import { useHistory, useLocation } from '@docusaurus/router';
import { gameById } from './games';
import type { Difficulty, GameParams, Mode } from './gameTypes';
import Launcher from './Launcher';
import GameSetup from './GameSetup';
import GamePlay from './GamePlay';
import MuteToggle from './controls/MuteToggle';
import styles from './arcade.module.css';

type Screen =
  | { name: 'launcher' }
  | { name: 'setup'; gameId: string; initialMode?: Mode; initialDifficulty?: Difficulty }
  | { name: 'playing'; gameId: string; params: GameParams; mode: Mode; difficulty: Difficulty };

const MODES: Mode[] = ['pvp', 'pvai', 'aivai', 'solo'];
const DIFFS: Difficulty[] = ['easy', 'medium', 'hard'];

// The arcade screen lives entirely in the URL search string, driven through
// Docusaurus's own router. That keeps the browser Back/Forward buttons moving
// between arcade screens (launcher → setup → play) instead of jumping out to
// the docs home, and makes every game deep-linkable.

function screenToSearch(screen: Screen): string {
  if (screen.name === 'launcher') return '';
  const p = new URLSearchParams();
  p.set('game', screen.gameId);
  if (screen.name === 'setup') {
    if (screen.initialMode) p.set('mode', screen.initialMode);
    if (screen.initialDifficulty) p.set('difficulty', screen.initialDifficulty);
  } else {
    p.set('mode', screen.mode);
    p.set('difficulty', screen.difficulty);
    p.set('play', '1');
    p.set('p', JSON.stringify(screen.params));
  }
  return '?' + p.toString();
}

function screenFromSearch(search: string): Screen {
  const sp = new URLSearchParams(search);
  const gameId = sp.get('game');
  if (!gameId || !gameById(gameId)) return { name: 'launcher' };
  const rawMode = sp.get('mode');
  const rawDiff = sp.get('difficulty');
  const mode = rawMode && MODES.includes(rawMode as Mode) ? (rawMode as Mode) : undefined;
  const difficulty = rawDiff && DIFFS.includes(rawDiff as Difficulty) ? (rawDiff as Difficulty) : undefined;
  if (sp.get('play') === '1') {
    const def = gameById(gameId)!;
    let params: GameParams = def.defaultParams;
    try {
      const raw = sp.get('p');
      if (raw) params = { ...def.defaultParams, ...JSON.parse(raw) };
    } catch {
      /* fall back to defaults */
    }
    return { name: 'playing', gameId, params, mode: mode ?? 'pvai', difficulty: difficulty ?? 'medium' };
  }
  return { name: 'setup', gameId, initialMode: mode, initialDifficulty: difficulty };
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
          initialMode={screen.initialMode}
          initialDifficulty={screen.initialDifficulty}
          onBack={() => go({ name: 'launcher' })}
          onStart={({ params, mode, difficulty }) =>
            go({ name: 'playing', gameId: screen.gameId, params, mode, difficulty })
          }
        />
      );
    } else {
      content = (
        <GamePlay
          key={`${screen.gameId}-${JSON.stringify(screen.params)}-${screen.mode}-${screen.difficulty}`}
          wasm={wasm}
          def={def}
          params={screen.params}
          mode={screen.mode}
          difficulty={screen.difficulty}
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
