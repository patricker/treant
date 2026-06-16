# Arcade Foundation (Phase 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** A mobile-first playful game arcade at `/arcade` with two grid games (Connect Four, Tic-Tac-Toe), three play modes (pass-and-play, vs-AI w/ difficulty, watch), quick-play + wild presets, and a locked setup→play→game-over flow.

**Architecture:** Client-side React in the existing Docusaurus site. A reusable `GameDefinition` boundary + a `useGameSession` driver hook power a screen state machine (`launcher → setup → playing`). AI difficulty is epsilon-greedy over the Phase-0 WASM classes (no Rust changes).

**Tech Stack:** React 19, Docusaurus, TypeScript, `treant-wasm` (`ConnectFourWasm`, `TicTacToeWasm`). Spec: `specs/2026-06-15-arcade-foundation-phase1-design.md`.

---

## File Structure
- `docs/src/components/arcade/gameTypes.ts` — types + pure helpers (`seatTypes`, `DIFFICULTY`, `pickAiMove`).
- `docs/src/components/arcade/games/connectFour.tsx` / `ticTacToe.tsx` / `index.ts` — game definitions + registry.
- `docs/src/components/arcade/useGameSession.ts` — driver hook.
- `docs/src/components/arcade/ArcadeShell.tsx` — screen state machine.
- `docs/src/components/arcade/Launcher.tsx` / `GameSetup.tsx` / `GamePlay.tsx` — screens.
- `docs/src/components/arcade/controls/{ModePicker,DifficultyPicker,PresetChips,CustomKnobs}.tsx` — shared controls.
- `docs/src/components/arcade/arcade.module.css` — playful skin.
- `docs/src/pages/arcade.tsx` — the route.

---

## Task 1: Types and pure helpers

**Files:** Create `docs/src/components/arcade/gameTypes.ts`

- [ ] **Step 1: Write `gameTypes.ts`**
```ts
export type GameParams = { cols: number; rows: number; k: number; numPlayers: number };
export type Mode = 'pvp' | 'pvai' | 'aivai';
export type Difficulty = 'easy' | 'medium' | 'hard';
export type SeatType = 'human' | 'ai';

export interface Preset { label: string; emoji?: string; params: GameParams; }
export interface Knob { key: keyof GameParams; label: string; min: number; max: number; step: number; }

export interface GameHandle {
  applyMove(move: string): boolean;
  getBoard(): string;
  currentPlayer(): number;
  isTerminal(): boolean;
  result(): string;
  bestMove(): string | undefined;
  playoutN(n: number): void;
  free(): void;
}

export interface BoardProps {
  board: string;
  params: GameParams;
  currentPlayer: number;
  interactive: boolean;            // true only when it's a human seat's turn
  onMove: (move: string) => void;
}

export interface GameDefinition {
  id: string;
  name: string;
  icon: string;
  blurb: string;
  defaultParams: GameParams;
  presets: Preset[];
  knobs: Knob[];
  create(wasm: any, p: GameParams): GameHandle;
  legalMoves(board: string, p: GameParams): string[];
  renderBoard(props: BoardProps): JSX.Element;
}

export const DIFFICULTY: Record<Difficulty, { playouts: number; epsilon: number; label: string; emoji: string }> = {
  easy:   { playouts: 200,   epsilon: 0.5, label: 'Easy',   emoji: '😊' },
  medium: { playouts: 2000,  epsilon: 0.1, label: 'Medium', emoji: '😎' },
  hard:   { playouts: 10000, epsilon: 0.0, label: 'Hard',   emoji: '🔥' },
};

export function seatTypes(mode: Mode, numPlayers: number): SeatType[] {
  if (mode === 'pvp') return Array(numPlayers).fill('human');
  if (mode === 'aivai') return Array(numPlayers).fill('ai');
  return ['human', ...Array(Math.max(0, numPlayers - 1)).fill('ai')] as SeatType[];
}

/** Epsilon-greedy AI move. `rng` injectable for tests (defaults to Math.random). */
export function pickAiMove(
  handle: GameHandle,
  def: GameDefinition,
  params: GameParams,
  diff: Difficulty,
  rng: () => number = Math.random,
): string | undefined {
  const { playouts, epsilon } = DIFFICULTY[diff];
  const legal = def.legalMoves(handle.getBoard(), params);
  if (legal.length === 0) return undefined;
  if (rng() < epsilon) return legal[Math.floor(rng() * legal.length)];
  handle.playoutN(playouts);
  return handle.bestMove() ?? legal[0];
}
```

