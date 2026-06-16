import { useCallback, useEffect, useRef, useState } from 'react';
import type { Difficulty, GameDefinition, GameHandle, GameParams, Mode } from './gameTypes';
import { pickAiMove, seatTypes } from './gameTypes';
import { sound } from './sound';

type Phase = 'playing' | 'thinking' | 'over';
const AI_DELAY_MS = 400;
const SOLO_AI_PLAYOUTS = 800;
const HINT_PLAYOUTS = 1500;

export function useGameSession(
  wasm: any,
  def: GameDefinition,
  params: GameParams,
  mode: Mode,
  difficulty: Difficulty,
) {
  const handleRef = useRef<GameHandle | null>(null);
  const seats = seatTypes(mode, params.numPlayers);
  const seatsRef = useRef(seats);
  seatsRef.current = seats;

  const [board, setBoard] = useState('');
  const [current, setCurrent] = useState(0);
  const [phase, setPhase] = useState<Phase>('playing');
  const [result, setResult] = useState('');
  const [statusText, setStatusText] = useState('');
  const [endText, setEndText] = useState('');

  // Push board + solo status into state from the live handle.
  const syncBoard = useCallback((h: GameHandle) => {
    setBoard(h.getBoard());
    setCurrent(h.currentPlayer());
    setStatusText(h.statusText?.() ?? '');
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
      // End sound. Solo: cheer only on a milestone (endText starts with 🎉),
      // else a gentle neutral tone. Multiplayer: lose if an AI seat won.
      if (def.solo) {
        if ((h.endText?.() ?? '').startsWith('🎉')) sound.win();
        else sound.draw();
      } else if (result === 'Draw' || result === '') {
        sound.draw();
      } else {
        const winnerSeat = Number(result) - 1;
        if (seatsRef.current[winnerSeat] === 'ai') sound.lose();
        else sound.win();
      }
    },
    [def],
  );

  const runAiTurn = useCallback(() => {
    setPhase('thinking');
    setTimeout(() => {
      const h = handleRef.current;
      if (!h || h.isTerminal()) return;
      const mv = def.solo ? (h.playoutN(SOLO_AI_PLAYOUTS), h.bestMove()) : pickAiMove(h, difficulty);
      if (mv != null) {
        h.applyMove(mv);
        playMoveSound();
      }
      const terminal = h.isTerminal();
      syncBoard(h);
      if (terminal) {
        finish(h);
        return;
      }
      if (seatsRef.current[h.currentPlayer()] === 'ai') {
        runAiTurn();
      } else {
        setPhase('playing');
      }
    }, AI_DELAY_MS);
  }, [def, difficulty, syncBoard, finish, playMoveSound]);

  const start = useCallback(() => {
    if (handleRef.current) handleRef.current.free();
    const h = def.create(wasm, params);
    handleRef.current = h;
    setResult('');
    setEndText('');
    setPhase('playing');
    syncBoard(h);
    if (seatsRef.current[h.currentPlayer()] === 'ai') runAiTurn();
  }, [wasm, def, params, runAiTurn, syncBoard]);

  useEffect(() => {
    start();
    return () => {
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
      if (!h.applyMove(move)) return;
      playMoveSound();
      if (h.isTerminal()) {
        syncBoard(h);
        finish(h);
        return;
      }
      syncBoard(h);
      if (seatsRef.current[h.currentPlayer()] === 'ai') runAiTurn();
    },
    [phase, runAiTurn, syncBoard, finish, playMoveSound],
  );

  const getHint = useCallback((): string | undefined => {
    const h = handleRef.current;
    if (!h) return undefined;
    h.playoutN(HINT_PLAYOUTS);
    return h.bestMove();
  }, []);

  return { board, current, phase, result, seats, statusText, endText, onHumanMove, getHint, replay: start };
}
