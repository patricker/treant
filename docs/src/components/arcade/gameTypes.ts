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
  /** Canonical result contract, uniform across every engine:
   *  `""` while the game is in progress, `"Draw"` for a draw, or a bare
   *  1-indexed winner-seat digit (`"1"`..`"6"`). The session parses it as
   *  `Number(result) - 1` to get the winner seat, so any other shape (e.g. a
   *  `"P1"` prefix) yields a NaN seat. Emit exactly this from Rust `result()`. */
  result(): string;
  bestMove(): string | undefined;
  /** Cell indices of the winning line, when the game ended on a straight line
   *  of K (line games only). `[]` for a draw, a non-line win, or in progress.
   *  Absent on engines that don't report a line — the board simply skips the glow. */
  winningCells?(): string[];
  playoutN(n: number): void;
  /** The current player's legal move strings (random-move fallback when playouts===0, and movement-game target highlighting). */
  legalMoves(): string[];
  /** Difficulty-aware move (value-aware top-K visit temperature). Optional;
   *  handles without it fall back in pickAiMove. Parameter domains:
   *  `playouts===0` returns undefined (pickAiMove then plays a random move);
   *  `topK` is clamped to [1, #legal moves] (0 → greedy);
   *  `temp <= ~1e-4` (or any non-positive temp) means greedy (the engine's best
   *  move); larger temp flattens the choice over the top-K.
   *  `seed` only makes the final softmax tie-break draw reproducible — the
   *  underlying MCTS search RNG is unseeded, so repeated calls with the same
   *  seed can still return different moves. */
  weakMove?(playouts: number, topK: number, temp: number, seed: number): string | undefined;
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
  /** cell indices of the winning line to glow (line games; empty otherwise) */
  winCells?: number[];
  onMove: (move: string) => void;
}

export interface GameDefinition {
  id: string;
  name: string;
  icon: string;
  blurb: string;
  /** Full how-to-play text shown in the rules panel. Falls back to blurb. */
  rules?: string;
  /** Per-game calibrated AI knobs per level (else DEFAULT_DIFFICULTY). */
  difficulty?: Record<Difficulty, AiConfig>;
  defaultParams: GameParams;
  presets: Preset[];
  knobs: Knob[];
  create(wasm: any, p: GameParams): GameHandle;
  Board: ComponentType<BoardProps>;
  /** Single-player game: uses the solo setup/flow (You-play + Hint / Watch-AI). */
  solo?: boolean;
  /**
   * Disable the undo button. Set on chance games (dice rolls, random spawns):
   * undo replays the move log on a fresh engine, which would reroll the
   * randomness and rewrite history.
   */
  noUndo?: boolean;
  /** Solo games: prettify a hint move for display, e.g. "Up" -> "⬆️ Up". */
  formatHint?(move: string): string;
  /** Sound to play per move (default "move"; Connect Four uses "drop"). */
  moveSound?: 'move' | 'drop';
  /** Display names per seat (default Red/Yellow/Green/Purple). */
  playerLabels?: string[];
}

/** Concrete AI knobs for one difficulty level. */
export interface AiConfig {
  playouts: number;
  topK: number;
  temp: number;
}

/** Level metadata for the UI (labels/emoji). */
export const DIFFICULTY: Record<Difficulty, { label: string; emoji: string }> = {
  easy: { label: 'Easy', emoji: '😊' },
  medium: { label: 'Medium', emoji: '😎' },
  hard: { label: 'Hard', emoji: '🔥' },
};

/** Fallback ladder for games without a calibrated override. Placed LOW on the
 *  simulation curve and weakened with top-K visit temperature. */
export const DEFAULT_DIFFICULTY: Record<Difficulty, AiConfig> = {
  easy: { playouts: 40, topK: 6, temp: 1.6 },
  medium: { playouts: 500, topK: 3, temp: 0.5 },
  hard: { playouts: 5000, topK: 1, temp: 0.0 },
};

/** Resolve the AiConfig for a game + level (per-game override wins). */
export function aiConfig(def: GameDefinition, d: Difficulty): AiConfig {
  return def.difficulty?.[d] ?? DEFAULT_DIFFICULTY[d];
}

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
 * Choose an AI move under an AiConfig. Uses the engine's value-aware weakening
 * (weakMove) when available; else a graceful fallback.
 */
export function pickAiMove(
  handle: GameHandle,
  cfg: AiConfig,
  rng: () => number = Math.random,
): string | undefined {
  const legal = handle.legalMoves();
  if (legal.length === 0) return undefined;
  if (cfg.playouts === 0 || !handle.weakMove) {
    return legal[Math.floor(rng() * legal.length)];
  }
  const seed = Math.floor(rng() * 0xffffffff) >>> 0;
  // weakMove only returns null in degenerate cases; fall back to a full search.
  return handle.weakMove(cfg.playouts, cfg.topK, cfg.temp, seed) ?? (handle.playoutN(cfg.playouts), handle.bestMove() ?? legal[0]);
}
