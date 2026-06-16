import type { Difficulty, Mode } from './gameTypes';

export interface DeepLink {
  gameId?: string;
  mode?: Mode;
  difficulty?: Difficulty;
}

const MODES = ['pvp', 'pvai', 'aivai', 'solo'];
const DIFFS = ['easy', 'medium', 'hard'];

/** Read `?game=&mode=&difficulty=` from the current URL (client-only). */
export function parseArcadeParams(): DeepLink {
  if (typeof window === 'undefined') return {};
  const p = new URLSearchParams(window.location.search);
  const mode = p.get('mode');
  const difficulty = p.get('difficulty');
  return {
    gameId: p.get('game') ?? undefined,
    mode: mode && MODES.includes(mode) ? (mode as Mode) : undefined,
    difficulty: difficulty && DIFFS.includes(difficulty) ? (difficulty as Difficulty) : undefined,
  };
}

/** Build a shareable deep link to a game's setup. */
export function buildShareUrl(
  gameId: string,
  mode: Mode,
  difficulty: Difficulty,
  includeDifficulty: boolean,
): string {
  if (typeof window === 'undefined') return '';
  const base = window.location.origin + window.location.pathname;
  const params = new URLSearchParams();
  params.set('game', gameId);
  params.set('mode', mode);
  if (includeDifficulty) params.set('difficulty', difficulty);
  return `${base}?${params.toString()}`;
}
