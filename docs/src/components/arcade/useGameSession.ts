import { useCallback, useEffect, useRef, useState } from 'react';
import type { Difficulty, GameDefinition, GameHandle, GameParams, Mode } from './gameTypes';
import { pickAiMove, seatTypes } from './gameTypes';

type Phase = 'playing' | 'thinking' | 'over';
const AI_DELAY_MS = 400;

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

  const runAiTurn = useCallback(() => {
    setPhase('thinking');
    setTimeout(() => {
      const h = handleRef.current;
      if (!h || h.isTerminal()) return;
      const mv = pickAiMove(h, def, params, difficulty);
      if (mv != null) h.applyMove(mv);
      const terminal = h.isTerminal();
      setBoard(h.getBoard());
      setCurrent(h.currentPlayer());
      if (terminal) {
        setResult(h.result());
        setPhase('over');
        return;
      }
      if (seatsRef.current[h.currentPlayer()] === 'ai') {
        runAiTurn();
      } else {
        setPhase('playing');
      }
    }, AI_DELAY_MS);
  }, [def, params, difficulty]);

  const start = useCallback(() => {
    if (handleRef.current) handleRef.current.free();
    const h = def.create(wasm, params);
    handleRef.current = h;
    setResult('');
    setPhase('playing');
    setBoard(h.getBoard());
    setCurrent(h.currentPlayer());
    if (seatsRef.current[h.currentPlayer()] === 'ai') runAiTurn();
  }, [wasm, def, params, runAiTurn]);

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
      if (h.isTerminal()) {
        setBoard(h.getBoard());
        setResult(h.result());
        setPhase('over');
        return;
      }
      setBoard(h.getBoard());
      setCurrent(h.currentPlayer());
      if (seatsRef.current[h.currentPlayer()] === 'ai') runAiTurn();
    },
    [phase, runAiTurn],
  );

  return { board, current, phase, result, seats, onHumanMove, replay: start };
}
