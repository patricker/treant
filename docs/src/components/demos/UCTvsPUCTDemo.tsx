import { useCallback, useEffect, useRef, useState } from 'react';
import BrowserOnly from '@docusaurus/BrowserOnly';
import styles from './demos.module.css';

interface ChildStat {
  mov: string;
  visits: number;
  avg_reward: number;
  prior?: number;
}

interface SearchStats {
  total_playouts: number;
  total_nodes: number;
  best_move?: string;
  children: ChildStat[];
}

function UCTvsPUCTDemoInner() {
  const { useWasm } = require('../mcts/WasmProvider');
  const BarChart = require('../mcts/BarChart').default;
  const ParameterControls = require('../mcts/ParameterControls').default;
  const PlaybackControls = require('../mcts/PlaybackControls').default;
  const SideBySide = require('../mcts/SideBySide').default;
  const StatsPanel = require('../mcts/StatsPanel').default;

  const { wasm, ready, error } = useWasm();
  const uctRef = useRef<any>(null);
  const puctRef = useRef<any>(null);
  const [uctC, setUctC] = useState(2.0);
  const [puctC, setPuctC] = useState(1.5);
  const [uctStats, setUctStats] = useState<SearchStats | null>(null);
  const [puctStats, setPuctStats] = useState<SearchStats | null>(null);

  const createBoth = useCallback(
    (uc: number, pc: number) => {
      if (!wasm) return;
      if (uctRef.current) uctRef.current.free();
      if (puctRef.current) puctRef.current.free();
      uctRef.current = new wasm.PriorGameUctWasm(uc);
      puctRef.current = new wasm.PriorGamePuctWasm(pc);
      setUctStats(null);
      setPuctStats(null);
    },
    [wasm],
  );

  useEffect(() => {
    if (ready) {
      createBoth(uctC, puctC);
    }
    return () => {
      if (uctRef.current) {
        uctRef.current.free();
        uctRef.current = null;
      }
      if (puctRef.current) {
        puctRef.current.free();
        puctRef.current = null;
      }
    };
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  const refresh = useCallback(() => {
    if (uctRef.current) {
      setUctStats(uctRef.current.get_stats());
    }
    if (puctRef.current) {
      setPuctStats(puctRef.current.get_stats());
    }
  }, []);

  const handleStep = useCallback(() => {
    if (uctRef.current) uctRef.current.playout_n(1);
    if (puctRef.current) puctRef.current.playout_n(1);
    refresh();
  }, [refresh]);

  const handleRun = useCallback(
    (n: number) => {
      if (uctRef.current) uctRef.current.playout_n(n);
      if (puctRef.current) puctRef.current.playout_n(n);
      refresh();
    },
    [refresh],
  );

  const handleReset = useCallback(() => {
    createBoth(uctC, puctC);
  }, [createBoth, uctC, puctC]);

  const handleParamChange = useCallback(
    (key: string, value: number) => {
      if (key === 'uctC') {
        setUctC(value);
        createBoth(value, puctC);
      } else {
        setPuctC(value);
        createBoth(uctC, value);
      }
    },
    [createBoth, uctC, puctC],
  );

  if (error) {
    return <div className={styles.error}>Failed to load WASM: {error}</div>;
  }

  if (!ready) {
    return <div className={styles.loading}>Loading...</div>;
  }

  const toUctBarItems = (s: SearchStats | null) =>
    s?.children.map((c) => ({
      label: c.mov,
      value: c.visits,
      secondary: c.avg_reward,
    })) ?? [];

  const toPuctBarItems = (s: SearchStats | null) =>
    s?.children.map((c) => ({
      label: c.prior != null ? `${c.mov} (p=${c.prior.toFixed(1)})` : c.mov,
      value: c.visits,
      secondary: c.avg_reward,
    })) ?? [];

  const maxVisits = Math.max(
    ...(uctStats?.children.map((c) => c.visits) ?? [1]),
    ...(puctStats?.children.map((c) => c.visits) ?? [1]),
  );

  return (
    <div className={styles.demo}>
      <div className={styles.section}>
        <ParameterControls
          params={{
            uctC: { label: 'UCT C', value: uctC, min: 0.1, max: 5.0, step: 0.1 },
            puctC: { label: 'PUCT C', value: puctC, min: 0.1, max: 5.0, step: 0.1 },
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
          leftLabel="UCT (no priors)"
          rightLabel="PUCT (with priors)"
          left={
            <>
              <BarChart items={toUctBarItems(uctStats)} maxValue={maxVisits} />
              {uctStats && (
                <StatsPanel
                  totalPlayouts={uctStats.total_playouts}
                  totalNodes={uctStats.total_nodes}
                  bestMove={uctStats.best_move}
                  children={uctStats.children}
                />
              )}
            </>
          }
          right={
            <>
              <BarChart items={toPuctBarItems(puctStats)} maxValue={maxVisits} />
              {puctStats && (
                <StatsPanel
                  totalPlayouts={puctStats.total_playouts}
                  totalNodes={puctStats.total_nodes}
                  bestMove={puctStats.best_move}
                  children={puctStats.children}
                />
              )}
            </>
          }
        />
      </div>
    </div>
  );
}

export default function UCTvsPUCTDemo() {
  return (
    <BrowserOnly fallback={<div className={styles.loading}>Loading...</div>}>
      {() => {
        const { WasmProvider } = require('../mcts/WasmProvider');
        return (
          <WasmProvider>
            <UCTvsPUCTDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
