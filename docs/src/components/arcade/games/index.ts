import { connectFour, popOut, cylinderFour } from './connectFour';
import { ticTacToe } from './ticTacToe';
import { nim } from './nim';
import { mancala } from './mancala';
import { oware } from './oware';
import { shift } from './shift';
import { quadline } from './quadline';
import { game2048 } from './game2048';
import { orderChaos, noTacToe, trapThree, squareUp, connectSix } from './gridpack';
import { pig, twoDicePig, bigPig } from './pig';
import { climb } from './climb';
import { pinchFive } from './pinchfive';
import { frontline } from './frontline';
import { pawnDuel } from './pawnduel';
import { hex } from './hex';
import { ygame } from './ygame';
import { gale } from './gale';
import { clobber } from './clobber';
import { reversi, antiReversi } from './reversi';
import { trails, joust } from './trails';
import { strand } from './strand';
import { slimetrail } from './slimetrail';
import { captureGo } from './capturego';
import { chomp } from './chomp';
import { wythoff } from './wythoff';
import { dotsBoxes } from './dotsboxes';
import { gomoku } from './gomoku';
import { muTorere } from './mutorere';
import { domineering, cram } from './domineering';
import { konane } from './konane';
import { nineMorris, laskerMorris } from './ninemorris';
import { nogo } from './nogo';
import { amazons } from './amazons';
import { col, snort } from './col';
import { sim } from './sim';
import { foxHounds } from './foxhounds';
import { treblecross } from './treblecross';
import { toadsFrogs } from './toadsfrogs';
import { subtractSquare } from './subtractsquare';
import { euclid } from './euclid';
import { baghchal } from './baghchal';
import { lenChoa } from './lenchoa';
import { worldThrees } from './worldthrees';
import { bullsCows } from './bullscows';
import {
  draughts,
  internationalDraughts,
  brazilianDraughts,
  poolCheckers,
  russianDraughts,
  giveawayCheckers,
} from './draughts';
import type { GameDefinition } from '../gameTypes';

export const GAMES: GameDefinition[] = [
  connectFour,
  popOut,
  cylinderFour,
  ticTacToe,
  shift,
  quadline,
  frontline,
  pawnDuel,
  clobber,
  trails,
  joust,
  strand,
  slimetrail,
  captureGo,
  konane,
  nineMorris,
  laskerMorris,
  amazons,
  foxHounds,
  baghchal,
  lenChoa,
  reversi,
  antiReversi,
  hex,
  ygame,
  gale,
  orderChaos,
  noTacToe,
  trapThree,
  squareUp,
  connectSix,
  pinchFive,
  gomoku,
  treblecross,
  toadsFrogs,
  dotsBoxes,
  nim,
  chomp,
  wythoff,
  subtractSquare,
  euclid,
  nogo,
  col,
  snort,
  sim,
  mancala,
  oware,
  muTorere,
  worldThrees,
  draughts,
  internationalDraughts,
  brazilianDraughts,
  poolCheckers,
  russianDraughts,
  giveawayCheckers,
  domineering,
  cram,
  pig,
  twoDicePig,
  bigPig,
  climb,
  bullsCows,
  game2048,
];

export function gameById(id: string): GameDefinition | undefined {
  return GAMES.find((g) => g.id === id);
}
