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

function ExplorationDemoInner() {
  const { useWasm } = require('../treant/WasmProvider');
  const BarChart = require('../treant/BarChart').default;
  const ParameterControls = require('../treant/ParameterControls').default;
  const PlaybackControls = require('../treant/PlaybackControls').default;
  const SideBySide = require('../treant/SideBySide').default;
  const StatsPanel = require('../treant/StatsPanel').default;

  const { wasm, ready, error } = useWasm();
  const leftRef = useRef<any>(null);
  const rightRef = useRef<any>(null);
  const [leftC, setLeftC] = useState(0.5);
  const [rightC, setRightC] = useState(3.0);
  const [leftStats, setLeftStats] = useState<SearchStats | null>(null);
  const [rightStats, setRightStats] = useState<SearchStats | null>(null);

  const createBoth = useCallback(
    (lc: number, rc: number) => {
      if (!wasm) return;
      if (leftRef.current) leftRef.current.free();
      if (rightRef.current) rightRef.current.free();
      leftRef.current = new wasm.CountingGameWasm(lc);
      rightRef.current = new wasm.CountingGameWasm(rc);
      setLeftStats(null);
      setRightStats(null);
    },
    [wasm],
  );

  useEffect(() => {
    if (ready) {
      createBoth(leftC, rightC);
    }
    return () => {
      if (leftRef.current) {
        leftRef.current.free();
        leftRef.current = null;
      }
      if (rightRef.current) {
        rightRef.current.free();
        rightRef.current = null;
      }
    };
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  const refresh = useCallback(() => {
    if (leftRef.current) {
      setLeftStats(leftRef.current.get_stats());
    }
    if (rightRef.current) {
      setRightStats(rightRef.current.get_stats());
    }
  }, []);

  const handleStep = useCallback(() => {
    if (leftRef.current) leftRef.current.playout_n(1);
    if (rightRef.current) rightRef.current.playout_n(1);
    refresh();
  }, [refresh]);

  const handleRun = useCallback(
    (n: number) => {
      if (leftRef.current) leftRef.current.playout_n(n);
      if (rightRef.current) rightRef.current.playout_n(n);
      refresh();
    },
    [refresh],
  );

  const handleReset = useCallback(() => {
    createBoth(leftC, rightC);
  }, [createBoth, leftC, rightC]);

  const handleParamChange = useCallback(
    (key: string, value: number) => {
      if (key === 'leftC') {
        setLeftC(value);
        createBoth(value, rightC);
      } else {
        setRightC(value);
        createBoth(leftC, value);
      }
    },
    [createBoth, leftC, rightC],
  );

  if (error) {
    return <div className={styles.error}>Failed to load WASM: {error}</div>;
  }

  if (!ready) {
    return <div className={styles.loading}>Loading...</div>;
  }

  const toBarItems = (s: SearchStats | null) =>
    s?.children.map((c) => ({
      label: c.mov,
      value: c.visits,
      secondary: c.avg_reward,
    })) ?? [];

  const maxVisits = Math.max(
    ...(leftStats?.children.map((c) => c.visits) ?? [1]),
    ...(rightStats?.children.map((c) => c.visits) ?? [1]),
  );

  return (
    <div className={styles.demo}>
      <div className={styles.section}>
        <ParameterControls
          params={{
            leftC: { label: 'Left C (exploit)', value: leftC, min: 0.1, max: 5.0, step: 0.1 },
            rightC: { label: 'Right C (explore)', value: rightC, min: 0.1, max: 5.0, step: 0.1 },
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

      <div className={styles.section}>
        <SideBySide
          leftLabel={`C = ${leftC.toFixed(1)} (exploit)`}
          rightLabel={`C = ${rightC.toFixed(1)} (explore)`}
          left={
            <>
              <BarChart items={toBarItems(leftStats)} maxValue={maxVisits} />
              {leftStats && (
                <StatsPanel
                  totalPlayouts={leftStats.total_playouts}
                  totalNodes={leftStats.total_nodes}
                  bestMove={leftStats.best_move}
                />
              )}
            </>
          }
          right={
            <>
              <BarChart items={toBarItems(rightStats)} maxValue={maxVisits} />
              {rightStats && (
                <StatsPanel
                  totalPlayouts={rightStats.total_playouts}
                  totalNodes={rightStats.total_nodes}
                  bestMove={rightStats.best_move}
                />
              )}
            </>
          }
        />
      </div>
    </div>
  );
}

export default function ExplorationDemo() {
  return (
    <BrowserOnly fallback={<div className={styles.loading}>Loading...</div>}>
      {() => {
        const { WasmProvider } = require('../treant/WasmProvider');
        return (
          <WasmProvider>
            <ExplorationDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