- [ ] **Step 2: Type-check**

Run: `cd docs && npx tsc --noEmit -p tsconfig.json 2>&1 | head` (expect no errors referencing gameTypes; JSX type needs `import type {JSX}` only if tsconfig is strict — if it errors on `JSX`, add `import type { JSX } from 'react';`).

- [ ] **Step 3: Commit**
```bash
git add docs/src/components/arcade/gameTypes.ts
git commit -m "feat(arcade): game types + seat/difficulty/epsilon helpers"
```

---

## Task 2: Game definitions (Connect Four, Tic-Tac-Toe) + registry

**Files:** Create `games/connectFour.tsx`, `games/ticTacToe.tsx`, `games/index.ts`

- [ ] **Step 1: `games/connectFour.tsx`**

Board string is top-row-first, `' '`=empty, `'1'..'4'`=players. Gravity moves = column index. `renderBoard` shows column-drop buttons + the grid.
```tsx
import type { GameDefinition, GameHandle, GameParams, BoardProps } from '../gameTypes';
import styles from '../arcade.module.css';

const DISC = ['', 'var(--arc-p1)', 'var(--arc-p2)', 'var(--arc-p3)', 'var(--arc-p4)'];

function handle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.ConnectFourWasm(p.cols, p.rows, p.k, p.numPlayers);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    free: () => g.free(),
  };
}

export const connectFour: GameDefinition = {
  id: 'connect-four',
  name: 'Connect Four',
  icon: '🔴',
  blurb: 'Drop discs, line up four. Or seven. With four players.',
  defaultParams: { cols: 7, rows: 6, k: 4, numPlayers: 2 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { cols: 7, rows: 6, k: 4, numPlayers: 2 } },
    { label: 'Connect-5', emoji: '🖐️', params: { cols: 9, rows: 7, k: 5, numPlayers: 2 } },
    { label: '4-Player Frenzy', emoji: '🎉', params: { cols: 9, rows: 8, k: 4, numPlayers: 4 } },
    { label: 'Giant', emoji: '🦣', params: { cols: 10, rows: 10, k: 5, numPlayers: 2 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 3, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 3, max: 10, step: 1 },
    { key: 'k', label: 'In a row', min: 3, max: 10, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 4, step: 1 },
  ],
  create: handle,
  legalMoves: (board, p) => {
    // a column is legal if its TOP cell (first row in the string) is empty
    const out: string[] = [];
    for (let c = 0; c < p.cols; c++) if (board[c] === ' ') out.push(String(c));
    return out;
  },
  renderBoard: ({ board, params, interactive, onMove }: BoardProps) => {
    const { cols, rows } = params;
    const cells = Array.from({ length: cols * rows }, (_, i) => board[i] ?? ' ');
    const legalCols = new Set<number>();
    for (let c = 0; c < cols; c++) if (board[c] === ' ') legalCols.add(c);
    return (
      <div className={styles.cfBoard}>
        <div className={styles.cfColHeaders} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
          {Array.from({ length: cols }, (_, c) => (
            <button key={c} className={styles.cfColButton}
              disabled={!interactive || !legalCols.has(c)}
              onClick={() => onMove(String(c))} aria-label={`Drop in column ${c + 1}`}>▾</button>
          ))}
        </div>
        <div className={styles.cfGrid} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
          {cells.map((ch, i) => {
            const p = ch === ' ' ? 0 : Number(ch);
            return <div key={i} className={styles.cfCell}>
              <span className={styles.cfDisc} style={{ background: p ? DISC[p] : 'transparent' }} />
            </div>;
          })}
        </div>
      </div>
    );
  },
};
```

