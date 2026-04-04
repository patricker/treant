import { useCallback, useEffect, useRef, useState } from 'react';
import BrowserOnly from '@docusaurus/BrowserOnly';
import styles from './demos.module.css';

interface ChildStat {
  mov: string;
  visits: number;
  avg_reward: number;
  proven?: string;
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
  proven?: string;
  children: Array<{
    mov: string;
    visits: number;
    avg_reward: number;
    child?: TreeNode;
  }>;
}

type Phase = 'human' | 'mcts' | 'gameover';

function NimSolverDemoInner() {
  const { useWasm } = require('../mcts/WasmProvider');
  const TreeVisualization = require('../mcts/TreeVisualization').default;
  const StatsPanel = require('../mcts/StatsPanel').default;
  const ParameterControls = require('../mcts/ParameterControls').default;
  const NimBoard = require('../mcts/GameBoard/NimBoard').default;

  const { wasm, ready, error } = useWasm();
  const gameRef = useRef<any>(null);
  const [startingStones, setStartingStones] = useState(7);
  const [stones, setStones] = useState(7);
  const [currentPlayer, setCurrentPlayer] = useState('P1');
  const [phase, setPhase] = useState<Phase>('human');
  const [stats, setStats] = useState<SearchStats | null>(null);
  const [tree, setTree] = useState<TreeNode | null>(null);
  const [provenValue, setProvenValue] = useState<string>('Unknown');
  const [winner, setWinner] = useState<string | null>(null);

  const initGame = useCallback(
    (numStones: number) => {
      if (!wasm) return;
      if (gameRef.current) {
        gameRef.current.free();
      }
      gameRef.current = new wasm.NimWasm(numStones);
      setStones(numStones);
      setCurrentPlayer('P1');
      setPhase('human');
      setStats(null);
      setTree(null);
      setProvenValue('Unknown');
      setWinner(null);

      // Run initial analysis for human's position
      gameRef.current.playout_n(500);
      const s = gameRef.current.get_stats();
      const t = gameRef.current.get_tree(4);
      setStats(s);
      setTree(t);
      setProvenValue(gameRef.current.root_proven_value());
    },
    [wasm],
  );

  useEffect(() => {
    if (ready) {
      initGame(startingStones);
    }
    return () => {
      if (gameRef.current) {
        gameRef.current.free();
        gameRef.current = null;
      }
    };
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  const syncState = useCallback(() => {
    if (!gameRef.current) return;
    setStones(gameRef.current.current_stones());
    setCurrentPlayer(gameRef.current.current_player());
    setProvenValue(gameRef.current.root_proven_value());
  }, []);

  const handleHumanMove = useCallback(
    (move: 'Take1' | 'Take2') => {
      if (!gameRef.current || phase !== 'human') return;

      gameRef.current.apply_move(move);
      syncState();

      if (gameRef.current.is_terminal()) {
        // Human took the last stone(s) -- human loses in misere Nim
        const s = gameRef.current.get_stats();
        setStats(s);
        setTree(null);
        setPhase('gameover');
        // The player who just moved loses (they took the last stone)
        setWinner(gameRef.current.current_player());
        return;
      }

      // MCTS turn
      setPhase('mcts');

      // Run playouts and pick best move
      gameRef.current.playout_n(500);
      const bestMove = gameRef.current.best_move();
      const s = gameRef.current.get_stats();
      const t = gameRef.current.get_tree(4);
      setStats(s);
      setTree(t);
      setProvenValue(gameRef.current.root_proven_value());

      if (bestMove) {
        // Use setTimeout so the user can briefly see the MCTS analysis
        setTimeout(() => {
          if (!gameRef.current) return;
          gameRef.current.apply_move(bestMove);
          syncState();

          if (gameRef.current.is_terminal()) {
            const finalStats = gameRef.current.get_stats();
            setStats(finalStats);
            setTree(null);
            setPhase('gameover');
            setWinner(gameRef.current.current_player());
            return;
          }

          // Analyze human's new position
          gameRef.current.playout_n(500);
          const newStats = gameRef.current.get_stats();
          const newTree = gameRef.current.get_tree(4);
          setStats(newStats);
          setTree(newTree);
          setProvenValue(gameRef.current.root_proven_value());
          setPhase('human');
        }, 400);
      }
    },
    [phase, syncState],
  );

  const handleParamChange = useCallback(
    (_key: string, value: number) => {
      const rounded = Math.round(value);
      setStartingStones(rounded);
      initGame(rounded);
    },
    [initGame],
  );

  if (error) {
    return <div className={styles.error}>Failed to load WASM: {error}</div>;
  }

  if (!ready) {
    return <div className={styles.loading}>Loading...</div>;
  }

  const provenStatus = provenValue.toLowerCase() as 'win' | 'loss' | 'draw' | 'unknown';

  return (
    <div className={styles.demo}>
      <div className={styles.section}>
        <ParameterControls
          params={{
            stones: {
              label: 'Starting stones',
              value: startingStones,
              min: 3,
              max: 12,
              step: 1,
            },
          }}
          onChange={handleParamChange}
        />
      </div>

      <div className={styles.section}>
        <NimBoard
          stones={stones}
          currentPlayer={currentPlayer}
          onMove={phase === 'human' ? handleHumanMove : undefined}
          disabled={phase !== 'human'}
        />
      </div>

      {phase === 'mcts' && (
        <div className={styles.section}>
          <span className={styles.thinking}>MCTS is thinking...</span>
        </div>
      )}

      {phase === 'gameover' && (
        <div className={styles.gameOver}>
          <p>{winner} wins!</p>
          <button
            className="button button--sm button--outline button--primary"
            onClick={() => initGame(startingStones)}
          >
            New Game
          </button>
        </div>
      )}

      <div className={styles.section}>
        <span className={styles.provenValue} data-status={provenStatus}>
          MCTS proved: {provenValue}
        </span>
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
          <TreeVisualization tree={tree} maxDepth={4} />
        </div>
      )}
    </div>
  );
}

export default function NimSolverDemo() {
  return (
    <BrowserOnly fallback={<div className={styles.loading}>Loading...</div>}>
      {() => {
        const { WasmProvider } = require('../mcts/WasmProvider');
        return (
          <WasmProvider>
            <NimSolverDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
