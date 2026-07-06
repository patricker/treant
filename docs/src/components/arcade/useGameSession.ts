import { useCallback, useEffect, useRef, useState } from 'react';
import type { GameDefinition, GameHandle, GameParams, PlayerKind } from './gameTypes';
import { aiConfig, pickAiMove } from './gameTypes';
import { diffCells } from './boardDiff';
import { sound } from './sound';

type Phase = 'playing' | 'thinking' | 'over';
const AI_DELAY_MS = 400;
const SOLO_AI_PLAYOUTS = 800;
const HINT_PLAYOUTS = 1500;

export function useGameSession(
  wasm: any,
  def: GameDefinition,
  params: GameParams,
  seats: PlayerKind[],
) {
  const handleRef = useRef<GameHandle | null>(null);
  // Generation guard: bumped whenever a game (re)starts or the hook tears down,
  // so any AI turn still waiting in a setTimeout from a previous game aborts
  // instead of mutating the new handle. `timerRef` lets us cancel it outright.
  const genRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Per-seat config: 'human' or an AI difficulty. Kept in a ref so the AI loop
  // always reads the latest seats without being re-created.
  const seatsRef = useRef(seats);
  seatsRef.current = seats;

  const [board, setBoard] = useState('');
  const [current, setCurrent] = useState(0);
  // Hidden-info pass-and-play only: the seat we must hand the phone to before
  // revealing the board (null = no pending handoff). Perfect-information games
  // never set it, so all handoff logic below is inert for them.
  const [handoff, setHandoff] = useState<number | null>(null);
  const [phase, setPhase] = useState<Phase>('playing');
  const [result, setResult] = useState('');
  const [statusText, setStatusText] = useState('');
  const [endText, setEndText] = useState('');
  const [legalMoves, setLegalMoves] = useState<string[]>([]);
  // Cells of the winning line, set when the game ends on a line (line games only).
  // Drives the board's win glow; cleared on start/undo.
  const [winCells, setWinCells] = useState<number[]>([]);
  // Cells changed by the most recent move (placement + any flips/captures) —
  // the board marks them so the AI's reply is findable at a glance. Diffed
  // here at move time (not in the board) so it survives unrelated re-renders.
  const [lastCells, setLastCells] = useState<number[]>([]);
  const [canUndo, setCanUndo] = useState(false);
  // Move log for undo: seat captured BEFORE the move applies. Undo replays the
  // prefix ending just before the last human move (popping any AI replies too).
  const movesRef = useRef<{ move: string; seat: number }[]>([]);

  // Which seat's view to render. Perfect-information games: null (use getBoard).
  // Hidden-info pass-and-play: the current mover. Hidden-info vs an AI: the lone
  // human's fixed seat, so the human never sees the AI's secret even on the AI's
  // turn. Watch-AI (no humans): the current mover, for the demo.
  const viewSeatOf = useCallback(
    (h: GameHandle): number | null => {
      if (!def.hiddenInfo) return null;
      const humans = seatsRef.current.map((s, i) => (s === 'human' ? i : -1)).filter((i) => i >= 0);
      if (humans.length === 0) return h.currentPlayer();
      if (humans.length === seatsRef.current.length) return h.currentPlayer();
      return humans[0];
    },
    [def],
  );
  const readBoard = useCallback(
    (h: GameHandle): string => {
      const vs = viewSeatOf(h);
      return vs != null && h.getBoardFor ? h.getBoardFor(vs) : h.getBoard();
    },
    [viewSeatOf],
  );

  const syncBoard = useCallback(
    (h: GameHandle) => {
      setBoard(readBoard(h));
      setCurrent(h.currentPlayer());
      setStatusText(h.statusText?.() ?? '');
      setLegalMoves(h.isTerminal() ? [] : h.legalMoves());
      setCanUndo(movesRef.current.some((e) => seatsRef.current[e.seat] === 'human'));
    },
    [readBoard],
  );

  // Pass-and-play hidden-info handoff: after a move hands control to a DIFFERENT
  // human seat, raise the blackout so the next player picks up the phone without
  // seeing the prior view. `prevMover === null` covers the game's first turn.
  const maybeHandoff = useCallback(
    (prevMover: number | null, h: GameHandle) => {
      if (!def.hiddenInfo || h.isTerminal()) return;
      const allHuman = seatsRef.current.every((s) => s === 'human');
      if (!allHuman) return;
      const next = h.currentPlayer();
      if (prevMover === null || next !== prevMover) setHandoff(next);
    },
    [def],
  );

  const playMoveSound = useCallback(() => {
    if (def.moveSound === 'drop') sound.drop();
    else sound.move();
  }, [def]);

  const finish = useCallback(
    (h: GameHandle) => {
      const result = h.result();
      setResult(result);
      setEndText(h.endText?.() ?? '');
      setWinCells((h.winningCells?.() ?? []).map(Number).filter((n) => Number.isFinite(n)));
      setPhase('over');
      if (def.solo) {
        if ((h.endText?.() ?? '').startsWith('🎉')) sound.win();
        else sound.draw();
      } else if (result === 'Draw' || result === '') {
        sound.draw();
      } else {
        // Lose tone only if an AI seat won (a human at the table didn't).
        const winnerSeat = Number(result) - 1;
        if (seatsRef.current[winnerSeat] !== 'human') sound.lose();
        else sound.win();
      }
    },
    [def],
  );

  const runAiTurn = useCallback(() => {
    setPhase('thinking');
    const gen = genRef.current;
    timerRef.current = setTimeout(() => {
      if (gen !== genRef.current) return; // a newer game started; abort this turn
      const h = handleRef.current;
      if (!h || h.isTerminal()) return;
      const kind = seatsRef.current[h.currentPlayer()];
      if (kind === 'human') {
        setPhase('playing');
        return;
      }
      // Solo "watch" uses a fixed budget; multiplayer uses this seat's strength.
      const mv = def.solo ? (h.playoutN(SOLO_AI_PLAYOUTS), h.bestMove()) : pickAiMove(h, aiConfig(def, kind));
      if (mv != null) {
        const seat = h.currentPlayer();
        const before = h.getBoard();
        if (h.applyMove(mv)) {
          movesRef.current.push({ move: mv, seat });
          setLastCells(diffCells(before, h.getBoard()));
        }
        playMoveSound();
      }
      const terminal = h.isTerminal();
      syncBoard(h);
      if (terminal) {
        finish(h);
        return;
      }
      if (seatsRef.current[h.currentPlayer()] !== 'human') {
        runAiTurn();
      } else {
        setPhase('playing');
      }
    }, AI_DELAY_MS);
  }, [def, syncBoard, finish, playMoveSound]);

  const start = useCallback(() => {
    genRef.current++; // invalidate any AI turn still pending from a prior game
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (handleRef.current) handleRef.current.free();
    const h = def.create(wasm, params);
    handleRef.current = h;
    movesRef.current = [];
    setResult('');
    setEndText('');
    setWinCells([]);
    setLastCells([]);
    setHandoff(null);
    setPhase('playing');
    syncBoard(h);
    maybeHandoff(null, h);
    if (seatsRef.current[h.currentPlayer()] !== 'human') runAiTurn();
  }, [wasm, def, params, runAiTurn, syncBoard, maybeHandoff]);

  // Rewind to just before the last human move (also popping AI replies after
  // it) by replaying the move log on a fresh engine. Disabled for solo/chance
  // games (def.noUndo) where replay would reroll randomness.
  const undo = useCallback(() => {
    const log = movesRef.current;
    let cut = -1;
    for (let i = log.length - 1; i >= 0; i--) {
      if (seatsRef.current[log[i].seat] === 'human') {
        cut = i;
        break;
      }
    }
    if (cut < 0) return;
    genRef.current++; // cancel any AI turn in flight
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (handleRef.current) handleRef.current.free();
    const h = def.create(wasm, params);
    handleRef.current = h;
    const kept = log.slice(0, cut);
    for (const e of kept) h.applyMove(e.move);
    movesRef.current = kept;
    setResult('');
    setEndText('');
    setWinCells([]);
    setLastCells([]);
    setPhase('playing');
    syncBoard(h);
    // A human is to move by construction (we cut at a human's move).
  }, [wasm, def, params, syncBoard]);

  useEffect(() => {
    start();
    return () => {
      genRef.current++; // stop any pending AI turn from touching a freed handle
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
      if (handleRef.current) {
        handleRef.current.free();
        handleRef.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onHumanMove = useCallback(
    (move: string) => {
      const h = handleRef.current;
      if (!h || phase !== 'playing') return;
      if (seatsRef.current[h.currentPlayer()] !== 'human') return;
      const seat = h.currentPlayer();
      const before = h.getBoard();
      if (!h.applyMove(move)) return;
      movesRef.current.push({ move, seat });
      setLastCells(diffCells(before, h.getBoard()));
      playMoveSound();
      if (h.isTerminal()) {
        syncBoard(h);
        finish(h);
        return;
      }
      syncBoard(h);
      maybeHandoff(seat, h);
      if (seatsRef.current[h.currentPlayer()] !== 'human') runAiTurn();
    },
    [phase, runAiTurn, syncBoard, finish, playMoveSound, maybeHandoff],
  );

  const getHint = useCallback((): string | undefined => {
    const h = handleRef.current;
    if (!h) return undefined;
    h.playoutN(HINT_PLAYOUTS);
    return h.bestMove();
  }, []);

  const dismissHandoff = useCallback(() => setHandoff(null), []);

  return { board, current, phase, result, seats, statusText, endText, legalMoves, winCells, lastCells, onHumanMove, getHint, replay: start, undo, canUndo, handoff, dismissHandoff };
}