- [ ] **Step 2: `games/ticTacToe.tsx`**

Board string row-major, `' '`=empty, symbols `X O A B`. Placement moves = cell index. `renderBoard` shows a grid of cell buttons.
```tsx
import type { GameDefinition, GameHandle, GameParams, BoardProps } from '../gameTypes';
import styles from '../arcade.module.css';

const SYM = [' ', 'X', 'O', 'A', 'B'];
const COLOR: Record<string, string> = { X: 'var(--arc-p1)', O: 'var(--arc-p2)', A: 'var(--arc-p3)', B: 'var(--arc-p4)' };

function handle(wasm: any, p: GameParams): GameHandle {
  const g = new wasm.TicTacToeWasm(p.cols, p.rows, p.k, p.numPlayers);
  return {
    applyMove: (m) => g.apply_move(m),
    getBoard: () => g.get_board(),
    currentPlayer: () => g.current_player(),
    isTerminal: () => g.is_terminal(),
    result: () => g.result(),
    bestMove: () => g.best_move() ?? undefined,
    playoutN: (n) => g.playout_n(n),
    free: () => g.free(),
  };
}

export const ticTacToe: GameDefinition = {
  id: 'tic-tac-toe',
  name: 'Tic-Tac-Toe',
  icon: '⭕',
  blurb: 'Classic 3×3 — or a giant 5-in-a-row brain-bender.',
  defaultParams: { cols: 3, rows: 3, k: 3, numPlayers: 2 },
  presets: [
    { label: 'Classic', emoji: '⭐', params: { cols: 3, rows: 3, k: 3, numPlayers: 2 } },
    { label: 'Gomoku-lite', emoji: '🧠', params: { cols: 9, rows: 9, k: 5, numPlayers: 2 } },
    { label: 'Big Board', emoji: '🔲', params: { cols: 6, rows: 6, k: 4, numPlayers: 2 } },
    { label: '3-Player', emoji: '👨‍👩‍👦', params: { cols: 6, rows: 6, k: 4, numPlayers: 3 } },
  ],
  knobs: [
    { key: 'cols', label: 'Width', min: 2, max: 10, step: 1 },
    { key: 'rows', label: 'Height', min: 2, max: 10, step: 1 },
    { key: 'k', label: 'In a row', min: 2, max: 10, step: 1 },
    { key: 'numPlayers', label: 'Players', min: 2, max: 4, step: 1 },
  ],
  create: handle,
  legalMoves: (board, p) => {
    const out: string[] = [];
    for (let i = 0; i < p.cols * p.rows; i++) if ((board[i] ?? ' ') === ' ') out.push(String(i));
    return out;
  },
  renderBoard: ({ board, params, interactive, onMove }: BoardProps) => {
    const { cols, rows } = params;
    return (
      <div className={styles.tttGrid} style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}>
        {Array.from({ length: cols * rows }, (_, i) => {
          const ch = board[i] ?? ' ';
          return (
            <button key={i} className={styles.tttCell}
              disabled={!interactive || ch !== ' '}
              onClick={() => onMove(String(i))}
              style={{ color: COLOR[ch] }}
              aria-label={`Cell ${i + 1}: ${ch === ' ' ? 'empty' : ch}`}>{ch === ' ' ? '' : ch}</button>
          );
        })}
      </div>
    );
  },
};
```

