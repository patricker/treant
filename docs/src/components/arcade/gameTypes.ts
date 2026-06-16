import type { ComponentType } from 'react';

/** `numPlayers` is required (the seat/mode system needs it); every other knob
 *  (cols, rows, k, pits, stones, pieces, …) is a game-specific numeric entry. */
export type GameParams = { numPlayers: number } & Record<string, number>;
export type Mode = 'pvp' | 'pvai' | 'aivai' | 'solo';
export type Difficulty = 'easy' | 'medium' | 'hard';
export type SeatType = 'human' | 'ai';
/** What occupies one seat: a human, or an AI at a chosen strength. */
export type PlayerKind = 'human' | Difficulty;

/** Default per-seat display names/colours (seats 0..5). */
export const PLAYER_LABEL = ['Red', 'Yellow', 'Green', 'Purple', 'Teal', 'Orange'];

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
  /** the current player's legal moves (for movement games to highlight targets) */
  legalMoves: string[];
  onMove: (move: string) => void;
}

export interface GameDefinition {
  id: string;
  name: string;
  icon: string;
  blurb: string;
  /** Full how-to-play text shown in the rules panel. Falls back to blurb. */
  rules?: string;
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

/** Build a per-seat list from a quick mode + a default AI strength. */
export function seatsForMode(mode: Mode, numPlayers: number, ai: Difficulty): PlayerKind[] {
  if (mode === 'pvp' || mode === 'solo') return Array(numPlayers).fill('human');
  if (mode === 'aivai') return Array(numPlayers).fill(ai);
  // pvai: seat 0 is human, the rest are AI at strength `ai`
  return ['human', ...Array(Math.max(0, numPlayers - 1)).fill(ai)] as PlayerKind[];
}

// Compact URL encoding for a seat list: one char per seat.
const SEAT_CODE: Record<PlayerKind, string> = { human: 'h', easy: 'e', medium: 'm', hard: 'd' };
const CODE_SEAT: Record<string, PlayerKind> = { h: 'human', e: 'easy', m: 'medium', d: 'hard' };

export function encodeSeats(seats: PlayerKind[]): string {
  return seats.map((s) => SEAT_CODE[s]).join('');
}

/** Decode a seat code, padding/trimming to `numPlayers`. */
export function decodeSeats(code: string, numPlayers: number): PlayerKind[] {
  const out = [...code].map((c) => CODE_SEAT[c]).filter(Boolean) as PlayerKind[];
  if (out.length === 0) return seatsForMode('pvai', numPlayers, 'medium');
  while (out.length < numPlayers) out.push(out[out.length - 1]);
  return out.slice(0, numPlayers);
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
