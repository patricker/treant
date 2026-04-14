import { useCallback, useEffect, useRef, useState } from 'react';
import BrowserOnly from '@docusaurus/BrowserOnly';
import sharedStyles from './demos.module.css';
import styles from './Game2048Demo.module.css';

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

const DIRECTIONS = ['Up', 'Down', 'Left', 'Right'] as const;
type Direction = (typeof DIRECTIONS)[number];

const DIRECTION_ARROWS: Record<Direction, string> = {
  Up: '\u2191',
  Down: '\u2193',
  Left: '\u2190',
  Right: '\u2192',
};

const TILE_DATA_VALUES = [
  0, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384,
  32768, 65536,
];

function tileDataValue(v: number): string {
  if (TILE_DATA_VALUES.includes(v)) return String(v);
  if (v > 2048) return '4096';
  return String(v);
}

const PLAYOUT_OPTIONS = [
  { label: '50 (fast, weak)', value: 50 },
  { label: '200 (balanced)', value: 200 },
  { label: '500 (strong)', value: 500 },
  { label: '2000 (strongest)', value: 2000 },
];

const SPEED_OPTIONS = [
  { label: 'Instant', value: 0 },
  { label: 'Fast (100ms)', value: 100 },
  { label: 'Normal (300ms)', value: 300 },
  { label: 'Slow (700ms)', value: 700 },
];

