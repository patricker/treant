import { useEffect, useState } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import { useT } from '../i18n';
import styles from '../arcade.module.css';

function makeHandle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.NimWasm(p.stones, p.heaps ?? 1, p.maxTake ?? 2, p.misere ?? 0);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => (g.current_player() === 'P1' ? 0 : 1),
    isTerminal: () => g.is_terminal(),
    // Misère polarity lives in the engine's result(); the tile never re-derives
    // who won, so the two can't disagree (the comma/dash lesson from quadline).
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => {
      const s: string = g.legal_moves();
      return s ? s.split(',') : [];
    },
    weakMove: (p, k, t, s) => g.weak_move(p, k, t, s) ?? undefined,
    free: () => g.free(),
  };
}

function NimBoard({ board, params, interactive, onMove }: BoardProps) {
  const { t, tn } = useT();
  const heaps = board.split(',').map((s) => Number(s) || 0);
  const maxTake = params.maxTake ?? 0; // 0 == take any number from one heap
  const cap = (n: number) => (maxTake === 0 ? n : Math.min(n, maxTake));

  const [sel, setSel] = useState<{ heap: number; count: number } | null>(null);
  const [shake, setShake] = useState<number | null>(null);
  // Drop any stale selection once the board changes (our move landed, or the AI
  // replied) so a highlight never points at the wrong heap.
  useEffect(() => setSel(null), [board]);

  // Classic single-pile feel: one heap with a small finite take limit renders
  // the original pile + quick-take buttons, unchanged from the shipped toy.
  const classic = heaps.length === 1 && maxTake >= 1 && maxTake <= 3;

  if (classic) {
    const stones = heaps[0];
    return (
      <div className={styles.nimWrap}>
        <div className={styles.nimPile}>
          {Array.from({ length: stones }, (_, i) => (
            <span key={i} className={styles.nimStone} />
          ))}
        </div>
        <div className={styles.nimCount}>{tn(stones, '{n} stone left', '{n} stones left')}</div>
        <div className={styles.nimButtons}>
          {Array.from({ length: maxTake }, (_, i) => i + 1).map((k) => (
            <button
              key={k}
              className={styles.nimTake}
              disabled={!interactive || stones < k}
              onClick={() => onMove(`0-${k}`)}
            >
              {t('Take {n}', { n: k })}
            </button>
          ))}
        </div>
      </div>
    );
  }

  // Multi-heap / unbounded take: tap a stone to select "take from here to the
  // end of the row", tap again (or the confirm button) to commit. Stones beyond
  // the max-take limit are dimmed and un-tappable; tapping a full heap that's
  // over the cap head-shakes instead of dying silently.
  const tap = (heap: number, i: number, n: number) => {
    if (!interactive) return;
    const count = n - i; // this stone plus everything to its right
    if (count > cap(n)) {
      setShake(heap);
      window.setTimeout(() => setShake(null), 550);
      return;
    }
    if (sel && sel.heap === heap && sel.count === count) {
      onMove(`${heap}-${count}`);
      setSel(null);
      return;
    }
    setSel({ heap, count });
  };

  return (
    <div className={styles.nimWrap}>
      <div className={styles.shiftCaption}>
        {sel ? t('Tap again to take {n}', { n: sel.count }) : t('Tap stones from the right to take')}
      </div>
      <div className={styles.nimHeaps}>
        {heaps.map((n, h) => (
          <div key={h} className={`${styles.nimHeapRow} ${h === shake ? styles.shakeCell : ''}`}>
            <span className={styles.nimHeapLabel}>{n}</span>
            <div className={styles.nimHeapDots}>
              {n === 0 ? (
                <span className={styles.nimHeapEmpty}>{t('empty')}</span>
              ) : (
                Array.from({ length: n }, (_, i) => {
                  const count = n - i;
                  const legal = count <= cap(n);
                  const selected = sel != null && sel.heap === h && count <= sel.count;
                  return (
                    <button
                      key={i}
                      className={`${styles.nimStoneBtn} ${legal ? '' : styles.nimStoneOff} ${selected ? styles.nimStoneSel : ''}`}
                      disabled={!interactive || !legal}
                      onClick={() => tap(h, i, n)}
                      aria-label={t('Heap {h}: take {n}', { h: h + 1, n: count })}
                    />
                  );
                })
              )}
            </div>
          </div>
        ))}
      </div>
      {sel && (
        <div className={styles.nimButtons}>
          <button
            className={styles.nimTake}
            disabled={!interactive}
            onClick={() => {
              onMove(`${sel.heap}-${sel.count}`);
              setSel(null);
            }}
          >
            {t('Take {n}', { n: sel.count })}
          </button>
        </div>
      )}
    </div>
  );
}

export const nim: GameDefinition = {
  id: 'nim',
  name: 'Nim',
  icon: '🪨',
  blurb: 'Clear stones from the heaps. Take the last one to win — or flip on misère, where the last stone is the one to avoid.',
  // Hand-set (nim lacks the uniform seat API the calibration harness uses).
  // Easy stays genuinely beatable — including in misère, where the solver still
  // protects proven wins but the wide top-K/temp lets it slip a won position.
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 4, temp: 1 },
    hard: { playouts: 100, topK: 1, temp: 0 },
  },
  defaultParams: { numPlayers: 2, stones: 15, heaps: 1, maxTake: 2, misere: 0 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { numPlayers: 2, stones: 15, heaps: 1, maxTake: 2, misere: 0 } },
    { label: 'Marienbad', emoji: '🎩', params: { numPlayers: 2, stones: 7, heaps: 4, maxTake: 0, misere: 1 } },
    { label: 'Ten Heaps', emoji: '🤯', params: { numPlayers: 2, stones: 17, heaps: 8, maxTake: 0, misere: 0 } },
    { label: 'Last Loses', emoji: '🙃', params: { numPlayers: 2, stones: 15, heaps: 1, maxTake: 2, misere: 1 } },
  ],
  knobs: [
    { key: 'stones', label: 'Stones', min: 3, max: 60, step: 1 },
    { key: 'heaps', label: 'Heaps', min: 1, max: 8, step: 1 },
    { key: 'maxTake', label: 'Max take (0 = any)', min: 0, max: 10, step: 1 },
    { key: 'misere', label: 'Misère', min: 0, max: 1, step: 1 },
  ],
  create: makeHandle,
  Board: NimBoard,
};
