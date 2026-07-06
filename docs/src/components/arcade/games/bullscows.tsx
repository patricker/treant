import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

// Board string is single-sourced with the Rust engine (bullscows.rs `render`):
//   phase|len|sym|rep|cur|winner|mycode|oppcode|myrows|opprows
// rows are `digits:bulls:cows` joined by ';'. `mycode` is ALWAYS only the
// viewing seat's own code; `oppcode` is populated only once the game is over.
interface Row {
  guess: string;
  bulls: number;
  cows: number;
}
interface Parsed {
  phase: string;
  len: number;
  sym: number;
  repeats: boolean;
  cur: number;
  winner: string;
  mycode: string;
  oppcode: string;
  my: Row[];
  opp: Row[];
}
function parseBoard(board: string): Parsed {
  const parts = board.split('|');
  const rows = (s: string): Row[] =>
    s
      ? s.split(';').map((r) => {
          const [guess, b, c] = r.split(':');
          return { guess, bulls: Number(b), cows: Number(c) };
        })
      : [];
  return {
    phase: parts[0] || 'set',
    len: Number(parts[1]) || 4,
    sym: Number(parts[2]) || 10,
    repeats: parts[3] === '1',
    cur: Number(parts[4]) || 0,
    winner: parts[5] || '',
    mycode: parts[6] || '',
    oppcode: parts[7] || '',
    my: rows(parts[8] || ''),
    opp: rows(parts[9] || ''),
  };
}

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.BullsCowsWasm(p.length ?? 4, p.symbols ?? 10, p.repeats ?? 0);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    getBoardFor: (seat) => g.get_board_for(seat),
    // Secrecy-safe outgoing-move summary for the handoff blackout: parse the
    // seat's OWN view (get_board_for(seat) — guaranteed to omit the opponent's
    // code mid-game) and return only the last entry of that seat's own guess
    // log, `guess:bulls:cows`, all of which is public. Setup moves leave `my`
    // empty → undefined (nothing to show beyond the normal blackout).
    lastMoveSummaryFor: (seat) => {
      const st = parseBoard(g.get_board_for(seat));
      if (st.my.length === 0) return undefined;
      const last = st.my[st.my.length - 1];
      return `${last.guess}:${last.bulls}:${last.cows}`;
    },
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const s: string = g.legal_moves();
      return s ? s.split(',') : [];
    },
    weakMove: (pl, k, t, s) => g.weak_move(pl, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

function CodeDigits({ digits, len, hidden, active }: { digits: string; len: number; hidden?: boolean; active?: boolean }) {
  const chars = digits.split('');
  // The cursor highlights the next slot to fill (chars.length), but clamps to the
  // final slot so it stays visible once the code is complete (chars.length === len
  // would otherwise point past the last cell and vanish right as you submit).
  const cursor = Math.min(chars.length, len - 1);
  return (
    <div className={styles.bcCode}>
      {Array.from({ length: len }, (_, i) => {
        const d = chars[i];
        const filled = d !== undefined;
        return (
          <div
            key={i}
            className={`${styles.bcDigit} ${!filled ? styles.bcDigitEmpty : ''} ${active && i === cursor ? styles.bcDigitActive : ''} ${hidden ? styles.bcHidden : ''}`}
          >
            {filled ? (hidden ? '•' : d) : ''}
          </div>
        );
      })}
    </div>
  );
}

function Keypad({
  sym,
  entry,
  repeats,
  interactive,
  onDigit,
  onBackspace,
}: {
  sym: number;
  entry: string;
  repeats: boolean;
  interactive: boolean;
  onDigit: (d: number) => void;
  onBackspace: () => void;
}) {
  const { t } = useT();
  const used = new Set(entry.split(''));
  return (
    <div className={styles.bcKeypad}>
      {Array.from({ length: sym }, (_, d) => {
        const dup = !repeats && used.has(String(d));
        return (
          <button
            key={d}
            className={styles.bcKey}
            disabled={!interactive || dup}
            onClick={() => onDigit(d)}
            aria-label={t('Digit {n}', { n: d })}
          >
            {d}
          </button>
        );
      })}
      <button
        className={`${styles.bcKey} ${styles.bcKeyWide}`}
        disabled={!interactive || entry.length === 0}
        onClick={onBackspace}
        aria-label={t('Delete')}
      >
        ⌫
      </button>
    </div>
  );
}

function HistoryTable({ title, rows }: { title: string; rows: Row[] }) {
  const { t } = useT();
  return (
    <div className={styles.bcSection}>
      <div className={styles.bcLabel}>{title}</div>
      <div className={styles.bcTable}>
        {rows.length === 0 ? (
          <div className={styles.bcEmptyHist}>{t('No guesses yet')}</div>
        ) : (
          rows.map((r, i) => (
            <div key={i} className={styles.bcRow}>
              <span className={styles.bcRowGuess}>{r.guess}</span>
              <span className={styles.bcRowFeedback}>
                <span>🎯 {r.bulls}</span>
                <span>🐄 {r.cows}</span>
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function BullsCowsBoard({ board, interactive, onMove }: BoardProps) {
  const { t } = useT();
  const st = parseBoard(board);
  const [entry, setEntry] = useState('');
  const [hideCode, setHideCode] = useState(false);
  // Reset the in-progress entry whenever the board changes (our move landed or
  // the turn passed) so a half-typed guess never carries into the next turn.
  useEffect(() => setEntry(''), [board]);

  const settingUp = st.phase === 'set';
  const guessing = st.phase === 'guess';
  const over = st.phase === 'over';

  const addDigit = (d: number) => {
    if (entry.length >= st.len) return;
    setEntry(entry + d);
  };
  const backspace = () => setEntry(entry.slice(0, -1));
  const submit = () => {
    if (entry.length !== st.len) return;
    onMove(`${settingUp ? 'set' : 'g'}:${entry}`);
    setEntry('');
  };

  return (
    <div className={styles.bcWrap}>
      {/* Own secret code: shown during guessing/over, hideable. During setup the
          keypad entry IS the code being set. */}
      {(guessing || over) && (
        <div className={styles.bcSection}>
          <div className={styles.bcLabel}>
            <span>{t('Your secret code')}</span>
            {!over && (
              <button className={styles.bcToggle} onClick={() => setHideCode((v) => !v)}>
                {hideCode ? t('👁 show') : t('🙈 hide my code')}
              </button>
            )}
          </div>
          <CodeDigits digits={st.mycode} len={st.len} hidden={hideCode && !over} />
        </div>
      )}

      {/* Setup: build your secret code. */}
      {settingUp && (
        <div className={styles.bcSection}>
          <div className={styles.bcLabel}>{t('Set your secret code')}</div>
          <CodeDigits digits={entry} len={st.len} active={interactive} />
          <div className={styles.bcHint}>
            {st.repeats
              ? t('Pick {len} digits (0–{max}); repeats allowed.', { len: st.len, max: st.sym - 1 })
              : t('Pick {len} different digits (0–{max}).', { len: st.len, max: st.sym - 1 })}
          </div>
          <Keypad sym={st.sym} entry={entry} repeats={st.repeats} interactive={interactive} onDigit={addDigit} onBackspace={backspace} />
          <button className={styles.playBtn} disabled={!interactive || entry.length !== st.len} onClick={submit}>
            {t('Lock in code 🔒')}
          </button>
        </div>
      )}

      {/* Guessing: build a guess at the opponent's code. */}
      {guessing && (
        <div className={styles.bcSection}>
          <div className={styles.bcLabel}>{t('Your guess')}</div>
          <CodeDigits digits={entry} len={st.len} active={interactive} />
          <Keypad sym={st.sym} entry={entry} repeats={st.repeats} interactive={interactive} onDigit={addDigit} onBackspace={backspace} />
          <button className={styles.playBtn} disabled={!interactive || entry.length !== st.len} onClick={submit}>
            {t('Guess 🎯')}
          </button>
        </div>
      )}

      {over && st.oppcode && (
        <div className={styles.bcSection}>
          <div className={styles.bcLabel}>{t("Opponent's code was")}</div>
          <CodeDigits digits={st.oppcode} len={st.len} />
        </div>
      )}

      <HistoryTable title={t('Your guesses')} rows={st.my} />
      <HistoryTable title={t('Their guesses')} rows={st.opp} />
    </div>
  );
}

export const bullsCows: GameDefinition = {
  id: 'bulls-cows',
  name: 'Bulls & Cows',
  icon: '🎯',
  blurb: 'A secret-code duel. Guess the other player’s digits — bulls for a perfect hit, cows for a near miss — and crack the code first.',
  // Hand-set: strict alternation gives seat 0 a first-guesser edge that dominates
  // the self-play ladder, so win-rate calibration is meaningless here (nim
  // precedent). These map to the deducer's candidate-scoring budget + sampling:
  // Easy scores few candidates and samples sloppily from a wide top-K; Hard
  // scores exhaustively and plays the information-optimal guess.
  difficulty: {
    easy: { playouts: 6, topK: 8, temp: 2.5 },
    medium: { playouts: 40, topK: 4, temp: 0.9 },
    hard: { playouts: 100000, topK: 1, temp: 0 },
  },
  hiddenInfo: true,
  noUndo: true,
  defaultParams: { numPlayers: 2, length: 4, symbols: 10, repeats: 0 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, length: 4, symbols: 10, repeats: 0 } },
    { label: 'Quick', emoji: '⚡', params: { numPlayers: 2, length: 3, symbols: 6, repeats: 0 } },
    { label: 'Expert', emoji: '🤯', params: { numPlayers: 2, length: 5, symbols: 10, repeats: 1 } },
  ],
  knobs: [
    { key: 'length', label: 'Code length', min: 3, max: 5, step: 1 },
    { key: 'symbols', label: 'Digits (0..n-1)', min: 6, max: 10, step: 1 },
    { key: 'repeats', label: 'Allow repeats', min: 0, max: 1, step: 1 },
  ],
  create: makeHandle,
  Board: BullsCowsBoard,
  playerLabels: ['Player 1', 'Player 2'],
};
