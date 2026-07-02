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
  const { useWasm } = require('../treant/WasmProvider');
  const BarChart = require('../treant/BarChart').default;
  const ParameterControls = require('../treant/ParameterControls').default;
  const PlaybackControls = require('../treant/PlaybackControls').default;
  const SideBySide = require('../treant/SideBySide').default;
  const StatsPanel = require('../treant/StatsPanel').default;

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
        <div className={styles.explainer}>
          <p>
            <strong>The setup.</strong> 3 moves over 3 turns: <code>A</code> (+10),
            {' '}<code>B</code> (+5), <code>C</code> (+1). Optimal play is
            {' '}<code>A,A,A = 30</code>. PUCT has been given{' '}
            <strong>deliberately misleading priors</strong>{' '}
            <code>(A=0.1, B=0.2, C=0.7)</code> &mdash; a "policy network"
            that thinks <code>C</code> is best when really <code>A</code> is.
          </p>
          <details>
            <summary>Why does UCT pick A(+10) but PUCT pick B(+5)?</summary>
            <p>
              <strong>UCT</strong> selects with{' '}
              <code>Q + c·√(ln N / n)</code> &mdash; exploration is a pure
              function of visit counts, no priors involved. Every move gets
              explored fairly, so UCT discovers the true ordering and picks{' '}
              <code>A</code>.
            </p>
            <p>
              <strong>PUCT</strong> selects with{' '}
              <code>Q + c·P·√N / (1+n)</code> &mdash; the prior{' '}
              <em>multiplies</em> the exploration bonus, which warps the search:
            </p>
            <ul>
              <li>
                <strong><code>A</code> is starved.</strong> P=0.1 makes A's
                exploration bonus 7× smaller than C's. Even though every visit
                returns a huge reward, A barely gets sampled, so its high Q
                never accumulates enough visit count to win.
              </li>
              <li>
                <strong><code>C</code> is over-explored, then collapses.</strong>{' '}
                P=0.7 pulls early visits to C, but C's subtree only yields ~3
                (1+1+1). As <code>n<sub>C</sub></code> grows, the exploration
                term shrinks and the low Q starts to dominate.
              </li>
              <li>
                <strong><code>B</code> wins by default.</strong> P=0.2 gives B
                enough exploration to find Q ≈ 7-15 (genuinely better than C),
                and its prior is 2× A's, so B ends up the most-visited child.
              </li>
            </ul>
            <p>
              <strong>The lesson:</strong> PUCT is only as good as its priors.
              Good priors → much faster convergence than UCT (the AlphaZero
              superpower). Bad priors → biased search that under-explores
              actually-good moves. Crank playouts to 10k+ and PUCT eventually
              finds <code>A</code> &mdash; the bad prior slows convergence
              rather than preventing it. <code>best_move</code> reports the
              most-visited child, which is why the visit-count race is what
              matters here.
            </p>
          </details>
        </div>
      </div>

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
        const { WasmProvider } = require('../treant/WasmProvider');
        return (
          <WasmProvider>
            <UCTvsPUCTDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
