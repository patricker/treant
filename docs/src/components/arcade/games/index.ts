import { connectFour } from './connectFour';
import { ticTacToe } from './ticTacToe';
import { nim } from './nim';
import { mancala } from './mancala';
import { oware } from './oware';
import { shift } from './shift';
import { quadline } from './quadline';
import { game2048 } from './game2048';
import { orderChaos, noTacToe, trapThree, squareUp, connectSix } from './gridpack';
import { pig } from './pig';
import { pinchFive } from './pinchfive';
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
import { nineMorris } from './ninemorris';
import { nogo } from './nogo';
import { amazons } from './amazons';
import { col } from './col';
import { sim } from './sim';
import { foxHounds } from './foxhounds';
import { treblecross } from './treblecross';
import { subtractSquare } from './subtractsquare';
import { euclid } from './euclid';
import { baghchal } from './baghchal';
import type { GameDefinition } from '../gameTypes';

export const GAMES: GameDefinition[] = [
  connectFour,
  ticTacToe,
  shift,
  quadline,
  frontline,
  clobber,
  trails,
  captureGo,
  konane,
  nineMorris,
  amazons,
  foxHounds,
  baghchal,
  reversi,
  hex,
  orderChaos,
  noTacToe,
  trapThree,
  squareUp,
  connectSix,
  pinchFive,
  gomoku,
  treblecross,
  dotsBoxes,
  nim,
  chomp,
  wythoff,
  subtractSquare,
  euclid,
  nogo,
  col,
  sim,
  mancala,
  oware,
  muTorere,
  domineering,
  pig,
  game2048,
];

export function gameById(id: string): GameDefinition | undefined {
  return GAMES.find((g) => g.id === id);
}
