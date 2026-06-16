import { connectFour } from './connectFour';
import { ticTacToe } from './ticTacToe';
import { nim } from './nim';
import { mancala } from './mancala';
import { shift } from './shift';
import { game2048 } from './game2048';
import { orderChaos, noTacToe, trapThree, squareUp, connectSix } from './gridpack';
import type { GameDefinition } from '../gameTypes';

export const GAMES: GameDefinition[] = [
  connectFour,
  ticTacToe,
  shift,
  orderChaos,
  noTacToe,
  trapThree,
  squareUp,
  connectSix,
  nim,
  mancala,
  game2048,
];

export function gameById(id: string): GameDefinition | undefined {
  return GAMES.find((g) => g.id === id);
}
