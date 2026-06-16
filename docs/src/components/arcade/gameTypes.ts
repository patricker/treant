import type { ComponentType } from 'react';

/** `numPlayers` is required (the seat/mode system needs it); every other knob
 *  (cols, rows, k, pits, stones, pieces, …) is a game-specific numeric entry. */
export type GameParams = { numPlayers: number } & Record<string, number>;
export type Mode = 'pvp' | 'pvai' | 'aivai' | 'solo';
export type Difficulty = 'easy' | 'medium' | 'hard';
export type SeatType = 'human' | 'ai';

export interface Preset {
  label: string;
  emoji?: string;
  params: GameParams;
}
export interface Knob {
  key: string;
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
  /** The current player's legal move strings (used for epsilon-random AI). */
  legalMoves(): string[];
  free(): void;
  /** Solo games: a live status line, e.g. "Score 1234 · Best 128". */
  statusText?(): string;
  /** Solo games: the game-over message, e.g. "Game over — score 1234". */
  endText?(): string;
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
  Board: ComponentType<BoardProps>;
  /** Single-player game: uses the solo setup/flow (You-play + Hint / Watch-AI). */
  solo?: boolean;
  /** Solo games: prettify a hint move for display, e.g. "Up" -> "⬆️ Up". */
  formatHint?(move: string): string;
  /** Sound to play per move (default "move"; Connect Four uses "drop"). */
  moveSound?: 'move' | 'drop';
  /** Display names per seat (default Red/Yellow/Green/Purple). */
  playerLabels?: string[];
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
  if (mode === 'solo') return Array(numPlayers).fill('human');
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
  diff: Difficulty,
  rng: () => number = Math.random,
): string | undefined {
  const { playouts, epsilon } = DIFFICULTY[diff];
  const legal = handle.legalMoves();
  if (legal.length === 0) return undefined;
  if (rng() < epsilon) return legal[Math.floor(rng() * legal.length)];
  handle.playoutN(playouts);
  return handle.bestMove() ?? legal[0];
}
