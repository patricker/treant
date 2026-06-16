import { connectFour } from './connectFour';
import { ticTacToe } from './ticTacToe';
import { nim } from './nim';
import { mancala } from './mancala';
import { shift } from './shift';
import { game2048 } from './game2048';
import { orderChaos, noTacToe, trapThree, squareUp, connectSix } from './gridpack';
import { pig } from './pig';
import { frontline } from './frontline';
import { hex } from './hex';
import { clobber } from './clobber';
import { reversi } from './reversi';
import { trails } from './trails';
import { captureGo } from './capturego';
import { chomp } from './chomp';
import { wythoff } from './wythoff';
import { dotsBoxes } from './dotsboxes';
import { gomoku } from './gomoku';
import { muTorere } from './mutorere';
import { domineering } from './domineering';
import { konane } from './konane';
import { nogo } from './nogo';
import { amazons } from './amazons';
import { col } from './col';
import { sim } from './sim';
import type { GameDefinition } from '../gameTypes';

export const GAMES: GameDefinition[] = [
  connectFour,
  ticTacToe,
  shift,
  frontline,
  clobber,
  trails,
  captureGo,
  konane,
  amazons,
  reversi,
  hex,
  orderChaos,
  noTacToe,
  trapThree,
  squareUp,
  connectSix,
  gomoku,
  dotsBoxes,
  nim,
  chomp,
  wythoff,
  nogo,
  col,
  sim,
  mancala,
  muTorere,
  domineering,
  pig,
  game2048,
];

export function gameById(id: string): GameDefinition | undefined {
  return GAMES.find((g) => g.id === id);
}
