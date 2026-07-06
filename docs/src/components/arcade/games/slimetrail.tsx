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
function SlimetrailBoard({ board, params, currentPlayer, interactive, legalMoves, winCells, onMove }: BoardProps) {
  const { t } = useT();
  const n = params.size;
  const goal0 = (n - 1) * n; // P0 / Red — bottom-left
  const goal1 = n - 1; //        P1 / Gold — top-right
  const targets = new Set(legalMoves.map(Number));
  const wins = new Set(winCells ?? []);
  const turnColor = `var(--arc-p${currentPlayer + 1})`;

  const cells = Array.from({ length: n * n }, (_, i) => board[i] ?? ' ');

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
};
