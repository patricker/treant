# Arcade Phase 2b (2048 + solo mode) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or superpowers:executing-plans. Checkbox (`- [ ]`) steps.

**Goal:** Add 2048 to the arcade as the first single-player game, with a new solo mode (You-play + Hint / Watch-AI) and swipe/arrow input.

**Architecture:** Additive solo support (`solo` flag, `'solo'` mode, optional `statusText`/`endText` on the handle, `getHint` + fixed solo-AI strength in the session, a solo branch in setup/play) + a 2048 `GameDefinition`. JS-only — 2048's WASM is complete.

**Tech Stack:** React 19, Docusaurus, TS, `treant-wasm` (`Game2048Wasm`). Spec: `specs/2026-06-15-arcade-phase2b-2048-solo-design.md`.

---

## File Structure
- Modify: `arcade/gameTypes.ts` — `Mode` += `'solo'`; `seatTypes`; `GameHandle.statusText?/endText?`; `GameDefinition.solo?/formatHint?`.
- Modify: `arcade/useGameSession.ts` — `statusText`/`endText` state, `getHint`, fixed solo-AI in `runAiTurn`.
- Modify: `arcade/GameSetup.tsx` — solo 2-option toggle, hide difficulty for solo.
- Modify: `arcade/GamePlay.tsx` — solo banner/overlay + Hint button.
- Create: `arcade/games/game2048.tsx`; modify `games/index.ts`, `arcade.module.css`.

---

## Task 1: Solo-mode abstraction

- [ ] **Step 1: `gameTypes.ts`**

`Mode` union: add `'solo'`:
```ts
export type Mode = 'pvp' | 'pvai' | 'aivai' | 'solo';
```
`seatTypes`: add the solo case (top):
```ts
export function seatTypes(mode: Mode, numPlayers: number): SeatType[] {
  if (mode === 'solo') return Array(numPlayers).fill('human');
  if (mode === 'pvp') return Array(numPlayers).fill('human');
  if (mode === 'aivai') return Array(numPlayers).fill('ai');
  return ['human', ...Array(Math.max(0, numPlayers - 1)).fill('ai')] as SeatType[];
}
```
`GameHandle` — add optional solo hooks:
```ts
  statusText?(): string;
  endText?(): string;
```
`GameDefinition` — add:
```ts
  solo?: boolean;
  formatHint?(move: string): string;
```

- [ ] **Step 2: `useGameSession.ts`**

Add solo constants + state, fixed solo AI, `getHint`, and expose `statusText`/`endText`.
- After `const AI_DELAY_MS = 400;` add: `const SOLO_AI_PLAYOUTS = 800; const HINT_PLAYOUTS = 1500;`
- Add state: `const [statusText, setStatusText] = useState(''); const [endText, setEndText] = useState('');`
- A helper to refresh solo text (call wherever board is set):
```ts
  const refreshText = (h: GameHandle) => {
    setStatusText(h.statusText?.() ?? '');
  };
```
  Call `refreshText(h)` right after each `setBoard(h.getBoard())` in `runAiTurn`, `start`, and `onHumanMove`. On terminal set `setEndText(h.endText?.() ?? '')` (right before/after `setPhase('over')` in all three terminal branches).
- In `runAiTurn`, replace the move pick with a solo-aware version:
```ts
      const mv = def.solo
        ? (h.playoutN(SOLO_AI_PLAYOUTS), h.bestMove())
        : pickAiMove(h, difficulty);
```
- Add `getHint`:
```ts
  const getHint = useCallback((): string | undefined => {
    const h = handleRef.current;
    if (!h) return undefined;
    h.playoutN(HINT_PLAYOUTS);
    return h.bestMove();
  }, []);
```
- Return: add `statusText, endText, getHint` to the returned object.

- [ ] **Step 3: `GameSetup.tsx`** — solo branch

