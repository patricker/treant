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

function StepThroughDemoInner() {
  const { useWasm } = require('../treant/WasmProvider');
  const TreeVisualization = require('../treant/TreeVisualization').default;
  const StatsPanel = require('../treant/StatsPanel').default;
  const ParameterControls = require('../treant/ParameterControls').default;
  const PlaybackControls = require('../treant/PlaybackControls').default;

  const { wasm, ready, error } = useWasm();
  const gameRef = useRef<any>(null);
  const [c, setC] = useState(2.0);
  const [stats, setStats] = useState<SearchStats | null>(null);
  const [tree, setTree] = useState<TreeNode | null>(null);

  const createGame = useCallback(
    (exploration: number) => {
      if (!wasm) return;
      if (gameRef.current) {
        gameRef.current.free();
      }
      gameRef.current = new wasm.CountingGameWasm(exploration);
      setStats(null);
      setTree(null);
    },
    [wasm],
  );

  useEffect(() => {
    if (ready) {
      createGame(c);
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
    const s = gameRef.current.get_stats();
    const t = gameRef.current.get_tree(5);
    setStats(s);
    setTree(t);
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
    createGame(c);
  }, [createGame, c]);

  const handleParamChange = useCallback(
    (_key: string, value: number) => {
      setC(value);
      createGame(value);
    },
    [createGame],
  );

  if (error) {
    return <div className={styles.error}>Failed to load WASM: {error}</div>;
  }

  if (!ready) {
    return <div className={styles.loading}>Loading...</div>;
  }

  return (
    <div className={styles.demo}>
      <div className={styles.section}>
        <ParameterControls
          params={{
            c: { label: 'C (exploration)', value: c, min: 0.1, max: 5.0, step: 0.1 },
          }}
          onChange={handleParamChange}
        />
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

export default function StepThroughDemo() {
  return (
    <BrowserOnly fallback={<div className={styles.loading}>Loading...</div>}>
      {() => {
        const { WasmProvider } = require('../treant/WasmProvider');
        return (
          <WasmProvider>
            <StepThroughDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
