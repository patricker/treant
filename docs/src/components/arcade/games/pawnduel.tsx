import type { GameDefinition } from '../gameTypes';
import { moveHandle, makeMoveBoard } from './frontline';

// Pawn Duel — chess pawns only. Reuses Frontline's engine-driven select-then-move
// board wholesale (`moveHandle` + `makeMoveBoard('pawn')`): legality comes from
// the engine's `legal_moves()` "from-to" strings, so the en-passant capture shows
// up as an ordinary move target (a glowing empty square) with no special-casing,
// and executing it plays the "from-to" into the crossed square. We keep the
// param keys `cols`/`rows` (labelled Files/Ranks) precisely so the shared board
// renders without a wrapper — the divergence from the plan's `files`/`ranks`
// naming is purely to reuse Frontline's board contract unchanged.
export const pawnDuel: GameDefinition = {
  id: 'pawn-duel',
  name: 'Pawn Duel',
  icon: '♟️',
  blurb: 'Chess pawns, and only pawns. March, capture on the diagonal, and be first to break through.',
  rules:
    'Every piece is a chess pawn. On your turn move one pawn straight forward into an empty square, or capture an enemy pawn sitting one square diagonally forward. From its starting row a pawn may jump two squares (if double-step is on), and a pawn that just jumped past you can be caught “en passant” on your very next move only (if en passant is on). You win the moment one of your pawns reaches the far row, or you capture every enemy pawn. Unlike chess, being stuck with NO legal move is a LOSS, not a draw — so the game is always decisive. Turn double-step and en passant off on a 3×3 board and you get Hexapawn, the classic teaching game — where the second player can always win.',
  // Calibrated via `calibrate -- pawn-duel` (n=20/pair, 2026-07): strength keeps
  // climbing to the top rung on the Classic 8×6 tree, so Hard caps at p2000.
  // seat-0 win rate 52% — balanced.
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, cols: 8, rows: 6, doubleStep: 1, enPassant: 1 },
  presets: [
    { label: 'Classic 8×6', emoji: '⭐', params: { numPlayers: 2, cols: 8, rows: 6, doubleStep: 1, enPassant: 1 } },
    { label: 'Hexapawn 3×3', emoji: '🎓', params: { numPlayers: 2, cols: 3, rows: 3, doubleStep: 0, enPassant: 0 } },
    { label: 'Wide 10×8', emoji: '🤯', params: { numPlayers: 2, cols: 10, rows: 8, doubleStep: 1, enPassant: 1 } },
    { label: 'No Frills', emoji: '🚫', params: { numPlayers: 2, cols: 8, rows: 6, doubleStep: 0, enPassant: 0 } },
  ],
  knobs: [
    { key: 'cols', label: 'Files', min: 4, max: 10, step: 1 },
    { key: 'rows', label: 'Ranks', min: 5, max: 8, step: 1 },
    { key: 'doubleStep', label: 'Double-step', min: 0, max: 1, step: 1 },
    { key: 'enPassant', label: 'En passant', min: 0, max: 1, step: 1 },
  ],
  create: (wasm, p) => moveHandle(new wasm.PawnDuelWasm(p.cols, p.rows, p.doubleStep, p.enPassant)),
  Board: makeMoveBoard('pawn'),
  playerLabels: ['Red', 'Gold'],
  // A promotion win glows the breakthrough cell (winCells non-empty) and reads
  // fine with the default trophy. But a win by STALEMATE (opponent stuck) or by
  // ELIMINATION has no line to glow, so the default "{winner} wins!" looks
  // arbitrary — tell the story instead. (Shared resultFlavor seam.)
  resultFlavor: ({ result, winCells, board, labels, seats, t }) => {
    if (winCells.length > 0) return undefined; // promotion — keep the glow + default line
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner) || winner < 0) return undefined;
    const loser = winner === 0 ? 1 : 0;
    const loserGlyph = loser === 0 ? 'X' : 'O';
    const loserHasPawns = board.includes(loserGlyph);
    const name = (seat: number) => (labels[seat] ? t(labels[seat]) : t('Player {n}', { n: seat + 1 }));
    const humans = seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
    const lone = humans.length === 1 ? humans[0] : null;
    if (loserHasPawns) {
      // Stalemate-as-loss: the loser still has pawns but no legal move.
      if (lone === loser) return t('🚫 No moves left — you’re stuck, so {winner} wins.', { winner: name(winner) });
      if (lone === winner) return t('🚫 {loser} has no move — you win! 🎉', { loser: name(loser) });
      return t('🚫 No moves — {loser} is stuck, so {winner} wins!', { winner: name(winner), loser: name(loser) });
    }
    // Elimination: every enemy pawn captured.
    if (lone === loser) return t('💥 Every one of your pawns is gone — {winner} wins.', { winner: name(winner) });
    if (lone === winner) return t('💥 You captured every enemy pawn — you win! 🎉');
    return t('💥 {winner} captured every {loser} pawn!', { winner: name(winner), loser: name(loser) });
  },
};