Change the default mode for solo games and render the right pickers. Replace the mode block:
```tsx
  const [mode, setMode] = useState<Mode>(def.solo ? 'solo' : 'pvai');
  /* …difficulty, params, showCustom unchanged… */

  /* in JSX, replace the "Who's playing?" + ModePicker + difficulty block with: */
  <div className={styles.setupLabel}>{def.solo ? 'Mode' : "Who's playing?"}</div>
  {def.solo ? (
    <div className={styles.seg}>
      <button className={`${styles.segBtn} ${mode === 'solo' ? styles.segOn : ''}`} onClick={() => setMode('solo')}>🙂 You play</button>
      <button className={`${styles.segBtn} ${mode === 'aivai' ? styles.segOn : ''}`} onClick={() => setMode('aivai')}>🤖 Watch AI</button>
    </div>
  ) : (
    <>
      <ModePicker value={mode} onChange={setMode} />
      {mode !== 'pvp' && (
        <>
          <div className={styles.setupLabel}>AI strength</div>
          <DifficultyPicker value={difficulty} onChange={setDifficulty} />
        </>
      )}
    </>
  )}
```
Keep the Board/preset/custom block, but for solo games with **no presets and no
knobs**, guard it:
```tsx
  {def.presets.length > 0 && (<><div className={styles.setupLabel}>Board</div><PresetChips … /></>)}
  {def.knobs.length > 0 && (<><button className={styles.customToggle} …>Customize</button>{showCustom && <CustomKnobs … />}</>)}
```

- [ ] **Step 4: `GamePlay.tsx`** — solo banner, overlay, Hint

- Destructure new session fields: `const s = useGameSession(...)` already; use `s.statusText`, `s.endText`, `s.getHint`.
- Add hint state: `const [hint, setHint] = useState('');` (import `useState`).
- Banner:
```tsx
  const status = def.solo
    ? s.statusText
    : s.phase === 'thinking' ? '🤖 Thinking…'
    : s.phase === 'over' ? ''
    : `${PLAYER_LABEL[s.current] ?? `Player ${s.current + 1}`}'s turn`;
```
- Below the board (before the overlay), add the Hint control for solo You-play:
```tsx
  {def.solo && mode === 'solo' && s.phase === 'playing' && (
    <div className={styles.hintRow}>
      <button className={styles.hintBtn} onClick={() => {
        const m = s.getHint?.();
        setHint(m ? (def.formatHint ? def.formatHint(m) : m) : '');
      }}>💡 Hint</button>
      {hint && <span className={styles.hintText}>Try {hint}</span>}
    </div>
  )}
```
- Overlay text: `{def.solo ? s.endText : winnerLabel(s.result)}`; icon `{def.solo ? '🎮' : s.result === 'Draw' ? '🤝' : '🏆'}`.

- [ ] **Step 5: Build check** — `cd docs && npm run build 2>&1 | tail -6` → success (existing games unaffected).

- [ ] **Step 6: Commit**
```bash
git add docs/src/components/arcade
git commit -m "feat(arcade): solo mode (You-play + Hint / Watch-AI)"
```

---

## Task 2: 2048

- [ ] **Step 1: `games/game2048.tsx`**
```tsx
import { useEffect, useRef } from 'react';
import type { BoardProps, GameDefinition, GameHandle, GameParams } from '../gameTypes';
import styles from '../arcade.module.css';

const DIRS = ['Up', 'Down', 'Left', 'Right'] as const;
const TILE_BG: Record<number, string> = {
  0: 'var(--arc-soft)', 2: '#eee4da', 4: '#ede0c8', 8: '#f2b179', 16: '#f59563',
  32: '#f67c5f', 64: '#f65e3b', 128: '#edcf72', 256: '#edcc61', 512: '#edc850',
  1024: '#edc53f', 2048: '#edc22e',
};
const tileColor = (v: number) => (v <= 4 ? '#776e65' : '#f9f6f2');

function makeHandle(wasm: any, _p: GameParams): GameHandle {
  const g = new wasm.Game2048Wasm();
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => (g.get_board() as number[]).join(','),
    currentPlayer: () => 0,
    isTerminal: () => g.is_terminal(),
    result: () => '',
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    legalMoves: () => ['Up', 'Down', 'Left', 'Right'],
    statusText: () => `Score ${g.score()} · Best ${g.max_tile()}`,
    endText: () =>
      g.max_tile() >= 2048
        ? `🎉 You made ${g.max_tile()}!  Score ${g.score()}`
        : `Game over — score ${g.score()}, best tile ${g.max_tile()}`,
    free: () => g.free(),
  };
}

