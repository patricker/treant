import { useState } from 'react';
import { gameById } from './games';
import type { Difficulty, GameParams, Mode } from './gameTypes';
import Launcher from './Launcher';
import GameSetup from './GameSetup';
import GamePlay from './GamePlay';
import MuteToggle from './controls/MuteToggle';
import styles from './arcade.module.css';

type Screen =
  | { name: 'launcher' }
  | { name: 'setup'; gameId: string }
  | { name: 'playing'; gameId: string; params: GameParams; mode: Mode; difficulty: Difficulty };

export default function ArcadeShell({ wasm }: { wasm: any }) {
  const [screen, setScreen] = useState<Screen>({ name: 'launcher' });

  let content: JSX.Element;
  if (screen.name === 'launcher') {
    content = <Launcher onPick={(gameId) => setScreen({ name: 'setup', gameId })} />;
  } else {
    const def = gameById(screen.gameId)!;
    if (screen.name === 'setup') {
      content = (
        <GameSetup
          def={def}
          onBack={() => setScreen({ name: 'launcher' })}
          onStart={({ params, mode, difficulty }) =>
            setScreen({ name: 'playing', gameId: screen.gameId, params, mode, difficulty })
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
          onQuit={() => setScreen({ name: 'launcher' })}
          onChangeSetup={() => setScreen({ name: 'setup', gameId: screen.gameId })}
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
