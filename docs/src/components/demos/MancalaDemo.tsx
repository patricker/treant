import { useCallback, useEffect, useRef, useState } from 'react';
import BrowserOnly from '@docusaurus/BrowserOnly';
import styles from './demos.module.css';
import bs from './MancalaDemo.module.css';

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

const PLAYER_COLORS = ['#ef4444', '#3b82f6', '#22c55e', '#eab308'];
const PLAYER_NAMES = ['Red', 'Blue', 'Green', 'Yellow'];

const PLAYOUT_OPTIONS = [
  { label: '1,000', value: 1000 },
  { label: '5,000', value: 5000 },
  { label: '20,000', value: 20000 },
  { label: '50,000', value: 50000 },
];

function MancalaDemoInner() {
  const { useWasm } = require('../treant/WasmProvider');
  const { wasm, ready, error } = useWasm();
  const gameRef = useRef<any>(null);

  const [sPits, setSPits] = useState(6);
  const [sStones, setSStones] = useState(4);
  const [sPlayers, setSPlayers] = useState(2);
  const [playerPlayouts, setPlayerPlayouts] = useState<number[]>([20000, 20000]);
  const [playerTypes, setPlayerTypes] = useState<string[]>(['human', 'mcts']);

  const [board, setBoard] = useState<number[]>([]);
  const [pits, setPits] = useState(6);
  const [numPlayers, setNumPlayers] = useState(2);
  const [currentPlayer, setCurrentPlayer] = useState(0);
  const [scores, setScores] = useState<number[]>([0, 0]);
  const [gameOver, setGameOver] = useState(false);
  const [resultText, setResultText] = useState('');
  const [stats, setStats] = useState<SearchStats | null>(null);
  const [thinking, setThinking] = useState(false);

  const playerPlayoutsRef = useRef([20000, 20000]);
  const playerTypesRef = useRef(['human', 'mcts']);
  useEffect(() => { playerPlayoutsRef.current = playerPlayouts; }, [playerPlayouts]);
  useEffect(() => { playerTypesRef.current = playerTypes; }, [playerTypes]);

  // Re-shape arrays when sPlayers changes.
  useEffect(() => {
    setPlayerTypes((prev) =>
      Array.from({ length: sPlayers }, (_, i) => (i < prev.length ? prev[i] : 'mcts'))
    );
    setPlayerPlayouts((prev) =>
      Array.from({ length: sPlayers }, (_, i) => (i < prev.length ? prev[i] : 20000))
    );
  }, [sPlayers]);

  const syncState = useCallback(() => {
    if (!gameRef.current) return;
    const boardStr: string = gameRef.current.get_board();
    setBoard(boardStr.split(',').map(Number));
    setCurrentPlayer(gameRef.current.current_player());
    setGameOver(gameRef.current.is_terminal());
    const scoreStr: string = gameRef.current.scores();
    setScores(scoreStr.split(',').map(Number));
    const result: string = gameRef.current.result();
    if (result === 'Draw') {
      setResultText("It's a draw!");
    } else if (result.startsWith('P')) {
      const idx = parseInt(result.slice(1), 10) - 1;
      setResultText(`${PLAYER_NAMES[idx] ?? result} wins!`);
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
    gameRef.current = new wasm.MancalaWasm(sPits, sStones, sPlayers);
    setPits(gameRef.current.pits());
    setNumPlayers(gameRef.current.num_players());
    setThinking(false);
    setStats(null);
    setResultText('');
    setGameOver(false);
    syncState();
    runAnalysis();
    const firstPlayer = gameRef.current.current_player();
    if (playerTypesRef.current[firstPlayer] === 'mcts') {
      setTimeout(() => triggerMCTSTurn(), 100);
    }
  }, [wasm, sPits, sStones, sPlayers, syncState, runAnalysis]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (ready) initGame();
    return () => {
      if (gameRef.current) {
        gameRef.current.free();
        gameRef.current = null;
      }
    };
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  const playMove = useCallback((local: number) => {
    if (!gameRef.current || gameRef.current.is_terminal()) return;
    const ok = gameRef.current.apply_move(local.toString());
    if (!ok) return;
    syncState();
    if (gameRef.current.is_terminal()) {
      setGameOver(true);
      syncState();
      runAnalysis();
      return;
    }
    runAnalysis();
    const next = gameRef.current.current_player();
    if (playerTypesRef.current[next] === 'mcts') triggerMCTSTurn();
  }, [syncState, runAnalysis]);

  const triggerMCTSTurn = useCallback(() => {
    setThinking(true);
    setTimeout(() => {
      if (!gameRef.current || gameRef.current.is_terminal()) {
        setThinking(false);
        return;
      }
      gameRef.current.playout_n(playerPlayoutsRef.current[gameRef.current.current_player()]);
      setStats(gameRef.current.get_stats());
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
      runAnalysis();
      const next = gameRef.current.current_player();
      if (playerTypesRef.current[next] === 'mcts') {
        setTimeout(() => triggerMCTSTurn(), 50);
      }
    }, 50);
  }, [syncState, runAnalysis]);

  const handlePitClick = useCallback((player: number, local: number) => {
    if (!gameRef.current || gameOver || thinking) return;
    if (player !== currentPlayer) return;
    if (playerTypesRef.current[currentPlayer] !== 'human') return;
    const ringIdx = player * (pits + 1) + local;
    if (!board[ringIdx]) return;
    playMove(local);
  }, [gameOver, thinking, currentPlayer, pits, board, playMove]);

  if (error) return <div className={styles.error}>Failed to load WASM: {error}</div>;
  if (!ready) return <div className={styles.loading}>Loading...</div>;

  // Per-player pit stones, indexed by LOCAL pit index (0..pits-1).
  const playerPits = (p: number) =>
    Array.from({ length: pits }, (_, j) => board[p * (pits + 1) + j] ?? 0);

  // 2-player display: bottom row is P0 in local order; top row is P1 in
  // reversed local order (rightmost cell = P1 local 0, near P0's store).
  const p0Pits = playerPits(0);
  const p1Pits = playerPits(1);
  const p1RowDisplay = p1Pits.map((stones, local) => ({ stones, local })).reverse();

  // 4-player display orientation per edge (matches counterclockwise sowing):
  //   bottom (P0): left→right is local 0..pits-1
  //   right  (P1): top→bottom is local pits-1..0
  //   top    (P2): left→right is local pits-1..0
  //   left   (P3): top→bottom is local 0..pits-1
  const bottomRow = p0Pits.map((stones, local) => ({ stones, local }));
  const rightCol =
    numPlayers === 4
      ? playerPits(1)
          .map((stones, local) => ({ stones, local }))
          .reverse()
      : [];
  const topRow =
    numPlayers === 4
      ? playerPits(2)
          .map((stones, local) => ({ stones, local }))
          .reverse()
      : [];
  const leftCol =
    numPlayers === 4 ? playerPits(3).map((stones, local) => ({ stones, local })) : [];

  const maxVisits = stats ? Math.max(...stats.children.map((c) => c.visits), 1) : 1;
  const isHumanTurn = !gameOver && !thinking && playerTypes[currentPlayer] === 'human';

  const renderPit = (player: number, local: number, stones: number) => (
    <button
      key={`${player}-${local}`}
      className={`${bs.pit} ${currentPlayer === player && isHumanTurn ? bs.pitActive : ''}`}
      onClick={() => handlePitClick(player, local)}
      disabled={!(currentPlayer === player && isHumanTurn) || stones === 0}
      title={`${PLAYER_NAMES[player]} pit ${local + 1}: ${stones} stones`}
    >
      <span className={bs.pitStones}>{stones}</span>
    </button>
  );

  return (
    <div className={styles.demo}>
      {/* Settings */}
      <div className={bs.settingsRow}>
        <label className={bs.settingLabel}>
          <span>Pits</span>
          <select value={sPits} onChange={(e) => setSPits(Number(e.target.value))} className={bs.select}>
            {[3, 4, 5, 6, 7, 8].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <label className={bs.settingLabel}>
          <span>Stones</span>
          <select value={sStones} onChange={(e) => setSStones(Number(e.target.value))} className={bs.select}>
            {[2, 3, 4, 5, 6, 7, 8].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <label className={bs.settingLabel}>
          <span>Players</span>
          <select value={sPlayers} onChange={(e) => setSPlayers(Number(e.target.value))} className={bs.select}>
            <option value={2}>2</option>
            <option value={4}>4</option>
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

      {/* Player controls */}
      <div className={bs.playerRow}>
        {Array.from({ length: numPlayers }, (_, i) => (
          <div key={i} className={bs.playerBadge}>
            <span className={bs.dot} style={{ background: PLAYER_COLORS[i] }} />
            <span className={bs.playerName}>{PLAYER_NAMES[i]}</span>
            <span className={bs.playerScore}>{scores[i] ?? 0}</span>
            <select
              value={playerTypes[i] || 'mcts'}
              onChange={(e) => setPlayerTypes((prev) => {
                const next = [...prev];
                next[i] = e.target.value;
                return next;
              })}
            >
              <option value="human">Human</option>
              <option value="mcts">MCTS</option>
            </select>
            <select
              value={playerPlayouts[i] || 20000}
              onChange={(e) => setPlayerPlayouts((prev) => {
                const next = [...prev];
                next[i] = Number(e.target.value);
                return next;
              })}
              className={bs.playoutSelect}
            >
              {PLAYOUT_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>{o.label}</option>
              ))}
            </select>
          </div>
        ))}
      </div>

      {/* Status */}
      <div className={bs.statusBar}>
        {gameOver ? null : thinking ? (
          <span className={styles.thinking}>
            MCTS ({PLAYER_NAMES[currentPlayer]}) is thinking...
          </span>
        ) : isHumanTurn ? (
          <span>
            <span style={{ color: PLAYER_COLORS[currentPlayer] }}>
              {PLAYER_NAMES[currentPlayer]}
            </span>
            &apos;s turn — click one of your pits
          </span>
        ) : null}
      </div>

      {/* Board: 2-player layout */}
      {numPlayers === 2 && (
        <div className={styles.section} style={{ display: 'flex', justifyContent: 'center' }}>
          <div className={bs.board}>
            <div
              className={bs.store}
              style={{ borderColor: PLAYER_COLORS[1] }}
              title={`${PLAYER_NAMES[1]}'s store`}
            >
              <div className={bs.storeLabel}>{PLAYER_NAMES[1]}</div>
              <div className={bs.storeValue}>{scores[1] ?? 0}</div>
            </div>
            <div className={bs.center}>
              <div className={bs.pitRow}>
                {p1RowDisplay.map(({ stones, local }) => (
                  <button
                    key={local}
                    className={`${bs.pit} ${currentPlayer === 1 && isHumanTurn ? bs.pitActive : ''}`}
                    onClick={() => handlePitClick(1, local)}
                    disabled={!(currentPlayer === 1 && isHumanTurn) || stones === 0}
                    title={`${PLAYER_NAMES[1]} pit ${local + 1}: ${stones} stones`}
                  >
                    <span className={bs.pitStones}>{stones}</span>
                  </button>
                ))}
              </div>
              <div className={bs.pitRow}>
                {p0Pits.map((stones, local) => (
                  <button
                    key={local}
                    className={`${bs.pit} ${currentPlayer === 0 && isHumanTurn ? bs.pitActive : ''}`}
                    onClick={() => handlePitClick(0, local)}
                    disabled={!(currentPlayer === 0 && isHumanTurn) || stones === 0}
                    title={`${PLAYER_NAMES[0]} pit ${local + 1}: ${stones} stones`}
                  >
                    <span className={bs.pitStones}>{stones}</span>
                  </button>
                ))}
              </div>
            </div>
            <div
              className={bs.store}
              style={{ borderColor: PLAYER_COLORS[0] }}
              title={`${PLAYER_NAMES[0]}'s store`}
            >
              <div className={bs.storeLabel}>{PLAYER_NAMES[0]}</div>
              <div className={bs.storeValue}>{scores[0] ?? 0}</div>
            </div>
          </div>
        </div>
      )}

      {numPlayers === 4 && (
        <div className={styles.section} style={{ display: 'flex', justifyContent: 'center' }}>
          <div className={bs.boardSquare}>
            {/* Stores at the four corners */}
            <div
              className={`${bs.cornerStore} ${bs.cornerTL}`}
              style={{ borderColor: PLAYER_COLORS[2] }}
              title={`${PLAYER_NAMES[2]}'s store`}
            >
              <div className={bs.storeLabel}>{PLAYER_NAMES[2]}</div>
              <div className={bs.storeValue}>{scores[2] ?? 0}</div>
            </div>
            <div
              className={`${bs.cornerStore} ${bs.cornerTR}`}
              style={{ borderColor: PLAYER_COLORS[1] }}
              title={`${PLAYER_NAMES[1]}'s store`}
            >
              <div className={bs.storeLabel}>{PLAYER_NAMES[1]}</div>
              <div className={bs.storeValue}>{scores[1] ?? 0}</div>
            </div>
            <div
              className={`${bs.cornerStore} ${bs.cornerBL}`}
              style={{ borderColor: PLAYER_COLORS[3] }}
              title={`${PLAYER_NAMES[3]}'s store`}
            >
              <div className={bs.storeLabel}>{PLAYER_NAMES[3]}</div>
              <div className={bs.storeValue}>{scores[3] ?? 0}</div>
            </div>
            <div
              className={`${bs.cornerStore} ${bs.cornerBR}`}
              style={{ borderColor: PLAYER_COLORS[0] }}
              title={`${PLAYER_NAMES[0]}'s store`}
            >
              <div className={bs.storeLabel}>{PLAYER_NAMES[0]}</div>
              <div className={bs.storeValue}>{scores[0] ?? 0}</div>
            </div>

            {/* Edges */}
            <div className={bs.topRow}>
              {topRow.map(({ stones, local }) => renderPit(2, local, stones))}
            </div>
            <div className={bs.rightCol}>
              {rightCol.map(({ stones, local }) => renderPit(1, local, stones))}
            </div>
            <div className={bs.bottomRow}>
              {bottomRow.map(({ stones, local }) => renderPit(0, local, stones))}
            </div>
            <div className={bs.leftCol}>
              {leftCol.map(({ stones, local }) => renderPit(3, local, stones))}
            </div>

            <div className={bs.centerLabel}>Mancala</div>
          </div>
        </div>
      )}

      {gameOver && (
        <div className={styles.gameOver}>
          <p>{resultText}</p>
          <button className="button button--sm button--outline button--primary" onClick={initGame}>
            New Game
          </button>
        </div>
      )}

      {stats && stats.children.length > 0 && (
        <div className={styles.section}>
          <div className={styles.sectionLabel}>
            MCTS analysis — {PLAYER_NAMES[currentPlayer]}&apos;s move
            ({(playerPlayouts[currentPlayer] || 20000).toLocaleString()} playouts)
          </div>
          <table className={bs.analysisTable}>
            <thead>
              <tr>
                <th>Pit</th>
                <th>Visits</th>
                <th>Avg Reward</th>
                <th className={bs.barCell}>Distribution</th>
              </tr>
            </thead>
            <tbody>
              {stats.children
                .slice()
                .sort((a, b) => parseInt(a.mov, 10) - parseInt(b.mov, 10))
                .map((child) => {
                  const isBest = stats.best_move === child.mov;
                  const pct = (child.visits / maxVisits) * 100;
                  return (
                    <tr key={child.mov}>
                      <td className={isBest ? bs.bestPit : ''}>
                        {parseInt(child.mov, 10) + 1}{isBest ? ' *' : ''}
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

export default function MancalaDemo() {
  return (
    <BrowserOnly fallback={<div className={styles.loading}>Loading...</div>}>
      {() => {
        const { WasmProvider } = require('../treant/WasmProvider');
        return (
          <WasmProvider>
            <MancalaDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