- [ ] **Step 3: `games/index.ts`**
```ts
import { connectFour } from './connectFour';
import { ticTacToe } from './ticTacToe';
import type { GameDefinition } from '../gameTypes';
export const GAMES: GameDefinition[] = [connectFour, ticTacToe];
export function gameById(id: string): GameDefinition | undefined {
  return GAMES.find((g) => g.id === id);
}
```

- [ ] **Step 4: Commit**
```bash
git add docs/src/components/arcade/games
git commit -m "feat(arcade): Connect Four + Tic-Tac-Toe game definitions"
```

---

## Task 3: `useGameSession` driver hook

**Files:** Create `docs/src/components/arcade/useGameSession.ts`

- [ ] **Step 1: Write the hook**
```ts
import { useCallback, useEffect, useRef, useState } from 'react';
import type { Difficulty, GameDefinition, GameHandle, GameParams, Mode } from './gameTypes';
import { pickAiMove, seatTypes } from './gameTypes';

type Phase = 'playing' | 'thinking' | 'over';
const AI_DELAY_MS = 400;

export function useGameSession(
  wasm: any,
  def: GameDefinition,
  params: GameParams,
  mode: Mode,
  difficulty: Difficulty,
) {
  const handleRef = useRef<GameHandle | null>(null);
  const seats = seatTypes(mode, params.numPlayers);
  const seatsRef = useRef(seats);
  seatsRef.current = seats;

  const [board, setBoard] = useState('');
  const [current, setCurrent] = useState(0);
  const [phase, setPhase] = useState<Phase>('playing');
  const [result, setResult] = useState('');

  const sync = useCallback(() => {
    const h = handleRef.current!;
    setBoard(h.getBoard());
    setCurrent(h.currentPlayer());
    if (h.isTerminal()) { setResult(h.result()); setPhase('over'); }
  }, []);

  const runAiTurn = useCallback(() => {
    setPhase('thinking');
    setTimeout(() => {
      const h = handleRef.current;
      if (!h || h.isTerminal()) return;
      const mv = pickAiMove(h, def, params, difficulty);
      if (mv != null) h.applyMove(mv);
      const terminal = h.isTerminal();
      setBoard(h.getBoard()); setCurrent(h.currentPlayer());
      if (terminal) { setResult(h.result()); setPhase('over'); return; }
      if (seatsRef.current[h.currentPlayer()] === 'ai') { runAiTurn(); }
      else setPhase('playing');
    }, AI_DELAY_MS);
  }, [def, params, difficulty]);

  const start = useCallback(() => {
    if (handleRef.current) handleRef.current.free();
    handleRef.current = def.create(wasm, params);
    setResult(''); setPhase('playing');
    setBoard(handleRef.current.getBoard());
    setCurrent(handleRef.current.currentPlayer());
    if (seatsRef.current[handleRef.current.currentPlayer()] === 'ai') runAiTurn();
  }, [wasm, def, params, runAiTurn]);

  useEffect(() => {
    start();
    return () => { if (handleRef.current) { handleRef.current.free(); handleRef.current = null; } };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const onHumanMove = useCallback((move: string) => {
    const h = handleRef.current;
    if (!h || phase !== 'playing') return;
    if (seatsRef.current[h.currentPlayer()] !== 'human') return;
    if (!h.applyMove(move)) return;
    if (h.isTerminal()) { setBoard(h.getBoard()); setResult(h.result()); setPhase('over'); return; }
    setBoard(h.getBoard()); setCurrent(h.currentPlayer());
    if (seatsRef.current[h.currentPlayer()] === 'ai') runAiTurn();
  }, [phase, runAiTurn]);

  return { board, current, phase, result, seats, onHumanMove, replay: start };
}
```

- [ ] **Step 2: Commit**
```bash
git add docs/src/components/arcade/useGameSession.ts
git commit -m "feat(arcade): useGameSession driver (turn loop, AI seats, epsilon-greedy)"
```

---

## Task 4: Shared controls + the playful CSS

