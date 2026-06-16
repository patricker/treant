import { connectFour } from './connectFour';
import { ticTacToe } from './ticTacToe';
import type { GameDefinition } from '../gameTypes';

export const GAMES: GameDefinition[] = [connectFour, ticTacToe];

export function gameById(id: string): GameDefinition | undefined {
  return GAMES.find((g) => g.id === id);
}
