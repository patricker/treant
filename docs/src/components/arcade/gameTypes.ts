import type { JSX } from 'react';

export type GameParams = { cols: number; rows: number; k: number; numPlayers: number };
export type Mode = 'pvp' | 'pvai' | 'aivai';
export type Difficulty = 'easy' | 'medium' | 'hard';
export type SeatType = 'human' | 'ai';

export interface Preset {
  label: string;
  emoji?: string;
  params: GameParams;
}
export interface Knob {
  key: keyof GameParams;
  label: string;
  min: number;
  max: number;
  step: number;
}

export interface GameHandle {
  applyMove(move: string): boolean;
  getBoard(): string;
  currentPlayer(): number;
  isTerminal(): boolean;
  result(): string;
  bestMove(): string | undefined;
  playoutN(n: number): void;
  free(): void;
}

export interface BoardProps {
  board: string;
  params: GameParams;
  currentPlayer: number;
  /** true only when it is a human seat's turn */
  interactive: boolean;
  onMove: (move: string) => void;
}

export interface GameDefinition {
  id: string;
  name: string;
  icon: string;
  blurb: string;
  defaultParams: GameParams;
  presets: Preset[];
  knobs: Knob[];
  create(wasm: any, p: GameParams): GameHandle;
  legalMoves(board: string, p: GameParams): string[];
  renderBoard(props: BoardProps): JSX.Element;
}

export const DIFFICULTY: Record<
  Difficulty,
  { playouts: number; epsilon: number; label: string; emoji: string }
> = {
  easy: { playouts: 200, epsilon: 0.5, label: 'Easy', emoji: '😊' },
  medium: { playouts: 2000, epsilon: 0.1, label: 'Medium', emoji: '😎' },
  hard: { playouts: 10000, epsilon: 0.0, label: 'Hard', emoji: '🔥' },
};

/** Assign a human/ai type to each seat based on the chosen mode. */
export function seatTypes(mode: Mode, numPlayers: number): SeatType[] {
  if (mode === 'pvp') return Array(numPlayers).fill('human');
  if (mode === 'aivai') return Array(numPlayers).fill('ai');
  // pvai: seat 0 is human, the rest are AI
  return ['human', ...Array(Math.max(0, numPlayers - 1)).fill('ai')] as SeatType[];
}

/**
 * Epsilon-greedy AI move. With probability `epsilon` plays a uniformly random
 * legal move (lets a kid beat "Easy"); otherwise searches `playouts` and plays
 * the best move. `rng` is injectable for deterministic tests.
 */
export function pickAiMove(
  handle: GameHandle,
  def: GameDefinition,
  params: GameParams,
  diff: Difficulty,
  rng: () => number = Math.random,
): string | undefined {
  const { playouts, epsilon } = DIFFICULTY[diff];
  const legal = def.legalMoves(handle.getBoard(), params);
  if (legal.length === 0) return undefined;
  if (rng() < epsilon) return legal[Math.floor(rng() * legal.length)];
  handle.playoutN(playouts);
  return handle.bestMove() ?? legal[0];
}