**Files:** Create `controls/ModePicker.tsx`, `controls/DifficultyPicker.tsx`, `controls/PresetChips.tsx`, `controls/CustomKnobs.tsx`, `arcade.module.css`

- [ ] **Step 1: `arcade.module.css`** — playful skin. Define CSS vars at `.arcade`:
  `--arc-p1:#ff3b5c; --arc-p2:#ffd23f; --arc-p3:#2f9e6f; --arc-p4:#7c5cff;` plus gradient headers, chunky rounded `.tab/.chip/.segBtn/.playBtn`, big touch targets, `.tttGrid/.tttCell`, `.cfBoard/.cfGrid/.cfCell/.cfDisc/.cfColButton`, `.launcherGrid/.gameTile`, `.overlay`. Mobile-first (max-width container, large tap sizes ≥44px). Use `var(--ifm-*)` only for neutrals; arcade accents are the bright vars above.

- [ ] **Step 2: `ModePicker.tsx`** — segmented control of `pvp/pvai/aivai`:
```tsx
import type { Mode } from '../gameTypes';
import styles from '../arcade.module.css';
const OPTS: { id: Mode; label: string; emoji: string }[] = [
  { id: 'pvp', label: '2 of us', emoji: '👥' },
  { id: 'pvai', label: 'vs AI', emoji: '🤖' },
  { id: 'aivai', label: 'Watch', emoji: '👀' },
];
export default function ModePicker({ value, onChange }: { value: Mode; onChange: (m: Mode) => void }) {
  return <div className={styles.seg}>{OPTS.map((o) => (
    <button key={o.id} className={`${styles.segBtn} ${value === o.id ? styles.segOn : ''}`}
      onClick={() => onChange(o.id)}>{o.emoji} {o.label}</button>))}</div>;
}
```

- [ ] **Step 3: `DifficultyPicker.tsx`** — segmented `easy/medium/hard` from `DIFFICULTY`:
```tsx
import type { Difficulty } from '../gameTypes';
import { DIFFICULTY } from '../gameTypes';
import styles from '../arcade.module.css';
export default function DifficultyPicker({ value, onChange }: { value: Difficulty; onChange: (d: Difficulty) => void }) {
  return <div className={styles.seg}>{(Object.keys(DIFFICULTY) as Difficulty[]).map((d) => (
    <button key={d} className={`${styles.segBtn} ${value === d ? styles.segOn : ''}`}
      onClick={() => onChange(d)}>{DIFFICULTY[d].emoji} {DIFFICULTY[d].label}</button>))}</div>;
}
```

- [ ] **Step 4: `PresetChips.tsx`** — tappable preset chips:
```tsx
import type { GameParams, Preset } from '../gameTypes';
import styles from '../arcade.module.css';
export default function PresetChips({ presets, active, onPick }:
  { presets: Preset[]; active: GameParams; onPick: (p: GameParams) => void }) {
  const same = (a: GameParams, b: GameParams) => a.cols === b.cols && a.rows === b.rows && a.k === b.k && a.numPlayers === b.numPlayers;
  return <div className={styles.chipRow}>{presets.map((p) => (
    <button key={p.label} className={`${styles.chip} ${same(active, p.params) ? styles.chipOn : ''}`}
      onClick={() => onPick(p.params)}>{p.emoji} {p.label}</button>))}</div>;
}
```

- [ ] **Step 5: `CustomKnobs.tsx`** — steppers for each knob:
```tsx
import type { GameParams, Knob } from '../gameTypes';
import styles from '../arcade.module.css';
export default function CustomKnobs({ knobs, params, onChange }:
  { knobs: Knob[]; params: GameParams; onChange: (p: GameParams) => void }) {
  return <div className={styles.knobs}>{knobs.map((kn) => {
    const v = params[kn.key];
    const set = (nv: number) => onChange({ ...params, [kn.key]: Math.max(kn.min, Math.min(kn.max, nv)) });
    return <div key={kn.key} className={styles.knob}>
      <span className={styles.knobLabel}>{kn.label}</span>
      <div className={styles.stepper}>
        <button onClick={() => set(v - kn.step)} disabled={v <= kn.min}>−</button>
        <span className={styles.knobVal}>{v}</span>
        <button onClick={() => set(v + kn.step)} disabled={v >= kn.max}>+</button>
      </div>
    </div>;
  })}</div>;
}
```