function Game2048Board({ board, interactive, onMove }: BoardProps) {
  const tiles = board.split(',').map(Number);
  const startRef = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    if (!interactive) return;
    const onKey = (e: KeyboardEvent) => {
      const map: Record<string, string> = { ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right' };
      if (map[e.key]) { e.preventDefault(); onMove(map[e.key]); }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [interactive, onMove]);

  const onTouchStart = (e: React.TouchEvent) => {
    const t = e.touches[0];
    startRef.current = { x: t.clientX, y: t.clientY };
  };
  const onTouchEnd = (e: React.TouchEvent) => {
    const s = startRef.current; startRef.current = null;
    if (!s || !interactive) return;
    const t = e.changedTouches[0];
    const dx = t.clientX - s.x, dy = t.clientY - s.y;
    if (Math.abs(dx) < 20 && Math.abs(dy) < 20) return;
    onMove(Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? 'Right' : 'Left') : (dy > 0 ? 'Down' : 'Up'));
  };

  return (
    <div className={styles.g2048Wrap}>
      <div className={styles.g2048Grid} onTouchStart={onTouchStart} onTouchEnd={onTouchEnd}>
        {tiles.map((v, i) => (
          <div key={i} className={styles.g2048Tile}
            style={{ background: TILE_BG[v] ?? '#3c3a32', color: tileColor(v), fontSize: v >= 1024 ? '1.1rem' : '1.5rem' }}>
            {v > 0 ? v : ''}
          </div>
        ))}
      </div>
      <div className={styles.g2048Arrows}>
        <button className={styles.g2048Arrow} disabled={!interactive} onClick={() => onMove('Up')} aria-label="Up">⬆️</button>
        <div className={styles.g2048ArrowRow}>
          <button className={styles.g2048Arrow} disabled={!interactive} onClick={() => onMove('Left')} aria-label="Left">⬅️</button>
          <button className={styles.g2048Arrow} disabled={!interactive} onClick={() => onMove('Down')} aria-label="Down">⬇️</button>
          <button className={styles.g2048Arrow} disabled={!interactive} onClick={() => onMove('Right')} aria-label="Right">➡️</button>
        </div>
      </div>
    </div>
  );
}

export const game2048: GameDefinition = {
  id: '2048',
  name: '2048',
  icon: '🔢',
  blurb: 'Swipe to merge tiles. Reach 2048 — or watch the AI try.',
  solo: true,
  defaultParams: { numPlayers: 1 },
  presets: [],
  knobs: [],
  create: makeHandle,
  Board: Game2048Board,
  formatHint: (m) => ({ Up: '⬆️ Up', Down: '⬇️ Down', Left: '⬅️ Left', Right: '➡️ Right' }[m] ?? m),
};
```

- [ ] **Step 2: register + CSS**

`index.ts`: import `game2048`, add to `GAMES` after `shift`.
`arcade.module.css`:
- `.g2048Wrap` (column, center, gap 14px).
- `.g2048Grid` (4-col grid, gap 8px, `background: #bbada0`, padding 8px, radius 12px, `touch-action: none`, `max-width: 320px`).
- `.g2048Tile` (aspect 1, radius 6px, flex center, font-weight 800).
- `.g2048Arrows` (column, center) / `.g2048ArrowRow` (row, gap 8px).
- `.g2048Arrow` (chunky button ~52px, soft bg, radius 12px, font 1.4rem, disabled dimmed).
Also add `.hintRow` (row, center, gap 10px, margin 8px), `.hintBtn` (rounded accent-outline button), `.hintText` (bold accent).

- [ ] **Step 3: Build + commit**

`cd docs && npm run build 2>&1 | tail -4` → success.
```bash
git add docs/src/components/arcade
git commit -m "feat(arcade): 2048 (swipe/arrow/keyboard, score, Hint, Watch-AI)"
```

---

## Task 3: End-to-end verification

- [ ] **Step 1:** Dev server up (the LAN one likely still running; else `cd docs && npm start -- --host 0.0.0.0 --port 3939 --no-open`).
- [ ] **Step 2: Playwright**, no console errors:
  - Launcher shows **2048** tile (6 games).
  - 2048 setup shows **You play / Watch AI** (no difficulty, no presets).
  - **You play:** click arrow buttons → board changes, `statusText` score climbs.
  - **Hint:** click 💡 Hint → a direction appears.
  - **Watch AI:** auto-plays several moves, score climbs, tiles merge.
  - **Regression:** Connect Four vs-AI still plays (a human drop → AI replies).
- [ ] **Step 3:** Clean `.playwright-mcp/`. Leave server up for the user.

---

## Self-Review Notes
- **Spec coverage:** solo flag/mode/seatTypes (Task 1.1), statusText/endText/getHint/fixed-AI (1.2), setup solo toggle (1.3), play banner/overlay/Hint (1.4), 2048 game (Task 2), verification (Task 3). Mapped.
- **Additive safety:** `solo`, `statusText`, `endText`, `formatHint` optional; the 5 existing games don't set them and keep the unchanged non-solo paths.
- **Type consistency:** `Mode` includes `'solo'`; `useGameSession` returns `…, statusText, endText, getHint`; `GamePlay` reads them; 2048 handle implements `statusText/endText`.
