import { useCallback, useEffect, useRef, useState } from 'react';
import BrowserOnly from '@docusaurus/BrowserOnly';
import sharedStyles from './demos.module.css';
import styles from './ShiftDemo.module.css';

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

const PLAYER_NAMES = ['X', 'O', 'A', 'B'];
const PLAYER_COLORS = ['#3b82f6', '#ef4444', '#22c55e', '#8b5cf6'];

const PLAYOUT_OPTIONS = [
  { label: '2,000', value: 2000 },
  { label: '5,000', value: 5000 },
  { label: '20,000', value: 20000 },
];

function ShiftDemoInner() {
  const { useWasm } = require('../mcts/WasmProvider');
  const { wasm, ready, error } = useWasm();
  const gameRef = useRef<any>(null);

  // Settings
  const [sCols, setSCols] = useState(3);
  const [sRows, setSRows] = useState(3);
  const [sK, setSK] = useState(3);
  const [sPieces, setSPieces] = useState(3);
  const [sPlayers, setSPlayers] = useState(2);
  const [playouts, setPlayouts] = useState(5000);
  const [playerTypes, setPlayerTypes] = useState<string[]>(['human', 'mcts']);

  // Game state
  const [board, setBoard] = useState('');
  const [cols, setCols] = useState(3);
  const [rows, setRows] = useState(3);
  const [currentPlayer, setCurrentPlayer] = useState(0);
  const [isPlacement, setIsPlacement] = useState(true);
  const [gameOver, setGameOver] = useState(false);
  const [resultText, setResultText] = useState('');
  const [stats, setStats] = useState<SearchStats | null>(null);
  const [provenValue, setProvenValue] = useState('Unknown');
  const [thinking, setThinking] = useState(false);
  const [selectedPiece, setSelectedPiece] = useState<number | null>(null);

  const playoutsRef = useRef(5000);
  const playerTypesRef = useRef(['human', 'mcts']);
  useEffect(() => { playoutsRef.current = playouts; }, [playouts]);
  useEffect(() => { playerTypesRef.current = playerTypes; }, [playerTypes]);

  useEffect(() => {
    setPlayerTypes((prev) =>
      Array.from({ length: sPlayers }, (_, i) => (i < prev.length ? prev[i] : 'mcts'))
    );
  }, [sPlayers]);

  const syncState = useCallback(() => {
    if (!gameRef.current) return;
    setBoard(gameRef.current.get_board());
    setCurrentPlayer(gameRef.current.current_player());
    setIsPlacement(gameRef.current.in_placement_phase());
    setGameOver(gameRef.current.is_terminal());
    const result = gameRef.current.result();
    if (result) {
      const pIdx = parseInt(result, 10) - 1;
      setResultText(`${PLAYER_NAMES[pIdx] ?? `Player ${result}`} wins!`);
    } else {
      setResultText('');
    }
  }, []);

  const runAnalysis = useCallback(() => {
    if (!gameRef.current || gameRef.current.is_terminal()) {
      setStats(null);
      return;
    }
    gameRef.current.playout_n(playoutsRef.current);
    setStats(gameRef.current.get_stats());
    setProvenValue(gameRef.current.root_proven_value());
  }, []);

  const triggerMCTSTurn = useCallback(() => {
    setThinking(true);
    setTimeout(() => {
      if (!gameRef.current || gameRef.current.is_terminal()) {
        setThinking(false);
        syncState();
        return;
      }
      gameRef.current.playout_n(playoutsRef.current);
      const s = gameRef.current.get_stats();
      setStats(s);
      setProvenValue(gameRef.current.root_proven_value());
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
      const nextPlayer = gameRef.current.current_player();
      if (playerTypesRef.current[nextPlayer] === 'mcts') {
        setTimeout(() => triggerMCTSTurn(), 50);
      }
    }, 50);
  }, [syncState, runAnalysis]);

  const initGame = useCallback(() => {
    if (!wasm) return;
    if (gameRef.current) gameRef.current.free();
    gameRef.current = new wasm.ShiftWasm(sCols, sRows, sK, sPlayers, sPieces);
    setCols(gameRef.current.cols());
    setRows(gameRef.current.rows());
    setThinking(false);
    setStats(null);
    setProvenValue('Unknown');
    setResultText('');
    setGameOver(false);
    setSelectedPiece(null);
    syncState();
    runAnalysis();
    const firstPlayer = gameRef.current.current_player();
    if (playerTypesRef.current[firstPlayer] === 'mcts') {
      setTimeout(() => triggerMCTSTurn(), 100);
    }
  }, [wasm, sCols, sRows, sK, sPlayers, sPieces, syncState, runAnalysis]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (ready) initGame();
    return () => {
      if (gameRef.current) {
        gameRef.current.free();
        gameRef.current = null;
      }
    };
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  const afterMove = useCallback(() => {
    syncState();
    setSelectedPiece(null);
    if (gameRef.current.is_terminal()) {
      setGameOver(true);
      syncState();
      return;
    }
    runAnalysis();
    const nextPlayer = gameRef.current.current_player();
    if (playerTypesRef.current[nextPlayer] === 'mcts') {
      triggerMCTSTurn();
    }
  }, [syncState, runAnalysis, triggerMCTSTurn]);

  const handleCellClick = useCallback((index: number) => {
    if (!gameRef.current || gameOver || thinking) return;
    const cp = gameRef.current.current_player();
    if (playerTypesRef.current[cp] !== 'human') return;

    const currentBoard = gameRef.current.get_board();
    const cellChar = currentBoard[index];
    const mySymbol = PLAYER_NAMES[cp];

    if (isPlacement) {
      if (cellChar !== ' ') return;
      const ok = gameRef.current.apply_move(`P${index}`);
      if (ok) afterMove();
    } else {
      if (selectedPiece === null) {
        if (cellChar === mySymbol) {
          setSelectedPiece(index);
        }
      } else {
        if (index === selectedPiece) {
          setSelectedPiece(null);
        } else if (cellChar === ' ') {
          const ok = gameRef.current.apply_move(`M${selectedPiece},${index}`);
          if (ok) afterMove();
          else setSelectedPiece(null);
        } else if (cellChar === mySymbol) {
          setSelectedPiece(index);
        }
      }
    }
  }, [gameOver, thinking, isPlacement, selectedPiece, afterMove]);

  if (error) return <div className={sharedStyles.error}>Failed to load WASM: {error}</div>;
  if (!ready) return <div className={sharedStyles.loading}>Loading...</div>;

  const isHumanTurn = !gameOver && !thinking && playerTypes[currentPlayer] === 'human';
  const provenStatus = provenValue.toLowerCase() as 'win' | 'loss' | 'draw' | 'unknown';
  const displayChildren = (stats?.children ?? []).slice().sort((a, b) => b.visits - a.visits);

  const cellSize = cols <= 5 ? 56 : cols <= 7 ? 44 : 36;
  const fontSize = cols <= 5 ? '1.5rem' : cols <= 7 ? '1.1rem' : '0.9rem';

  const formatMove = (mov: string) => {
    if (mov.startsWith('P')) {
      const cell = parseInt(mov.slice(1), 10);
      const r = Math.floor(cell / cols) + 1;
      const c = (cell % cols) + 1;
      return `Place (${r},${c})`;
    } else if (mov.startsWith('M')) {
      const parts = mov.slice(1).split(',');
      const from = parseInt(parts[0], 10);
      const to = parseInt(parts[1], 10);
      return `(${Math.floor(from / cols) + 1},${(from % cols) + 1})\u2192(${Math.floor(to / cols) + 1},${(to % cols) + 1})`;
    }
    return mov;
  };

  return (
    <div className={sharedStyles.demo}>
      {/* Settings */}
      <div className={styles.settingsRow}>
        <label className={styles.settingLabel}>
          <span>Width</span>
          <select value={sCols} onChange={(e) => setSCols(Number(e.target.value))} className={styles.select}>
            {[2,3,4,5,6,7,8].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <label className={styles.settingLabel}>
          <span>Height</span>
          <select value={sRows} onChange={(e) => setSRows(Number(e.target.value))} className={styles.select}>
            {[2,3,4,5,6,7,8].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <label className={styles.settingLabel}>
          <span>In a Row</span>
          <select value={sK} onChange={(e) => setSK(Number(e.target.value))} className={styles.select}>
            {[2,3,4,5,6].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <label className={styles.settingLabel}>
          <span>Pieces</span>
          <select value={sPieces} onChange={(e) => setSPieces(Number(e.target.value))} className={styles.select}>
            {[1,2,3,4,5,6,7,8].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <label className={styles.settingLabel}>
          <span>Players</span>
          <select value={sPlayers} onChange={(e) => setSPlayers(Number(e.target.value))} className={styles.select}>
            {[2,3,4].map((v) => <option key={v} value={v}>{v}</option>)}
          </select>
        </label>
        <label className={styles.settingLabel}>
          <span>Playouts</span>
          <select value={playouts} onChange={(e) => setPlayouts(Number(e.target.value))} className={styles.select}>
            {PLAYOUT_OPTIONS.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
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
      <div className={styles.settingsRow}>
        {Array.from({ length: sPlayers }, (_, i) => (
          <div key={i} style={{
            display: 'inline-flex', alignItems: 'center', gap: '0.375rem',
            padding: '0.25rem 0.5rem', borderRadius: '4px', fontSize: '0.75rem',
            fontWeight: 600, border: '1px solid var(--ifm-color-emphasis-200)',
          }}>
            <span style={{
              display: 'inline-block', width: 14, height: 14, borderRadius: '50%',
              background: PLAYER_COLORS[i],
            }} />
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
              style={{ fontSize: '0.75rem', padding: '0.125rem 0.25rem' }}
            >
              <option value="human">Human</option>
              <option value="mcts">MCTS</option>
            </select>
          </div>
        ))}
      </div>

      <div className={styles.boardInfo}>
        {cols}x{rows}, {gameRef.current?.win_length() ?? sK}-in-a-row, {gameRef.current?.pieces_per_player() ?? sPieces} pieces each
      </div>

      {/* Status */}
      <div className={styles.status}>
        {gameOver ? null : thinking ? (
          <span className={sharedStyles.thinking}>
            MCTS ({PLAYER_NAMES[currentPlayer]}) is thinking...
          </span>
        ) : isHumanTurn ? (
          <span>
            <span style={{ color: PLAYER_COLORS[currentPlayer] }}>{PLAYER_NAMES[currentPlayer]}</span>
            &apos;s turn
          </span>
        ) : null}
      </div>

      {/* Phase hint */}
      {isHumanTurn && !gameOver && (
        <div className={styles.phaseHint}>
          {isPlacement
            ? 'Click an empty cell to place your piece'
            : selectedPiece !== null
              ? 'Click an empty cell to move there, or click your piece to deselect'
              : 'Click one of your pieces to select it, then click where to move'}
        </div>
      )}

      {/* Board */}
      <div className={styles.board} style={{ gridTemplateColumns: `repeat(${cols}, ${cellSize}px)` }}>
        {board.split('').slice(0, cols * rows).map((cell, i) => {
          const pIdx = cell === ' ' ? -1 : PLAYER_NAMES.indexOf(cell);
          const color = pIdx >= 0 ? PLAYER_COLORS[pIdx] : undefined;
          const isMyPiece = pIdx === currentPlayer;
          const isSelected = selectedPiece === i;
          const canClick = isHumanTurn && (
            (isPlacement && cell === ' ') ||
            (!isPlacement && (isMyPiece || (selectedPiece !== null && cell === ' ')))
          );

          return (
            <button
              key={i}
              className={[
                styles.cell,
                !canClick && !isSelected ? styles.cellDisabled : '',
                isSelected ? styles.cellSelected : '',
                !isPlacement && isMyPiece && !isSelected && isHumanTurn ? styles.cellMovable : '',
              ].filter(Boolean).join(' ')}
              style={{ width: cellSize, height: cellSize, fontSize, color: color ?? 'inherit' }}
              onClick={() => handleCellClick(i)}
              disabled={!canClick && !isSelected}
            >
              {cell === ' ' ? '' : cell}
            </button>
          );
        })}
      </div>

      {/* Game over */}
      {gameOver && (
        <div className={sharedStyles.gameOver}>
          <p>{resultText}</p>
          <button className="button button--sm button--outline button--primary" onClick={initGame}>
            New Game
          </button>
        </div>
      )}

      {/* Proven value badge */}
      {!gameOver && provenValue !== 'Unknown' && (
        <div className={sharedStyles.section}>
          <span className={sharedStyles.provenValue} data-status={provenStatus}>
            Solver: {provenValue}
          </span>
        </div>
      )}

      {/* MCTS analysis */}
      {stats && displayChildren.length > 0 && (
        <div className={sharedStyles.section}>
          <div className={sharedStyles.sectionLabel}>
            MCTS analysis — {PLAYER_NAMES[currentPlayer]}&apos;s move ({stats.total_playouts.toLocaleString()} playouts)
          </div>
          <table className={styles.analysisTable}>
            <thead>
              <tr>
                <th>Move</th>
                <th>Visits</th>
                <th>Avg Reward</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {displayChildren.slice(0, 12).map((c) => (
                <tr key={c.mov}>
                  <td>{formatMove(c.mov)}</td>
                  <td>{c.visits.toLocaleString()}</td>
                  <td>{c.avg_reward.toFixed(3)}</td>
                  <td className={styles.statusCell} data-status={c.proven?.toLowerCase()}>
                    {c.proven ?? '—'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export default function ShiftDemo() {
  return (
    <BrowserOnly fallback={<div className={sharedStyles.loading}>Loading...</div>}>
      {() => {
        const { WasmProvider } = require('../mcts/WasmProvider');
        return (
          <WasmProvider>
            <ShiftDemoInner />
          </WasmProvider>
        );
      }}
    </BrowserOnly>
  );
}