- [ ] **Step 6: Commit**
```bash
git add docs/src/components/arcade/controls docs/src/components/arcade/arcade.module.css
git commit -m "feat(arcade): playful skin + mode/difficulty/preset/knob controls"
```

---

## Task 5: Screens + shell + route

**Files:** Create `Launcher.tsx`, `GameSetup.tsx`, `GamePlay.tsx`, `ArcadeShell.tsx`, `docs/src/pages/arcade.tsx`

- [ ] **Step 1: `Launcher.tsx`** — grid of game tiles (2 real + "coming soon"):
```tsx
import { GAMES } from './games';
import styles from './arcade.module.css';
export default function Launcher({ onPick }: { onPick: (id: string) => void }) {
  return (
    <div className={styles.launcher}>
      <h1 className={styles.arcadeTitle}>🌳 Treant Arcade</h1>
      <p className={styles.arcadeSub}>Pass-and-play or take on the AI. Crank the knobs and make it weird.</p>
      <div className={styles.launcherGrid}>
        {GAMES.map((g) => (
          <button key={g.id} className={styles.gameTile} onClick={() => onPick(g.id)}>
            <span className={styles.tileIcon}>{g.icon}</span>
            <span className={styles.tileName}>{g.name}</span>
            <span className={styles.tileBlurb}>{g.blurb}</span>
          </button>
        ))}
        <div className={`${styles.gameTile} ${styles.tileSoon}`}><span className={styles.tileIcon}>🚧</span><span className={styles.tileName}>More soon</span></div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: `GameSetup.tsx`** — mode/difficulty/presets/custom + Start:
```tsx
import { useState } from 'react';
import type { Difficulty, GameDefinition, GameParams, Mode } from './gameTypes';
import ModePicker from './controls/ModePicker';
import DifficultyPicker from './controls/DifficultyPicker';
import PresetChips from './controls/PresetChips';
import CustomKnobs from './controls/CustomKnobs';
import styles from './arcade.module.css';

export default function GameSetup({ def, onStart, onBack }:
  { def: GameDefinition; onStart: (cfg: { params: GameParams; mode: Mode; difficulty: Difficulty }) => void; onBack: () => void }) {
  const [mode, setMode] = useState<Mode>('pvai');
  const [difficulty, setDifficulty] = useState<Difficulty>('medium');
  const [params, setParams] = useState<GameParams>(def.defaultParams);
  const [showCustom, setShowCustom] = useState(false);
  return (
    <div className={styles.setup}>
      <div className={styles.setupHeader}><button className={styles.backBtn} onClick={onBack}>←</button><h2>{def.icon} {def.name}</h2></div>
      <div className={styles.setupLabel}>Who's playing?</div>
      <ModePicker value={mode} onChange={setMode} />
      {mode !== 'pvp' && (<><div className={styles.setupLabel}>AI strength</div><DifficultyPicker value={difficulty} onChange={setDifficulty} /></>)}
      <div className={styles.setupLabel}>Board</div>
      <PresetChips presets={def.presets} active={params} onPick={setParams} />
      <button className={styles.customToggle} onClick={() => setShowCustom((s) => !s)}>{showCustom ? '▾' : '▸'} Customize</button>
      {showCustom && <CustomKnobs knobs={def.knobs} params={params} onChange={setParams} />}
      <button className={styles.playBtn} onClick={() => onStart({ params, mode, difficulty })}>▶ Start game</button>
    </div>
  );
}
```

- [ ] **Step 3: `GamePlay.tsx`** — board + status + quit + game-over overlay (uses `useGameSession`):
```tsx
import type { Difficulty, GameDefinition, GameParams, Mode } from './gameTypes';
import { useGameSession } from './useGameSession';
import styles from './arcade.module.css';

