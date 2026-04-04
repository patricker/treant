import { useCallback, useEffect, useRef, useState } from 'react';
import BrowserOnly from '@docusaurus/BrowserOnly';
import styles from './demos.module.css';

interface ChildStat {
  mov: string;
  visits: number;
  avg_reward: number;
}

interface SearchStats {
  total_playouts: number;
  total_nodes: number;
  best_move?: string;
  children: ChildStat[];
}

function ChanceNodesDemoInner() {
  const { useWasm } = require('../mcts/WasmProvider');
  const BarChart = require('../mcts/BarChart').default;
  const StatsPanel = require('../mcts/StatsPanel').default;
  const PlaybackControls = require('../mcts/PlaybackControls').default;

  const { wasm, ready, error } = useWasm();
  const gameRef = useRef<any>(null);
  const [stats, setStats] = useState<SearchStats | null>(null);
  const [score, setScore] = useState<bigint>(0n);

  const createGame = useCallback(() => {
    if (!wasm) return;
    if (gameRef.current) gameRef.current.free();
    gameRef.current = new wasm.DiceGameWasm(0n);
    setStats(null);
    setScore(0n);
  }, [wasm]);

  useEffect(() => {
    if (ready) {
      createGame();
    }
    return () => {
      if (gameRef.current) {
        gameRef.current.free();
        gameRef.current = null;
      }
    };
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  const refresh = useCallback(() => {
    if (!gameRef.current) return;
    setStats(gameRef.current.get_stats());
    setScore(gameRef.current.current_score());
  }, []);

  const handleStep = useCallback(() => {
    if (!gameRef.current) return;
    gameRef.current.playout_n(1);
    refresh();
  }, [refresh]);

  const handleRun = useCallback(
    (n: number) => {
      if (!gameRef.current) return;
      gameRef.current.playout_n(n);
      refresh();
    },
    [refresh],
  );

  const handleReset = useCallback(() => {
    createGame();
  }, [createGame]);

  if (error) {
    return <div className={styles.error}>Failed to load WASM: {error}</div>;
  }

  if (!ready) {
    return <div className={styles.loading}>Loading...</div>;
  }

  const barItems =
    stats?.children.map((c) => ({
      label: c.mov,
      value: c.visits,
      secondary: c.avg_reward,
    })) ?? [];

  return (
    <div className={styles.demo}>
      <div className={styles.section}>
        <div className={styles.sectionLabel}>Current score: {score.toString()}</div>
      </div>

      <div className={styles.section}>
        <PlaybackControls
          onStep={handleStep}
          onRun={handleRun}
          onReset={handleReset}
          batchSizes={[10, 100, 1000]}
        />
      </div>

      {stats && (
        <>
          <div className={styles.section}>
            <div className={styles.sectionLabel}>Visit distribution</div>
            <BarChart items={barItems} />
          </div>

          <div className={styles.section}>
            <StatsPanel
              totalPlayouts={stats.total_playouts}
              totalNodes={stats.total_nodes}
              bestMove={stats.best_move}
              children={stats.children}
            />
          </div>
        </>
      )}
    </div>
  );
}

export default function ChanceNodesDemo() {
  return (
    <BrowserOnly fallback={<div className={styles.loading}>Loading...</div>}>
      {() => {
        const { WasmProvider } = require('../mcts/WasmProvider');
        return (
          <WasmProvider>
            <ChanceNodesDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
