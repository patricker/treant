import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

const SEAT = ['var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)'];
const PIPS: Record<number, string> = { 1: '⚀', 2: '⚁', 3: '⚂', 4: '⚃', 5: '⚄', 6: '⚅' };

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.ClimbWasm(p.numPlayers, p.toWin);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      if (g.is_terminal()) return [];
      const s = g.legal_moves();
      return s ? s.split(',') : [];
    },
    weakMove: (pl, k, t, s) => g.weak_move(pl, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

interface ColState {
  sum: number;
  height: number;
  claimed: number; // -1 unclaimed, else seat
  runner: number; // -1 none, else the current player's runner position
  banked: number[]; // per-seat banked marker position
}

// Turn a pairing move code ("P4-9" / "P4" / "P8-8") into a readable label.
function pairLabel(t: (k: string, p?: Record<string, string | number>) => string, code: string): string {
  const body = code.slice(1);
  if (body.includes('-')) {
    const [a, b] = body.split('-');
    return a === b ? t('Advance {a} twice', { a }) : t('Advance {a} & {b}', { a, b });
  }
  return t('Advance {a}', { a: body });
}

function ClimbBoard({ board, currentPlayer, interactive, legalMoves, onMove }: BoardProps) {
  const { t } = useT();
  const parts = board.split('|');
  const meta = (parts[0] ?? '').split(',');
  const numPlayers = Number(meta[1]) || 2;
  const toWin = Number(meta[2]) || 3;
  const phase = meta[5] ?? 'act';
  const busted = meta[6] === '1';
  const dice = !parts[1] || parts[1] === '-' ? [] : parts[1].split(',').map(Number);

  const cols: ColState[] = parts.slice(2).map((cstr, i) => {
    const [h, claimed, runner, bankedStr] = cstr.split(':');
    return {
      sum: i + 2,
      height: Number(h),
      claimed: Number(claimed),
      runner: Number(runner),
      banked: (bankedStr ?? '').split(',').map(Number),
    };
  });

  const claimedCounts = Array.from({ length: numPlayers }, (_, p) => cols.filter((c) => c.claimed === p).length);

  const rollMoves = legalMoves.filter((m) => m === 'Roll' || m === 'Stop');
  const pairMoves = legalMoves.filter((m) => m.startsWith('P'));

  const CELL = 15; // px per step

  return (
    <div className={styles.pigWrap}>
      {/* Per-seat claim tally */}
      <div className={styles.pigScores}>
        {claimedCounts.map((n, p) => (
          <div key={p} className={`${styles.pigRow} ${p === currentPlayer ? styles.pigRowActive : ''}`}>
            <span className={styles.pigName} style={{ color: SEAT[p] }}>
              ●
            </span>
            <div className={styles.pigBar}>
              <div className={styles.pigBarFill} style={{ width: `${Math.min(100, (n / toWin) * 100)}%`, background: SEAT[p] }} />
            </div>
            <span className={styles.pigVal}>{t('{n}/{toWin}', { n, toWin })}</span>
          </div>
        ))}
      </div>

      {/* Dice + status */}
      <div className={styles.pigCenter}>
        <div style={{ fontSize: '2.4rem', lineHeight: 1, letterSpacing: 4 }}>
          {dice.length ? dice.map((d, i) => <span key={i}>{PIPS[d]}</span>) : '🎲🎲🎲🎲'}
        </div>
        <div className={styles.pigTurn}>
          {busted
            ? t('💥 Bust! No legal pairing — turn lost.')
            : phase === 'pair'
              ? t('Pick a pairing to advance your climbers.')
              : t('Roll again to climb higher, or stop to bank. Claim {toWin} columns to win.', { toWin })}
        </div>
      </div>

      {/* Mountain: one track per column 2..12, bottom-aligned */}
      <div style={{ display: 'flex', alignItems: 'flex-end', gap: 3, width: '100%', justifyContent: 'center' }}>
        {cols.map((c) => {
          const owned = c.claimed >= 0;
          return (
            <div
              key={c.sum}
              style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 3, minWidth: 0 }}
              aria-label={t('Column {c}: height {h}, {status}', {
                c: c.sum,
                h: c.height,
                status: owned ? t('claimed') : t('open'),
              })}
            >
              <div
                style={{
                  fontSize: '0.7rem',
                  fontWeight: 900,
                  // Fixed light tone: --arc-ink is near-black on the light site
                  // theme, which vanished against the dark page backdrop.
                  color: owned ? '#fff' : '#cfd0e4',
                  background: owned ? SEAT[c.claimed] : 'transparent',
                  borderRadius: 6,
                  width: '100%',
                  textAlign: 'center',
                  padding: '1px 0',
                }}
              >
                {owned ? '👑' : c.sum}
              </div>
              <div style={{ display: 'flex', flexDirection: 'column-reverse', gap: 2, width: '100%', opacity: owned ? 0.45 : 1 }}>
                {Array.from({ length: c.height }, (_, k) => {
                  const level = k + 1; // 1..height
                  const runnerHere = c.runner === level;
                  const bankedHere = c.banked.map((b, p) => (b === level ? p : -1)).filter((p) => p >= 0);
                  const atTop = level === c.height;
                  return (
                    <div
                      key={level}
                      style={{
                        height: CELL,
                        borderRadius: 4,
                        background: 'var(--arc-soft)',
                        border: atTop ? '1px dashed var(--arc-line, rgba(0,0,0,.25))' : 'none',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        gap: 2,
                        position: 'relative',
                      }}
                    >
                      {bankedHere.map((p) => (
                        <span key={`b${p}`} style={{ width: 6, height: 6, borderRadius: '50%', background: SEAT[p] }} />
                      ))}
                      {runnerHere && (
                        <span
                          style={{
                            position: 'absolute',
                            width: 11,
                            height: 11,
                            borderRadius: '50%',
                            background: SEAT[currentPlayer],
                            border: '2px solid #fff',
                            boxShadow: '0 0 0 1.5px ' + SEAT[currentPlayer],
                          }}
                        />
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>

      {/* Actions, driven entirely by legalMoves */}
      {pairMoves.length > 0 && (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, justifyContent: 'center' }}>
          {pairMoves.map((m) => (
            <button
              key={m}
              className={styles.pigRoll}
              style={{ flex: '1 1 40%', minWidth: 120, background: 'var(--arc-p3)' }}
              disabled={!interactive}
              onClick={() => onMove(m)}
            >
              {pairLabel(t, m)}
            </button>
          ))}
        </div>
      )}
      {rollMoves.length > 0 && (
        <div className={styles.pigButtons}>
          {rollMoves.includes('Roll') && (
            <button className={styles.pigRoll} disabled={!interactive} onClick={() => onMove('Roll')}>
              {t('🎲 Roll')}
            </button>
          )}
          {rollMoves.includes('Stop') && (
            <button className={styles.pigHold} disabled={!interactive} onClick={() => onMove('Stop')}>
              {t('✋ Stop')}
            </button>
          )}
        </div>
      )}
    </div>
  );
}

export const climb: GameDefinition = {
  id: 'climb',
  noUndo: true, // dice game — replay-based undo would reroll the rolls
  name: 'Climb',
  icon: '🧗',
  blurb: 'Pair the dice, push your luck up the mountain — bust and you slide back.',
  // Measured by the self-play ladder (n=8/pair): field 12% → 50% → 78%;
  // Hard capped at p800 where strength plateaus. Seat balance 53%.
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  defaultParams: { numPlayers: 2, toWin: 3 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, toWin: 3 } },
    { label: '4-Player', emoji: '🎉', params: { numPlayers: 4, toWin: 3 } },
    { label: 'Sprint', emoji: '⚡', params: { numPlayers: 2, toWin: 2 } },
    { label: 'Marathon', emoji: '🤯', params: { numPlayers: 2, toWin: 5 } },
  ],
  knobs: [
    { key: 'numPlayers', label: 'Players', min: 2, max: 4, step: 1 },
    { key: 'toWin', label: 'Columns to win', min: 2, max: 5, step: 1 },
  ],
  create: makeHandle,
  Board: ClimbBoard,
  playerLabels: ['Red', 'Yellow', 'Green', 'Purple'],
};
