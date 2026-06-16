// How-to-play text for every arcade game, keyed by game id. The rules panel
// prefers a game's own `rules` field, then this map, then the short blurb.
export const RULES: Record<string, string> = {
  'connect-four': 'Take turns dropping a disc into a column; it falls to the lowest empty slot. First to line up four of your colour in a row — across, down, or diagonally — wins.',
  'tic-tac-toe': 'Take turns marking an empty square. First to get your mark three-in-a-row (across, down, or diagonally) wins. With the knobs you can grow the board and the line length.',
  shift: 'Phase one: take turns placing your pieces. Once everyone has placed all their pieces, phase two begins — slide one of your pieces into an empty neighbouring cell each turn. Make a line to win.',
  frontline: 'Move a pawn one square straight forward into an empty cell, or one square diagonally forward to capture an enemy. Reach the far row, or capture every enemy pawn, to win. No draws.',
  clobber: 'The board starts as a full checkerboard of two colours. On your turn, move one of your pieces onto an orthogonally-adjacent enemy piece, removing it. If you cannot move, you lose.',
  konane: 'A jump-and-capture game. Hop one of your stones over an adjacent enemy stone into the empty cell beyond, removing the stone you jumped. The first player who cannot jump loses.',
  trails: 'Move your token one step to an open neighbour. The cell you leave becomes a permanent wall. Box your opponent in — the last player able to move wins. (Like light-cycles.)',
  'first-capture': 'Place a stone on any empty point. A group of stones with no empty neighbours (no "liberties") is captured. The first player to capture any enemy stone wins.',
  reversi: 'Place a disc so it flanks a straight line of enemy discs between your new disc and another of yours — all flanked discs flip to your colour. Most discs when the board fills wins.',
  hex: 'Place one stone on any empty hexagon each turn. Red wins by linking the top and bottom edges with a connected chain; Gold links left and right. Hex can never be a draw.',
  'order-chaos': 'Both players may place either an X or an O. "Order" wins by making five-in-a-row of a single symbol; "Chaos" wins if the board fills with no such line.',
  'no-tac-toe': 'Everyone plays the same mark (X). Completing any three-in-a-row LOSES — so force your opponent to make the line they’re trying to avoid.',
  'trap-three': 'Four-in-a-row WINS, but three-in-a-row LOSES — and both are live at once. Build toward four while carefully avoiding an accidental three.',
  'square-up': 'Take turns placing your stones. You win the moment four of your stones form the corners of a square — any size, and even tilted on a diagonal.',
  'connect-six': 'After the first move, place TWO stones every turn. The first player to make six (or more) of their colour in an unbroken row, column, or diagonal wins.',
  gomoku: 'Take turns placing a stone on any empty point. The first player to get exactly five of their colour in an unbroken row — across, down, or diagonally — wins.',
  treblecross: 'A single strip, and BOTH players place the same mark (X). Whoever completes three X’s in a row wins — even using marks the other player placed.',
  'dots-and-boxes': 'Take turns drawing one edge between two dots. Complete the fourth side of a 1×1 box to claim it and immediately take another turn. Most boxes when the grid is full wins.',
  nim: 'Stones sit in a pile. On your turn take 1 or 2 stones. The player who takes the very last stone wins. (There’s a hidden winning strategy — can you find it?)',
  chomp: 'The grid is a chocolate bar; the top-left square is poisoned. On your turn eat a square and everything below-and-right of it. Whoever is forced to eat the poison loses.',
  wythoff: 'A queen sits on a grid and only ever moves toward the corner: any distance left, down, or diagonally down-left. The player who moves it onto the corner square wins.',
  'subtract-square': 'A pile of stones. On your turn remove a perfect-square number of them — 1, 4, 9, 16, and so on. Take the last stone to win.',
  euclid: 'Two numbers. On your turn, subtract any positive multiple of the smaller from the larger (the result must stay ≥ 0). The player who makes one of the numbers zero wins.',
  nogo: 'The opposite of capture-Go: place a stone, but you may NEVER capture an enemy group or leave your own group without a liberty. The first player with no safe move loses.',
  col: 'Colour any empty cell in your colour — but never one orthogonally next to a cell you have already coloured (keep your patches apart). The first player who cannot colour a cell loses.',
  sim: 'Six dots, fifteen connecting lines. Take turns colouring a line your colour. Completing a triangle of three dots all in your own colour LOSES — and someone always must.',
  mancala: 'Pick up all the seeds from one of your pits and sow them one-by-one into following pits. Land your last seed just right to capture. The most seeds in your store wins.',
  'mu-torere': 'Slide one of your pieces into the single empty point on the eight-pointed star. A piece may only move to the centre if it sits next to an enemy. Trap your opponent so they cannot move.',
  domineering: 'You and your opponent place dominoes on the grid, but you place yours vertically and they place theirs horizontally. The first player who cannot fit a domino loses.',
  amazons: 'Move one of your amazons like a chess queen (any distance in a straight line), then from its new square shoot an arrow the same way. The arrow burns its square forever. Trap your opponent to win.',
  'fox-hounds': 'You are the lone Fox and move one square diagonally in any direction; the four Hounds move one square diagonally forward only. The Fox wins by slipping past to the far row; the Hounds win by trapping it.',
  pig: 'Roll the die to build up a running total, then choose to bank it — but roll a 1 and you lose the whole turn’s points. First to reach the target score wins. Press your luck!',
  '2048': 'Swipe to slide all tiles one way; equal tiles that bump merge into one of double the value. A new tile appears after each move. Make a 2048 tile to win — but don’t fill the board.',
};

export function gameRules(id: string, ownRules: string | undefined, blurb: string): string {
  return ownRules ?? RULES[id] ?? blurb;
}
