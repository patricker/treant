import { useCallback, useEffect, useRef, useState } from 'react';
import type { GameDefinition, GameHandle, GameParams, PlayerKind } from './gameTypes';
import { aiConfig, pickAiMove } from './gameTypes';
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
  const [phase, setPhase] = useState<Phase>('playing');
  const [result, setResult] = useState('');
  const [statusText, setStatusText] = useState('');
  const [endText, setEndText] = useState('');
  const [legalMoves, setLegalMoves] = useState<string[]>([]);
  const [canUndo, setCanUndo] = useState(false);
  // Move log for undo: seat captured BEFORE the move applies. Undo replays the
  // prefix ending just before the last human move (popping any AI replies too).
  const movesRef = useRef<{ move: string; seat: number }[]>([]);

  const syncBoard = useCallback((h: GameHandle) => {
    setBoard(h.getBoard());
    setCurrent(h.currentPlayer());
    setStatusText(h.statusText?.() ?? '');
    setLegalMoves(h.isTerminal() ? [] : h.legalMoves());
    setCanUndo(movesRef.current.some((e) => seatsRef.current[e.seat] === 'human'));
  }, []);

  const playMoveSound = useCallback(() => {
    if (def.moveSound === 'drop') sound.drop();
    else sound.move();
  }, [def]);

  const finish = useCallback(
    (h: GameHandle) => {
      const result = h.result();
      setResult(result);
      setEndText(h.endText?.() ?? '');
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
        if (h.applyMove(mv)) movesRef.current.push({ move: mv, seat });
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
    setPhase('playing');
    syncBoard(h);
    if (seatsRef.current[h.currentPlayer()] !== 'human') runAiTurn();
  }, [wasm, def, params, runAiTurn, syncBoard]);

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
      if (!h.applyMove(move)) return;
      movesRef.current.push({ move, seat });
      playMoveSound();
      if (h.isTerminal()) {
        syncBoard(h);
        finish(h);
        return;
      }
      syncBoard(h);
      if (seatsRef.current[h.currentPlayer()] !== 'human') runAiTurn();
    },
    [phase, runAiTurn, syncBoard, finish, playMoveSound],
  );

  const getHint = useCallback((): string | undefined => {
    const h = handleRef.current;
    if (!h) return undefined;
    h.playoutN(HINT_PLAYOUTS);
    return h.bestMove();
  }, []);

  return { board, current, phase, result, seats, statusText, endText, legalMoves, onHumanMove, getHint, replay: start, undo, canUndo };
}