function Game2048DemoInner() {
  const { useWasm } = require('../treant/WasmProvider');
  const { wasm, ready, error } = useWasm();
  const gameRef = useRef<any>(null);

  const [board, setBoard] = useState<number[]>(new Array(16).fill(0));
  const [score, setScore] = useState(0);
  const [maxTile, setMaxTile] = useState(0);
  const [terminal, setTerminal] = useState(false);
  const [stats, setStats] = useState<SearchStats | null>(null);
  const [suggestion, setSuggestion] = useState<string | undefined>(undefined);
  const [autoPlay, setAutoPlay] = useState(false);
  const autoPlayRef = useRef(false);
  const [playouts, setPlayouts] = useState(200);
  const playoutsRef = useRef(200);
  const [speed, setSpeed] = useState(300);
  const speedRef = useRef(300);
  const [moveCount, setMoveCount] = useState(0);

  // Keep refs in sync for use inside intervals
  useEffect(() => { playoutsRef.current = playouts; }, [playouts]);
  useEffect(() => { speedRef.current = speed; }, [speed]);

  const syncState = useCallback(() => {
    if (!gameRef.current) return;
    const b = gameRef.current.get_board();
    setBoard(Array.from(b));
    setScore(gameRef.current.score());
    setMaxTile(gameRef.current.max_tile());
    setTerminal(gameRef.current.is_terminal());
  }, []);

  const runAnalysis = useCallback(() => {
    if (!gameRef.current || gameRef.current.is_terminal()) {
      setStats(null);
      setSuggestion(undefined);
      return;
    }
    gameRef.current.playout_n(playoutsRef.current);
    const s: SearchStats = gameRef.current.get_stats();
    setStats(s);
    setSuggestion(s.best_move ?? undefined);
  }, []);

  const initGame = useCallback(() => {
    if (!wasm) return;
    if (gameRef.current) {
      gameRef.current.free();
    }
    gameRef.current = new wasm.Game2048Wasm();
    setMoveCount(0);
    syncState();
    runAnalysis();
  }, [wasm, syncState, runAnalysis]);

  useEffect(() => {
    if (ready) {
      initGame();
    }
    return () => {
      if (gameRef.current) {
        gameRef.current.free();
        gameRef.current = null;
      }
    };
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleMove = useCallback(
    (dir: Direction) => {
      if (!gameRef.current || terminal) return;
      const moved = gameRef.current.apply_move(dir);
      if (!moved) return;
      setMoveCount((c) => c + 1);
      syncState();
      runAnalysis();
    },
    [terminal, syncState, runAnalysis],
  );

  const handleMCTSMove = useCallback(() => {
    if (!gameRef.current || terminal || !suggestion) return;
    gameRef.current.apply_move(suggestion);
    setMoveCount((c) => c + 1);
    syncState();
    runAnalysis();
  }, [terminal, suggestion, syncState, runAnalysis]);

  // Auto-play loop
  useEffect(() => {
    autoPlayRef.current = autoPlay;
    if (!autoPlay) return;

    const doStep = () => {
      if (!autoPlayRef.current || !gameRef.current) return;
      if (gameRef.current.is_terminal()) {
        setAutoPlay(false);
        setTerminal(true);
        return;
      }
      gameRef.current.playout_n(playoutsRef.current);
      const s: SearchStats = gameRef.current.get_stats();
      setStats(s);
      const best = s.best_move;
      setSuggestion(best ?? undefined);
      if (best) {
        gameRef.current.apply_move(best);
        setMoveCount((c) => c + 1);
        syncState();
      }
    };

    if (speedRef.current === 0) {
      // Instant mode: use requestAnimationFrame to run as fast as possible
      // while still allowing the UI to update between batches
      let running = true;
      const runBatch = () => {
        if (!running || !autoPlayRef.current) return;
        // Do multiple steps per frame for speed
        for (let i = 0; i < 5; i++) {
          if (!autoPlayRef.current || !gameRef.current || gameRef.current.is_terminal()) {
            if (gameRef.current?.is_terminal()) {
              setAutoPlay(false);
              setTerminal(true);
              syncState();
            }
            running = false;
            return;
          }
          gameRef.current.playout_n(playoutsRef.current);
          const s: SearchStats = gameRef.current.get_stats();
          const best = s.best_move;
          if (best) {
            gameRef.current.apply_move(best);
            setMoveCount((c) => c + 1);
          } else {
            running = false;
            return;
          }
        }
        // Update UI after the batch
        syncState();
        if (gameRef.current) {
          gameRef.current.playout_n(playoutsRef.current);
          const s2: SearchStats = gameRef.current.get_stats();
          setStats(s2);
          setSuggestion(s2.best_move ?? undefined);
        }
        requestAnimationFrame(runBatch);
      };
      requestAnimationFrame(runBatch);
      return () => { running = false; };
    } else {
      // Timed mode: step at the chosen interval
      const interval = setInterval(doStep, speedRef.current);
      return () => clearInterval(interval);
    }
  }, [autoPlay, syncState]);

  // Keyboard support
  useEffect(() => {
    const keyMap: Record<string, Direction> = {
      ArrowUp: 'Up',
      ArrowDown: 'Down',
      ArrowLeft: 'Left',
      ArrowRight: 'Right',
      w: 'Up',
      W: 'Up',
      s: 'Down',
      S: 'Down',
      a: 'Left',
      A: 'Left',
      d: 'Right',
      D: 'Right',
    };

    function onKeyDown(e: KeyboardEvent) {
      const dir = keyMap[e.key];
      if (dir) {
        e.preventDefault();
        handleMove(dir);
      }
    }

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [handleMove]);

  if (error) {
    return (
      <div className={sharedStyles.error}>Failed to load WASM: {error}</div>
    );
  }

  if (!ready) {
    return <div className={sharedStyles.loading}>Loading...</div>;
  }

  // Build 4x4 grid from flat array
  const grid: number[][] = [];
  for (let r = 0; r < 4; r++) {
    grid.push(board.slice(r * 4, r * 4 + 4));
  }

  return (
    <div className={sharedStyles.demo}>
      {/* Score row */}
      <div className={styles.boardContainer}>
        <div className={styles.scoreRow}>
          <div className={styles.scoreBadge}>
            <span>Score</span>
            <span>{score.toLocaleString()}</span>
          </div>
          <div className={styles.scoreBadge}>
            <span>Max Tile</span>
            <span>{maxTile}</span>
          </div>
          <div className={styles.scoreBadge}>
            <span>Moves</span>
            <span>{moveCount}</span>
          </div>
        </div>

        {/* Board */}
        <div className={styles.board}>
          {grid.flat().map((val, i) => (
            <div
              key={i}
              className={styles.tile}
              data-value={tileDataValue(val)}
            >
              {val > 0 ? val : ''}
            </div>
          ))}
          {terminal && (
            <div className={styles.gameOverOverlay}>
              <p>Game Over!</p>
              <p style={{ fontSize: '0.875rem', fontWeight: 400 }}>
                Score: {score.toLocaleString()} | Max: {maxTile} | Moves: {moveCount}
              </p>
              <button
                className="button button--sm button--primary"
                onClick={() => { setAutoPlay(false); initGame(); }}
              >
                New Game
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Settings row */}
      <div className={styles.settingsRow}>
        <label className={styles.settingLabel}>
          <span>Playouts</span>
          <select
            value={playouts}
            onChange={(e) => setPlayouts(Number(e.target.value))}
            className={styles.select}
            disabled={autoPlay}
          >
            {PLAYOUT_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
        </label>
        <label className={styles.settingLabel}>
          <span>Auto Speed</span>
          <select
            value={speed}
            onChange={(e) => setSpeed(Number(e.target.value))}
            className={styles.select}
            disabled={autoPlay}
          >
            {SPEED_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
        </label>
      </div>

      {/* Arrow controls */}
      <div className={styles.controls}>
        <div className={styles.controlRow}>
          <div className={styles.arrowSpacer} />
          <button
            className={styles.arrowBtn}
            onClick={() => handleMove('Up')}
            disabled={terminal || autoPlay}
            title="Up (W / Arrow Up)"
          >
            {DIRECTION_ARROWS.Up}
          </button>
          <div className={styles.arrowSpacer} />
        </div>
        <div className={styles.controlRow}>
          <button
            className={styles.arrowBtn}
            onClick={() => handleMove('Left')}
            disabled={terminal || autoPlay}
            title="Left (A / Arrow Left)"
          >
            {DIRECTION_ARROWS.Left}
          </button>
          <button
            className={styles.arrowBtn}
            onClick={() => handleMove('Down')}
            disabled={terminal || autoPlay}
            title="Down (S / Arrow Down)"
          >
            {DIRECTION_ARROWS.Down}
          </button>
          <button
            className={styles.arrowBtn}
            onClick={() => handleMove('Right')}
            disabled={terminal || autoPlay}
            title="Right (D / Arrow Right)"
          >
            {DIRECTION_ARROWS.Right}
          </button>
        </div>
        <div className={styles.keyHint}>Arrow keys or WASD</div>
        <div className={styles.actionBtns}>
          <button
            className="button button--sm button--outline button--primary"
            onClick={handleMCTSMove}
            disabled={terminal || !suggestion || autoPlay}
          >
            MCTS Move
          </button>
          <button
            className={`button button--sm button--outline ${autoPlay ? 'button--warning' : 'button--success'}`}
            onClick={() => setAutoPlay(!autoPlay)}
            disabled={terminal}
          >
            {autoPlay ? 'Stop' : 'Auto Play'}
          </button>
          <button
            className="button button--sm button--outline button--danger"
            onClick={() => { setAutoPlay(false); initGame(); }}
          >
            New Game
          </button>
        </div>
      </div>

      {/* MCTS suggestion */}
      {suggestion && !terminal && (
        <div className={styles.suggestion}>
          MCTS suggests: {DIRECTION_ARROWS[suggestion as Direction]}{' '}
          {suggestion}
        </div>
      )}

      {/* MCTS analysis table */}
      {stats && stats.children.length > 0 && (
        <div className={sharedStyles.section}>
          <div className={sharedStyles.sectionLabel}>
            MCTS analysis ({stats.total_playouts.toLocaleString()} playouts,{' '}
            {stats.total_nodes.toLocaleString()} nodes)
          </div>
          <table className={styles.analysisTable}>
            <thead>
              <tr>
                <th>Direction</th>
                <th>Visits</th>
                <th>Avg reward</th>
              </tr>
            </thead>
            <tbody>
              {stats.children
                .slice()
                .sort((a, b) => b.visits - a.visits)
                .map((c) => (
                  <tr
                    key={c.mov}
                    className={
                      c.mov === stats.best_move ? styles.bestRow : undefined
                    }
                  >
                    <td>
                      {DIRECTION_ARROWS[c.mov as Direction] ?? ''} {c.mov}
                    </td>
                    <td>{c.visits.toLocaleString()}</td>
                    <td>{c.avg_reward.toFixed(2)}</td>
                  </tr>
                ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export default function Game2048Demo() {
  return (
    <BrowserOnly
      fallback={<div className={sharedStyles.loading}>Loading...</div>}
    >
      {() => {
        const { WasmProvider } = require('../treant/WasmProvider');
        return (
          <WasmProvider>
            <Game2048DemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
