import { connectFour } from './connectFour';
import { ticTacToe } from './ticTacToe';
import { nim } from './nim';
import { mancala } from './mancala';
import { shift } from './shift';
import type { GameDefinition } from '../gameTypes';

export const GAMES: GameDefinition[] = [connectFour, ticTacToe, nim, mancala, shift];

export function gameById(id: string): GameDefinition | undefined {
  return GAMES.find((g) => g.id === id);
}
