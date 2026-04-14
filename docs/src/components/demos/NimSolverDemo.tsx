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

interface MoveEntry {
  turn: number;
  player: string;
  move: string;
  stonesAfter: number;
  proven: string;
  bestMove?: string;
  avgReward?: number;
}

type Phase = 'human' | 'mcts' | 'gameover';

function NimSolverDemoInner() {
  const { useWasm } = require('../treant/WasmProvider');
  const TreeVisualization = require('../treant/TreeVisualization').default;
  const StatsPanel = require('../treant/StatsPanel').default;
  const ParameterControls = require('../treant/ParameterControls').default;
  const NimBoard = require('../treant/GameBoard/NimBoard').default;

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
  const [moveHistory, setMoveHistory] = useState<MoveEntry[]>([]);

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
      setMoveHistory([]);

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

  const recordMove = useCallback(
    (player: string, move: string, statsBeforeMove: SearchStats | null) => {
      if (!gameRef.current) return;
      const chosenChild = statsBeforeMove?.children.find(
        (c) => c.mov === move,
      );
      const entry: MoveEntry = {
        turn: 0, // filled below
        player,
        move: move === 'Take1' ? 'Take 1' : 'Take 2',
        stonesAfter: gameRef.current.current_stones(),
        proven: statsBeforeMove
          ? (chosenChild?.proven ?? 'Unknown')
          : 'Unknown',
        bestMove: statsBeforeMove?.best_move
          ? statsBeforeMove.best_move === 'Take1'
            ? 'Take 1'
            : 'Take 2'
          : undefined,
        avgReward: chosenChild?.avg_reward,
      };
      setMoveHistory((prev) => {
        entry.turn = prev.length + 1;
        return [...prev, entry];
      });
    },
    [],
  );

  const handleHumanMove = useCallback(
    (move: 'Take1' | 'Take2') => {
      if (!gameRef.current || phase !== 'human') return;

      const statsBefore = stats;
      gameRef.current.apply_move(move);
      recordMove('P1', move, statsBefore);
      syncState();

      if (gameRef.current.is_terminal()) {
        const s = gameRef.current.get_stats();
        setStats(s);
        setTree(null);
        setPhase('gameover');
        setWinner(gameRef.current.current_player() === 'P1' ? 'P2' : 'P1');
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
        setTimeout(() => {
          if (!gameRef.current) return;
          const mctsStatsBefore: SearchStats = gameRef.current.get_stats();
          gameRef.current.apply_move(bestMove);
          recordMove('P2 (MCTS)', bestMove, mctsStatsBefore);
          syncState();

          if (gameRef.current.is_terminal()) {
            const finalStats = gameRef.current.get_stats();
            setStats(finalStats);
            setTree(null);
            setPhase('gameover');
            setWinner(gameRef.current.current_player() === 'P1' ? 'P2' : 'P1');
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
    [phase, syncState, stats, recordMove],
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

  const provenStatus = provenValue.toLowerCase() as
    | 'win'
    | 'loss'
    | 'draw'
    | 'unknown';

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
        <button
          className="button button--sm button--outline button--danger"
          onClick={() => initGame(startingStones)}
          style={{ marginTop: '0.5rem' }}
        >
          Reset
        </button>
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

      {moveHistory.length > 0 && (
        <div className={styles.section}>
          <div className={styles.sectionLabel}>Move history</div>
          <table className={styles.moveHistory}>
            <thead>
              <tr>
                <th>#</th>
                <th>Player</th>
                <th>Move</th>
                <th>Stones left</th>
                <th>MCTS eval</th>
                <th>MCTS best</th>
              </tr>
            </thead>
            <tbody>
              {moveHistory.map((m) => (
                <tr key={m.turn}>
                  <td>{m.turn}</td>
                  <td>{m.player}</td>
                  <td>{m.move}</td>
                  <td>{m.stonesAfter}</td>
                  <td
                    style={{
                      color:
                        m.avgReward != null && m.avgReward > 0
                          ? '#22c55e'
                          : m.avgReward != null && m.avgReward < 0
                            ? '#ef4444'
                            : undefined,
                    }}
                  >
                    {m.avgReward != null ? m.avgReward.toFixed(1) : '—'}
                  </td>
                  <td
                    style={{
                      color:
                        m.bestMove && m.bestMove !== m.move
                          ? '#eab308'
                          : undefined,
                    }}
                  >
                    {m.bestMove ?? '—'}
                    {m.bestMove && m.bestMove !== m.move ? ' !' : ''}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {stats && (
        <div className={styles.section}>
          <div className={styles.sectionLabel}>
            MCTS analysis (from {currentPlayer}'s perspective)
          </div>
          <StatsPanel
            totalPlayouts={stats.total_playouts}
            totalNodes={stats.total_nodes}
            bestMove={stats.best_move}
            children={stats.children.map((c) => ({
              ...c,
              proven: c.proven === 'Win' ? 'Loss' : c.proven === 'Loss' ? 'Win' : c.proven,
            }))}
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
        const { WasmProvider } = require('../treant/WasmProvider');
        return (
          <WasmProvider>
            <NimSolverDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
