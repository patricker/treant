import { useState } from 'react';
import { gameById } from './games';
import type { Difficulty, GameParams, Mode } from './gameTypes';
import Launcher from './Launcher';
import GameSetup from './GameSetup';
import GamePlay from './GamePlay';
import styles from './arcade.module.css';

type Screen =
  | { name: 'launcher' }
  | { name: 'setup'; gameId: string }
  | { name: 'playing'; gameId: string; params: GameParams; mode: Mode; difficulty: Difficulty };

export default function ArcadeShell({ wasm }: { wasm: any }) {
  const [screen, setScreen] = useState<Screen>({ name: 'launcher' });

  if (screen.name === 'launcher') {
    return (
      <div className={styles.arcade}>
        <Launcher onPick={(gameId) => setScreen({ name: 'setup', gameId })} />
      </div>
    );
  }

  const def = gameById(screen.gameId)!;

  if (screen.name === 'setup') {
    return (
      <div className={styles.arcade}>
        <GameSetup
          def={def}
          onBack={() => setScreen({ name: 'launcher' })}
          onStart={({ params, mode, difficulty }) =>
            setScreen({ name: 'playing', gameId: screen.gameId, params, mode, difficulty })
          }
        />
      </div>
    );
  }

  // playing — key forces a fresh session when params/mode/difficulty change
  return (
    <div className={styles.arcade}>
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
    </div>
  );
}
