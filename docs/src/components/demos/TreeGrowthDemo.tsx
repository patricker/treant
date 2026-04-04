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

interface TreeNode {
  visits: number;
  avg_reward: number;
  children: Array<{
    mov: string;
    visits: number;
    avg_reward: number;
    child?: TreeNode;
  }>;
}

function TreeGrowthDemoInner() {
  const { useWasm } = require('../mcts/WasmProvider');
  const TreeVisualization = require('../mcts/TreeVisualization').default;
  const StatsPanel = require('../mcts/StatsPanel').default;

  const { wasm, ready, error } = useWasm();
  const gameRef = useRef<any>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const [playing, setPlaying] = useState(false);
  const [interval, setInterval_] = useState(100);
  const [stats, setStats] = useState<SearchStats | null>(null);
  const [tree, setTree] = useState<TreeNode | null>(null);

  const createGame = useCallback(() => {
    if (!wasm) return;
    if (gameRef.current) gameRef.current.free();
    gameRef.current = new wasm.CountingGameWasm(2.0);
    setStats(null);
    setTree(null);
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
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  const tick = useCallback(() => {
    if (!gameRef.current) return;
    gameRef.current.playout_n(10);
    setStats(gameRef.current.get_stats());
    setTree(gameRef.current.get_tree(5));
  }, []);

  // Manage the interval when playing/interval changes
  useEffect(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
    if (playing) {
      intervalRef.current = setInterval(tick, interval);
    }
    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [playing, interval, tick]);

  const handlePlayPause = useCallback(() => {
    setPlaying((p) => !p);
  }, []);

  const handleReset = useCallback(() => {
    setPlaying(false);
    createGame();
  }, [createGame]);

  const handleStep = useCallback(() => {
    tick();
  }, [tick]);

  if (error) {
    return <div className={styles.error}>Failed to load WASM: {error}</div>;
  }

  if (!ready) {
    return <div className={styles.loading}>Loading...</div>;
  }

  return (
    <div className={styles.demo}>
      <div className={styles.section}>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.375rem', alignItems: 'center' }}>
          <button
            className={`button button--sm button--outline button--${playing ? 'warning' : 'success'}`}
            onClick={handlePlayPause}
          >
            {playing ? 'Pause' : 'Play'}
          </button>
          <button
            className="button button--sm button--outline button--primary"
            onClick={handleStep}
            disabled={playing}
          >
            Step (+10)
          </button>
          <button
            className="button button--sm button--outline button--danger"
            onClick={handleReset}
          >
            Reset
          </button>
        </div>
      </div>

      <div className={styles.section}>
        <div className={styles.speedControl}>
          <span className={styles.speedLabel}>Speed</span>
          <input
            className={styles.speedSlider}
            type="range"
            min={10}
            max={500}
            step={10}
            value={510 - interval}
            onChange={(e) => setInterval_(510 - parseInt(e.target.value, 10))}
          />
          <span className={styles.speedValue}>{interval}ms</span>
        </div>
      </div>

      {stats && (
        <div className={styles.section}>
          <StatsPanel
            totalPlayouts={stats.total_playouts}
            totalNodes={stats.total_nodes}
            bestMove={stats.best_move}
            children={stats.children}
          />
        </div>
      )}

      {tree && tree.visits > 0 && (
        <div className={styles.section}>
          <TreeVisualization tree={tree} maxDepth={5} />
        </div>
      )}
    </div>
  );
}

export default function TreeGrowthDemo() {
  return (
    <BrowserOnly fallback={<div className={styles.loading}>Loading...</div>}>
      {() => {
        const { WasmProvider } = require('../mcts/WasmProvider');
        return (
          <WasmProvider>
            <TreeGrowthDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
