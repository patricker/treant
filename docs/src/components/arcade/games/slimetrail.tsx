import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.SlimetrailWasm(p.size);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (k) => g.playout_n(k),
    legalMoves: () => {
      const s: string = g.legal_moves();
      return s ? s.split(',') : [];
    },
    weakMove: (pl, k, t, s) => g.weak_move(pl, k, t, s) ?? undefined,
    winningCells: () => {
      const s = g.winning_cells();
      return s ? s.split(',') : [];
    },
    free: () => g.free(),
  };
}

// Slimetrail board. One shared snail is walked toward YOUR goal corner; the cell
// it leaves turns to permanent slime. Move = the destination cell index (the
// engine already lists only the token's legal steps in `legalMoves`).
function SlimetrailBoard({
  board,
  params,
  currentPlayer,
  interactive,
  legalMoves,
  winCells,
  terminal,
  onMove,
}: BoardProps) {
  const { t } = useT();
  const n = params.size;
  const goal0 = (n - 1) * n; // P0 / Red — bottom-left
  const goal1 = n - 1; //        P1 / Gold — top-right
  const targets = new Set(legalMoves.map(Number));
  const wins = new Set(winCells ?? []);
  const turnColor = `var(--arc-p${currentPlayer + 1})`;

  const cells = Array.from({ length: n * n }, (_, i) => board[i] ?? ' ');

  // Box-in end: the game is over with no winning goal cell (winCells empty),
  // so the loser's snail is trapped. Mark the snail and the slime squares
  // penning it in, so the story ("nowhere to step") is visible on the board.
  const boxedIn = !!terminal && wins.size === 0;
  const trappedIdx = boxedIn ? cells.indexOf('T') : -1;
  const cage = new Set<number>();
  if (trappedIdx >= 0) {
    const sr = Math.floor(trappedIdx / n);
    const sc = trappedIdx % n;
    for (let dr = -1; dr <= 1; dr++) {
      for (let dc = -1; dc <= 1; dc++) {
        if (dr === 0 && dc === 0) continue;
        const rr = sr + dr;
        const cc = sc + dc;
        if (rr < 0 || rr >= n || cc < 0 || cc >= n) continue;
        const ni = rr * n + cc;
        if (cells[ni] === '#') cage.add(ni);
      }
    }
  }

  return (
    <div>
      <div className={styles.shiftCaption}>
        <span className={styles.hexSwatch} style={{ background: 'var(--arc-p1)' }} />
        {t("Red's goal (bottom-left)")} &nbsp;·&nbsp;
        <span className={styles.hexSwatch} style={{ background: 'var(--arc-p2)' }} />
        {t("Gold's goal (top-right)")} &nbsp;— {t('walk 🐌 to YOUR corner')}
      </div>
      <div className={styles.slimeGrid} style={{ gridTemplateColumns: `repeat(${n}, minmax(0, 1fr))` }}>
        {cells.map((ch, i) => {
          const isSlime = ch === '#';
          const isToken = ch === 'T';
          const isTarget = interactive && targets.has(i);
          const goalOwner = i === goal0 ? 0 : i === goal1 ? 1 : -1;
          const isGoal = goalOwner >= 0;
          const goalColor = goalOwner === 0 ? 'var(--arc-p1)' : 'var(--arc-p2)';

          // Layered background: slime cells are unmistakably green; goal corners
          // carry a strong wash of their owner's colour so "whose corner" is
          // self-evident; everything else is a neutral card.
          const bg = isSlime
            ? 'var(--arc-slime)'
            : isGoal
              ? `color-mix(in srgb, ${goalColor} 42%, var(--arc-card))`
              : 'var(--arc-card)';

          const cls = [
            styles.slimeCell,
            isTarget ? styles.slimeTarget : '',
            wins.has(i) ? styles.winCell : '',
            i === trappedIdx ? styles.slimeTrapped : '',
            cage.has(i) ? styles.slimeCage : '',
          ]
            .filter(Boolean)
            .join(' ');

          return (
            <button
              key={i}
              className={cls}
              disabled={!isTarget}
              onClick={() => isTarget && onMove(String(i))}
              style={{
                background: bg,
                // Tint the legal-step ring by whose turn it is.
                ...(isTarget ? { outlineColor: turnColor } : {}),
              }}
              aria-label={t('Cell {n}: {state}', {
                n: i + 1,
                state: isToken
                  ? t('snail')
                  : isSlime
                    ? t('slime')
                    : isGoal
                      ? goalOwner === 0
                        ? t("Red's goal")
                        : t("Gold's goal")
                      : t('empty'),
              })}
            >
              {isToken ? (
                <span className={styles.slimeSnail} aria-hidden>
                  🐌
                </span>
              ) : isGoal ? (
                <span className={styles.slimeGoalStar} style={{ color: goalColor }} aria-hidden>
                  ★
                </span>
              ) : isTarget ? (
                <span className={styles.slimeTargetDot} style={{ background: turnColor }} />
              ) : (
                ''
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const slimetrail: GameDefinition = {
  id: 'slimetrail',
  name: 'Slimetrail',
  icon: '🐌',
  blurb: 'Walk the shared snail to YOUR corner — the trail it leaves behind is slime nobody can cross.',
  rules:
    'One snail sits on the board and BOTH players take turns walking it one step — in any of the 8 directions (including diagonally). Every square the snail leaves turns to slime and can never be entered again. Red wins the moment the snail reaches Red’s corner (bottom-left); Gold wins when it reaches Gold’s corner (top-right) — it counts no matter who moved it there, so never push the snail into your rival’s corner! If it’s your turn and the snail is boxed in with nowhere to go, you lose.',
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, size: 7 },
  presets: [
    { label: 'Classic 7', emoji: '⭐', params: { numPlayers: 2, size: 7 } },
    { label: 'Quick 5', emoji: '⚡', params: { numPlayers: 2, size: 5 } },
    { label: 'Big 9', emoji: '🤯', params: { numPlayers: 2, size: 9 } },
  ],
  knobs: [{ key: 'size', label: 'Size', min: 5, max: 9, step: 1 }],
  create: makeHandle,
  Board: SlimetrailBoard,
  playerLabels: ['Red', 'Gold'],
  // A box-in loss (no goal reached) would otherwise show the generic "{winner}
  // wins!" trophy, which reads as arbitrary to a kid. Tell the story instead —
  // the snail got trapped with no move. A goal win keeps the classic line +
  // goal glow (winCells is populated there, so we bow out and return undefined).
  resultFlavor: ({ result, winCells, labels, seats, t }) => {
    if (winCells.length > 0) return undefined; // goal win — keep the classic line
    const winner = Number(result) - 1;
    if (!Number.isFinite(winner) || winner < 0) return undefined;
    const loser = winner === 0 ? 1 : 0;
    const name = (seat: number) =>
      labels[seat] ? t(labels[seat]) : t('Player {n}', { n: seat + 1 });
    // A lone human hears it in the second person so a loss is unmistakable.
    const humans = seats.map((k, i) => (k === 'human' ? i : -1)).filter((i) => i >= 0);
    if (humans.length === 1) {
      return humans[0] === loser
        ? t('🐌 Snail trapped! You’re boxed in — {winner} wins.', { winner: name(winner) })
        : t('🐌 Snail trapped! {loser} is boxed in — you win! 🎉', { loser: name(loser) });
    }
    return t('🐌 Snail trapped! {loser} had no move — {winner} wins!', {
      winner: name(winner),
      loser: name(loser),
    });
  },
};