const PLAYER_LABEL = ['Red', 'Yellow', 'Green', 'Purple'];

export default function GamePlay({ wasm, def, params, mode, difficulty, onQuit, onChangeSetup }:
  { wasm: any; def: GameDefinition; params: GameParams; mode: Mode; difficulty: Difficulty; onQuit: () => void; onChangeSetup: () => void }) {
  const s = useGameSession(wasm, def, params, mode, difficulty);
  const interactive = s.phase === 'playing' && s.seats[s.current] === 'human';
  const status = s.phase === 'thinking' ? '🤖 Thinking…'
    : s.phase === 'over' ? '' : `${PLAYER_LABEL[s.current]}'s turn`;
  return (
    <div className={styles.play}>
      <div className={styles.playTop}><span>{def.icon} {def.name}</span><button className={styles.quitBtn} onClick={onQuit} aria-label="Quit">✕</button></div>
      {status && <div className={styles.turnBanner}>{status}</div>}
      <div className={styles.boardWrap}>
        {def.renderBoard({ board: s.board, params, currentPlayer: s.current, interactive, onMove: s.onHumanMove })}
        {s.phase === 'over' && (
          <div className={styles.overlay}>
            <div className={styles.overlayCard}>
              <div className={styles.overlayIcon}>{s.result === 'Draw' ? '🤝' : '🏆'}</div>
              <div className={styles.overlayText}>{s.result === 'Draw' ? "It's a draw!" : `${PLAYER_LABEL[Number(s.result) - 1] ?? s.result} wins!`}</div>
              <button className={styles.playBtn} onClick={s.replay}>↺ Play again</button>
              <button className={styles.secondaryBtn} onClick={onChangeSetup}>⚙ Change setup</button>
              <button className={styles.ghostBtn} onClick={onQuit}>🏠 Arcade</button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 4: `ArcadeShell.tsx`** — state machine:
```tsx
import { useState } from 'react';
import { gameById } from './games';
import type { Difficulty, GameParams, Mode } from './gameTypes';
import Launcher from './Launcher';
import GameSetup from './GameSetup';
import GamePlay from './GamePlay';
import styles from './arcade.module.css';

type Screen =
  | { name: 'launcher' }
  | { name: 'setup'; gameId: string }
  | { name: 'playing'; gameId: string; params: GameParams; mode: Mode; difficulty: Difficulty };

export default function ArcadeShell({ wasm }: { wasm: any }) {
  const [screen, setScreen] = useState<Screen>({ name: 'launcher' });
  if (screen.name === 'launcher')
    return <div className={styles.arcade}><Launcher onPick={(gameId) => setScreen({ name: 'setup', gameId })} /></div>;
  const def = gameById(screen.gameId)!;
  if (screen.name === 'setup')
    return <div className={styles.arcade}><GameSetup def={def}
      onBack={() => setScreen({ name: 'launcher' })}
      onStart={({ params, mode, difficulty }) => setScreen({ name: 'playing', gameId: screen.gameId, params, mode, difficulty })} /></div>;
  return <div className={styles.arcade}><GamePlay wasm={wasm} def={def}
    params={screen.params} mode={screen.mode} difficulty={screen.difficulty}
    onQuit={() => setScreen({ name: 'launcher' })}
    onChangeSetup={() => setScreen({ name: 'setup', gameId: screen.gameId })} /></div>;
}
```

- [ ] **Step 5: `docs/src/pages/arcade.tsx`** — the route:
```tsx
import Layout from '@theme/Layout';
import BrowserOnly from '@docusaurus/BrowserOnly';

export default function ArcadePage(): JSX.Element {
  return (
    <Layout title="Arcade" description="Pass-and-play family games powered by treant MCTS">
      <BrowserOnly fallback={<div style={{ padding: 40, textAlign: 'center' }}>Loading arcade…</div>}>
        {() => {
          const { WasmProvider, useWasm } = require('@site/src/components/treant/WasmProvider');
          const ArcadeShell = require('@site/src/components/arcade/ArcadeShell').default;
          function Inner() {
            const { wasm, ready, error } = useWasm();
            if (error) return <div style={{ padding: 40 }}>Failed to load: {String(error)}</div>;
            if (!ready) return <div style={{ padding: 40, textAlign: 'center' }}>Loading arcade…</div>;
            return <ArcadeShell wasm={wasm} />;
          }
          return <WasmProvider><Inner /></WasmProvider>;
        }}
      </BrowserOnly>
    </Layout>
  );
}
```

- [ ] **Step 6: Add an "Arcade" navbar link** in `docs/docusaurus.config.ts` items (after Playground): `{ to: '/arcade', label: 'Arcade', position: 'left' }`.

- [ ] **Step 7: Build check**

Run: `cd docs && npm run build 2>&1 | tail -20`
Expected: build succeeds (route `/arcade` emitted).

- [ ] **Step 8: Commit**
```bash
git add docs/src/components/arcade docs/src/pages/arcade.tsx docs/docusaurus.config.ts
git commit -m "feat(arcade): launcher/setup/play screens, shell, and /arcade route"
```

---

## Task 6: End-to-end verification

**Files:** none (verification only). Rebuild WASM if needed (Phase 0 already built it).

- [ ] **Step 1: Start dev server on a free high port**

Run (background): `cd docs && npm start -- --port 3939 --no-open`. Wait for "compiled successfully".

- [ ] **Step 2: Drive with Playwright** — verify each acceptance case, checking no console errors after each:
  1. Navigate `/arcade` → launcher shows **Connect Four** + **Tic-Tac-Toe** tiles.
  2. Connect Four → setup → **2 of us** (pvp) → Start → drop discs alternately down columns to build a vertical four → overlay shows a winner.
  3. Connect Four → setup → **vs AI** + **Easy** → Start → one human drop → AI responds with a legal disc (board gains an AI piece).
  4. Tic-Tac-Toe → setup → **Watch** (aivai) → Start → board auto-fills to a terminal (overlay appears) with no human input.
  5. Connect Four → setup → **Connect-5** preset → Start → board is 9×7 and playable.
  Use `browser_evaluate` for clicks (the demos re-render; bypass stability waits) and to read board/overlay state.

- [ ] **Step 3: Stop the dev server.**

- [ ] **Step 4: Final commit (if cleanup needed)**
```bash
git add -A && git commit -m "test(arcade): e2e-verified launcher, modes, presets" || echo "nothing to commit"
```

---

## Self-Review Notes
- **Spec coverage:** route + playful skin (Tasks 4–5), `GameDefinition`/`useGameSession` boundary (Tasks 2–3), modes via `seatTypes` (Task 1/3), epsilon-greedy difficulty (Task 1/3), quick-play + presets + custom (Tasks 4–5), locked setup→play→over flow (Task 5), two grid games (Task 2), presets respecting players≤4 (Task 2), verification across modes/games (Task 6). Mapped.
- **Type consistency:** `GameParams`, `GameHandle`, `GameDefinition`, `Mode`, `Difficulty`, `seatTypes`, `pickAiMove`, `DIFFICULTY`, `useGameSession` returns `{board,current,phase,result,seats,onHumanMove,replay}` used consistently across Tasks 3–5.
- **Constraint check:** all presets use `numPlayers ≤ 4`; CF k≤10/dims 3–10, TTT dims 2–10 — within WASM clamps.
