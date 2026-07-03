import type { JSX } from 'react';
import { useEffect } from 'react';
import { useHistory, useLocation } from '@docusaurus/router';
import { gameById } from './games';
import type { Difficulty, GameParams, Mode, PlayerKind } from './gameTypes';
import { decodeSeats, encodeSeats, seatsForMode } from './gameTypes';
import { useArcadeAccent } from './arcadeAccent';
import Launcher from './Launcher';
import GameSetup from './GameSetup';
import GamePlay from './GamePlay';
import MuteToggle from './controls/MuteToggle';
import ThemeSwitcher from './controls/ThemeSwitcher';
import LanguagePicker from './controls/LanguagePicker';
import { LocaleProvider } from './i18n';
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
  const [accent, setAccent] = useArcadeAccent();
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

  // Native-app (Capacitor) integration. Everything here is feature-detected off
  // the injected `window.Capacitor` bridge — it's inert in the web build (the
  // bridge is absent) and pulls in no Capacitor imports, so the docs bundle is
  // unchanged. Tags <body> so CSS can hide the Docusaurus chrome, tints the
  // status bar to the arcade backdrop, and wires the Android hardware back
  // button to walk the arcade's history (exiting the app only at the launcher).
  useEffect(() => {
    const cap = (typeof window !== 'undefined' && (window as any).Capacitor) || null;
    if (!cap?.isNativePlatform?.()) return;
    document.body.classList.add('capacitor-app');

    const StatusBar = cap.Plugins?.StatusBar;
    if (StatusBar) {
      // `Style.Dark` = dark backdrop, light (readable) content.
      StatusBar.setStyle?.({ style: 'DARK' });
      StatusBar.setBackgroundColor?.({ color: '#1c1830' });
    }

    let remove: (() => void) | undefined;
    const App = cap.Plugins?.App;
    if (App?.addListener) {
      const handle = App.addListener('backButton', () => {
        // At the launcher there's nowhere left to go back to, so honour the
        // Android convention of backing out to the home screen; otherwise step
        // back through launcher → setup → play history.
        if (window.location.search === '') App.exitApp?.();
        else history.goBack();
      });
      remove = () => {
        // addListener may return a handle or a promise of one.
        Promise.resolve(handle).then((h: any) => h?.remove?.());
      };
    }

    return () => {
      document.body.classList.remove('capacitor-app');
      remove?.();
    };
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
    <>
      <LocaleProvider>
        <div className={`${styles.arcade} ${styles.neon}`} data-accent={accent}>
          <div className={styles.arcadeTopBar}>
            <ThemeSwitcher accent={accent} onChange={setAccent} />
            <LanguagePicker />
            <MuteToggle />
          </div>
          {content}
        </div>
      </LocaleProvider>
    </>
  );
}
