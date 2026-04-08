import { useCallback, useEffect, useRef, useState } from 'react';
import BrowserOnly from '@docusaurus/BrowserOnly';
import styles from './demos.module.css';
import bs from './ConnectFourDemo.module.css';

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

const PLAYER_COLORS = ['#ef4444', '#eab308', '#22c55e', '#8b5cf6'];
const PLAYER_NAMES = ['Red', 'Yellow', 'Green', 'Purple'];
const CELL_CLASSES = [bs.cellP1, bs.cellP2, bs.cellP3, bs.cellP4];
const DOT_CLASSES = [bs.dotP1, bs.dotP2, bs.dotP3, bs.dotP4];

const PLAYOUT_OPTIONS = [
  { label: '1,000', value: 1000 },
  { label: '5,000', value: 5000 },
  { label: '20,000', value: 20000 },
  { label: '50,000', value: 50000 },
];

function ConnectFourDemoInner() {
  const { useWasm } = require('../mcts/WasmProvider');
  const { wasm, ready, error } = useWasm();
  const gameRef = useRef<any>(null);

  // Settings
  const [sCols, setSCols] = useState(7);
  const [sRows, setSRows] = useState(6);
  const [sK, setSK] = useState(4);
  const [sPlayers, setSPlayers] = useState(2);
  const [playerPlayouts, setPlayerPlayouts] = useState<number[]>([20000, 20000]);
  const [playerTypes, setPlayerTypes] = useState<string[]>(['human', 'mcts']);

  // Game state
  const [board, setBoard] = useState('');
  const [cols, setCols] = useState(7);
  const [rows, setRows] = useState(6);
  const [numPlayers, setNumPlayers] = useState(2);
  const [currentPlayer, setCurrentPlayer] = useState(0);
  const [gameOver, setGameOver] = useState(false);
  const [resultText, setResultText] = useState('');
  const [stats, setStats] = useState<SearchStats | null>(null);
  const [thinking, setThinking] = useState(false);

  const playerPlayoutsRef = useRef<number[]>([20000, 20000]);
  const playerTypesRef = useRef(['human', 'mcts']);
  useEffect(() => { playerPlayoutsRef.current = playerPlayouts; }, [playerPlayouts]);
  useEffect(() => { playerTypesRef.current = playerTypes; }, [playerTypes]);

  // Adjust playerTypes and playerPlayouts arrays when player count changes
  useEffect(() => {
    setPlayerTypes((prev) => {
      const next = Array.from({ length: sPlayers }, (_, i) =>
        i < prev.length ? prev[i] : 'mcts'
      );
      return next;
    });
    setPlayerPlayouts((prev) => {
      const defaultVal = 20000;
      return Array.from({ length: sPlayers }, (_, i) => (i < prev.length ? prev[i] : defaultVal));
    });
  }, [sPlayers]);

  const syncState = useCallback(() => {
    if (!gameRef.current) return;
    setBoard(gameRef.current.get_board());
    setCurrentPlayer(gameRef.current.current_player());
    setGameOver(gameRef.current.is_terminal());
    const result = gameRef.current.result();
    if (result) {
      if (result === 'Draw') {
        setResultText("It's a draw!");
      } else {
        const pIdx = parseInt(result, 10) - 1;
        setResultText(`${PLAYER_NAMES[pIdx] ?? `Player ${result}`} wins!`);
      }
    } else {
      setResultText('');
    }
  }, []);

  const runAnalysis = useCallback(() => {
    if (!gameRef.current || gameRef.current.is_terminal()) {
      setStats(null);
      return;
    }
    gameRef.current.playout_n(playerPlayoutsRef.current[gameRef.current.current_player()]);
    setStats(gameRef.current.get_stats());
  }, []);

  const initGame = useCallback(() => {
    if (!wasm) return;
    if (gameRef.current) gameRef.current.free();
    gameRef.current = new wasm.ConnectFourWasm(sCols, sRows, sK, sPlayers);
    setCols(gameRef.current.cols());
    setRows(gameRef.current.rows());
    setNumPlayers(gameRef.current.num_players());
    setThinking(false);
    setStats(null);
    setResultText('');
    setGameOver(false);
    syncState();
    runAnalysis();
    // If first player is MCTS, kick off their turn
    const firstPlayer = gameRef.current.current_player();
    if (playerTypesRef.current[firstPlayer] === 'mcts') {
      setTimeout(() => triggerMCTSTurn(), 100);
    }
  }, [wasm, sCols, sRows, sK, sPlayers, syncState, runAnalysis]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (ready) initGame();
    return () => {
      if (gameRef.current) {
        gameRef.current.free();
        gameRef.current = null;
      }
    };
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  // Core move logic: plays move, checks game over, triggers next turn
  const playMove = useCallback((col: number) => {
    if (!gameRef.current || gameRef.current.is_terminal()) return;
    const ok = gameRef.current.apply_move(col.toString());
    if (!ok) return;
    syncState();
    if (gameRef.current.is_terminal()) {
      setGameOver(true);
      syncState();
      runAnalysis(); // final analysis
      return;
    }
    // Always run analysis after a move
    runAnalysis();
    // Check if next player is MCTS
    const nextPlayer = gameRef.current.current_player();
    if (playerTypesRef.current[nextPlayer] === 'mcts') {
      triggerMCTSTurn();
    }
  }, [syncState, runAnalysis]);

  const triggerMCTSTurn = useCallback(() => {
    setThinking(true);
    setTimeout(() => {
      if (!gameRef.current || gameRef.current.is_terminal()) {
        setThinking(false);
        return;
      }
      gameRef.current.playout_n(playerPlayoutsRef.current[gameRef.current.current_player()]);
      const s = gameRef.current.get_stats();
      setStats(s);
      const best = gameRef.current.best_move();
      if (best) {
        gameRef.current.apply_move(best);
        syncState();
      }
      setThinking(false);
      if (gameRef.current.is_terminal()) {
        setGameOver(true);
        syncState();
        return;
      }
      // Run analysis for display
      runAnalysis();
      // Chain: if next player is also MCTS, keep going
      const nextPlayer = gameRef.current.current_player();
      if (playerTypesRef.current[nextPlayer] === 'mcts') {
        setTimeout(() => triggerMCTSTurn(), 50);
      }
    }, 50);
  }, [syncState, runAnalysis]);

  const handleColumnClick = useCallback((col: number) => {
    if (!gameRef.current || gameOver || thinking) return;
    const cp = gameRef.current.current_player();
    if (playerTypesRef.current[cp] !== 'human') return;
    playMove(col);
  }, [gameOver, thinking, playMove]);

  if (error) return <div className={styles.error}>Failed to load WASM: {error}</div>;
  if (!ready) return <div className={styles.loading}>Loading...</div>;

  // Parse board
  const cells: string[] = [];
  for (let i = 0; i < cols * rows; i++) {
    cells.push(board[i] || ' ');
  }

  const maxVisits = stats ? Math.max(...stats.children.map((c) => c.visits), 1) : 1;
  const isHumanTurn = !gameOver && !thinking && playerTypes[currentPlayer] === 'human';

  return (
    <div className={styles.demo}>
      {/* Settings */}
      <div className={bs.settingsRow}>
        <label className={bs.settingLabel}>
          <span>Width</span>
          <select value={sCols} onChange={(e) => setSCols(Number(e.target.value))} className={bs.select}>
            {[3,4,5,6,7,8,9,10].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <label className={bs.settingLabel}>
          <span>Height</span>
          <select value={sRows} onChange={(e) => setSRows(Number(e.target.value))} className={bs.select}>
            {[3,4,5,6,7,8,9,10].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <label className={bs.settingLabel}>
          <span>In a Row</span>
          <select value={sK} onChange={(e) => setSK(Number(e.target.value))} className={bs.select}>
            {[3,4,5,6,7].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <label className={bs.settingLabel}>
          <span>Players</span>
          <select value={sPlayers} onChange={(e) => setSPlayers(Number(e.target.value))} className={bs.select}>
            {[2,3,4].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <button
          className="button button--sm button--outline button--primary"
          onClick={initGame}
          style={{ alignSelf: 'flex-end', marginBottom: '0.2rem' }}
        >
          New Game
        </button>
      </div>

      {/* Player type toggles */}
      <div className={bs.playerRow}>
        {Array.from({ length: sPlayers }, (_, i) => (
          <div key={i} className={bs.playerBadge}>
            <span className={`${bs.dot} ${DOT_CLASSES[i]}`} />
            <span>{PLAYER_NAMES[i]}</span>
            <select
              value={playerTypes[i] || 'mcts'}
              onChange={(e) => {
                setPlayerTypes((prev) => {
                  const next = [...prev];
                  next[i] = e.target.value;
                  return next;
                });
              }}
            >
              <option value="human">Human</option>
              <option value="mcts">MCTS</option>
            </select>
            <select
              value={playerPlayouts[i] || 20000}
              onChange={(e) => {
                setPlayerPlayouts((prev) => {
                  const next = [...prev];
                  next[i] = Number(e.target.value);
                  return next;
                });
              }}
              style={{ fontSize: '0.7rem', padding: '0.125rem 0.25rem', width: '4.5rem' }}
            >
              {PLAYOUT_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>{o.label}</option>
              ))}
            </select>
          </div>
        ))}
      </div>

      <div className={bs.boardInfo}>
        {cols}x{rows}, {gameRef.current?.win_length() ?? sK}-in-a-row, {numPlayers} players
      </div>

      {/* Status */}
      <div className={bs.statusBar}>
        {gameOver ? null : thinking ? (
          <span className={styles.thinking}>
            MCTS ({PLAYER_NAMES[currentPlayer]}) is thinking...
          </span>
        ) : isHumanTurn ? (
          <span>
            <span style={{ color: PLAYER_COLORS[currentPlayer] }}>{PLAYER_NAMES[currentPlayer]}</span>
            &apos;s turn — click a column
          </span>
        ) : null}
      </div>

      {/* Board */}
      <div className={styles.section} style={{ display: 'flex', justifyContent: 'center' }}>
        <div className={bs.board}>
          <div className={bs.columnHeaders} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
            {Array.from({ length: cols }, (_, col) => (
              <button
                key={col}
                className={bs.columnButton}
                onClick={() => handleColumnClick(col)}
                disabled={!isHumanTurn}
                title={`Drop in column ${col + 1}`}
              >
                &#x25BC;
              </button>
            ))}
          </div>
          <div className={bs.grid} style={{
            gridTemplateColumns: `repeat(${cols}, 1fr)`,
            gridTemplateRows: `repeat(${rows}, 1fr)`,
          }}>
            {cells.map((cell, i) => {
              const pIdx = cell === ' ' ? -1 : parseInt(cell, 10) - 1;
              const cellClass = pIdx >= 0 ? CELL_CLASSES[pIdx] ?? bs.cellP1 : bs.cellEmpty;
              return (
                <div key={i} className={`${bs.cell} ${cellClass}`} />
              );
            })}
          </div>
        </div>
      </div>

      {/* Game over */}
      {gameOver && (
        <div className={styles.gameOver}>
          <p>{resultText}</p>
          <button className="button button--sm button--outline button--primary" onClick={initGame}>
            New Game
          </button>
        </div>
      )}

      {/* MCTS analysis — always shown */}
      {stats && stats.children.length > 0 && (
        <div className={styles.section}>
          <div className={styles.sectionLabel}>
            MCTS analysis — {PLAYER_NAMES[currentPlayer]}&apos;s move ({(playerPlayouts[currentPlayer] || 20000).toLocaleString()} playouts)
          </div>
          <table className={bs.analysisTable}>
            <thead>
              <tr>
                <th>Column</th>
                <th>Visits</th>
                <th>Avg Reward</th>
                <th className={bs.barCell}>Distribution</th>
              </tr>
            </thead>
            <tbody>
              {stats.children
                .slice()
                .sort((a, b) => parseInt(a.mov) - parseInt(b.mov))
                .map((child) => {
                  const isBest = stats.best_move === child.mov;
                  const pct = (child.visits / maxVisits) * 100;
                  return (
                    <tr key={child.mov}>
                      <td className={isBest ? bs.bestCol : ''}>
                        {parseInt(child.mov) + 1}
                        {isBest ? ' *' : ''}
                      </td>
                      <td className={bs.mono}>{child.visits.toLocaleString()}</td>
                      <td className={bs.mono}>{child.avg_reward.toFixed(3)}</td>
                      <td className={bs.barCell}>
                        <div className={bs.bar} style={{ width: `${pct}%` }} />
                      </td>
                    </tr>
                  );
                })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export default function ConnectFourDemo() {
  return (
    <BrowserOnly fallback={<div className={styles.loading}>Loading...</div>}>
      {() => {
        const { WasmProvider } = require('../mcts/WasmProvider');
        return (
          <WasmProvider>
            <ConnectFourDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
