# Building Treant Arcade games

> How the arcade is put together, how we think about UX and "knobs", and the
> exact recipe + checklist for adding a new game. Read this before adding or
> changing a game. Companion to [`GAME-IDEAS.md`](GAME-IDEAS.md) (the backlog).

The arcade lives at **mcts.dev/arcade** — a free, static, family game arcade
where every game is playable pass-and-play, vs the treant AI (Easy/Medium/Hard),
or watch-AI. It doubles as the showcase for the `treant` MCTS library: **every
opponent is treant searching the game tree.**

---

## 1. The two layers

Every game is two thin pieces with a string-based boundary between them:

```
treant-wasm/src/<game>.rs      Rust: rules + a wasm-bindgen class   (the engine + AI)
        │  strings over wasm-bindgen ("X O  X", "3-7", "12", …)
docs/src/components/arcade/games/<game>.tsx   React: a GameDefinition   (the UI)
```

The Rust side owns **rules and search**; the React side owns **presentation and
interaction**. They never share types — only flat strings (board state, move
encodings, results). That keeps the boundary trivial to reason about and means
the same engine can back a CLI, a golden test, or the web UI unchanged.

**Rust engine** (`treant-wasm/src/<game>.rs`):
- A `GameState` implementing the rules (`treant`'s core trait).
- An `Evaluator` + `MCTS` config (usually trivial — pure rollouts + the solver).
- A `#[wasm_bindgen]` class exposing the handle methods the UI calls.

**React definition** (`docs/src/components/arcade/games/<game>.tsx`):
- A `GameDefinition` (metadata + presets + knobs + how to build a handle + a Board).
- Usually reuses a shared Board component; sometimes a small custom one.

---

## 2. The engine contract (`<game>.rs`)

### GameState

```rust
impl GameState for MyGame {
    type Move = u16;          // or a small Copy struct with a Display impl
    type Player = u8;         // seat index, 0-based
    type MoveList = Vec<Move>;
    fn current_player(&self) -> u8 { self.current }
    fn available_moves(&self) -> Vec<Move> { self.gen() }
    fn make_move(&mut self, m: &Move) { /* mutate + flip current */ }
    fn terminal_value(&self) -> Option<ProvenValue> { self.term() }
}
```

`terminal_value` returns `Some(...)` **from the perspective of the player to
move** (negamax). Get the sign right — it's the #1 source of subtle bugs:

| Game shape | When it ends | What the player-to-move sees |
|---|---|---|
| Line-to-win (Connect Four, Gomoku) | previous player completed a line | `Loss` |
| Misère (No-Tac-Toe, Sim, Trap-Three's 3-in-a-row) | previous player made the bad shape | `Win` |
| Last-to-move-wins (Nim, Kōnane, Domineering, Col) | current player has no move | `Loss` |
| Race (Frontline, Fox & Hounds) | someone reached the goal | depends who — compute it |

Write a Rust unit test that asserts the verdict for a hand-built terminal
position. We always do this; it has caught real sign errors.

### Evaluator + MCTS config

For almost every game the evaluator is a no-op (priors uniform, value 0) — the
AI strength comes from MCTS rollouts + the exact solver, not a heuristic:

```rust
fn evaluate_new_state(&self, _, m: &Vec<Move>, _) -> (Vec<()>, i64) { (vec![(); m.len()], 0) }
fn interpret_evaluation_for_player(&self, e: &i64, _) -> i64 { *e }
fn evaluate_existing_state(&self, _, _, _) -> i64 { 0 }
```

```rust
impl MCTS for MyCfg {
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool { true }   // exact endgame solving for small games
}
```

`solver_enabled()` turns on Score-Bounded MCTS / exact solving — leave it on for
finite games; it makes small games perfect and decisive ones sharp.

### ⚠️ Every playout must terminate

MCTS does **random rollouts to a terminal state**. If a random line of play can
loop forever, a single playout hangs the browser. This is the hard constraint
that shapes which games we pick:

- **Safe:** placement games (the board fills), removal/capture games (pieces
  strictly decrease), monotonic-progress games (Fox & Hounds: hounds only ever
  advance), bounded-value games (Euclid: the larger number strictly shrinks).
- **Dangerous:** pure sliding games with no progress measure (a piece can
  shuffle back and forth). Mu Tōrere is tiny enough that the solver resolves it,
  but in general prefer a mechanic that *provably* terminates. If you must ship a
  cycling game, add a ply cap that declares a draw.

This is why the overnight batch leaned on placement / capture / shoot-and-burn
mechanics — they can't cycle, so AI-vs-AI "Watch" mode is always safe.

### The wasm-bindgen class

A fixed surface the UI relies on (names matter — the handles call them):

```rust
#[wasm_bindgen]
impl MyGameWasm {
    #[wasm_bindgen(constructor)] pub fn new(/* params */) -> Self
    pub fn playout_n(&mut self, n: u32)          // run n MCTS iterations
    pub fn get_board(&self) -> String            // one char/cell, ' '=empty, 'X'/'O'/'A'/'B'/'C'/'D'
    pub fn current_player(&self) -> u32
    pub fn is_terminal(&self) -> bool
    pub fn result(&self) -> String               // winner seat 1-indexed, "Draw", or "" if unfinished
    pub fn legal_moves(&self) -> String          // comma-joined move encodings (for move-based games)
    pub fn best_move(&self) -> Option<String>
    pub fn apply_move(&mut self, mov: &str) -> bool   // validate, apply, REBUILD the manager (fresh tree)
    pub fn reset(&mut self)
}
```

`apply_move` validates against `available_moves()` and, on success, **rebuilds
`MCTSManager` from the new root state** — we throw away the old tree each move
rather than re-root it. Simple and correct; tree reuse isn't worth the
complexity at these search sizes.

Board/move **string encodings** by convention:
- Board: `' '` empty, `'X' 'O' 'A' 'B' 'C' 'D'` for seats 0–5 (or digits for
  Connect Four), `'#'` for a blocked/burnt cell (Amazons), etc.
- Move: a single cell index (`"12"`), a `from-to` pair (`"3-7"`), a `from-to-arrow`
  triple (Amazons), or a domain token (`"Roll"`, `"Take 4"`).

### Player count lives in the engine

Multiplayer games clamp to `MAX_PLAYERS` (currently **6**) and index a
`PLAYER_SYMBOLS = ['X','O','A','B','C','D']` array. To change the cap you touch
the const + symbol array in each multiplayer engine (`tictactoe`, `connectfour`,
`shift`, `mancala`, `pig`) **and** the UI colour/label/symbol maps (§5).

---

## 3. The React definition (`<game>.tsx`)

```ts
export interface GameDefinition {
  id: string;            // kebab-case, stable — used in URLs, icons, rules
  name: string;          // display name (non-copyrighted! see §6)
  icon: string;          // emoji fallback; real games get an SVG in icons.tsx
  blurb: string;         // one-liner on the tile/hero
  rules?: string;        // full how-to-play (else falls back to rules.ts map, then blurb)
  defaultParams: GameParams;          // { numPlayers, ...game-specific }
  presets: Preset[];     // quick-play + "go crazy" — see §4
  knobs: Knob[];         // { key, label, min, max, step } sliders behind "Customize"
  create(wasm, p): GameHandle;        // build the handle from params
  Board: ComponentType<BoardProps>;   // the board renderer
  solo?: boolean;        // single-player flow (2048): You-play + Hint / Watch-AI
  formatHint?(m): string;
  moveSound?: 'move' | 'drop';
  playerLabels?: string[];            // must match the rendered piece colours!
}
```

### The handle (UI ↔ engine adapter)

`GameHandle` wraps the wasm class with the methods `useGameSession` calls
(`applyMove`, `getBoard`, `currentPlayer`, `isTerminal`, `result`, `bestMove`,
`playoutN`, `legalMoves`, `free`, optional `statusText`/`endText`). Don't
hand-roll it — pick a shared one:

- **`cellHandle(g, cols, rows)`** (`gridpack.tsx`) — "place a mark on an empty
  cell". `legalMoves` = every empty cell. For games where *any* empty cell is
  legal (TTT, Connect Six, Notakto).
- **`moveHandle(g)`** (`frontline.tsx`) — moves come from the engine's
  `legal_moves()`. Use whenever legality is non-trivial: movement (`from-to`),
  player-dependent placement (Domineering), or constrained placement (NoGo, Col
  — where legal cells are a *subset* of empty cells).
- A **custom handle** only when the state isn't a board string (Nim/Euclid expose
  a count/pair; the handle adapts `getBoard`/`legalMoves` accordingly).

### The Board (`BoardProps → JSX`)

`BoardProps = { board, params, currentPlayer, interactive, legalMoves, onMove }`.
The board is **pure**: it renders `board`, highlights `legalMoves`, and calls
`onMove(encoded)`. It never talks to the engine or decides turns — the session
does. Reuse before building:

- **`MarkGridBoard`** (`gridpack.tsx`) — tap an empty cell to place. Grid games.
- **`makeMoveBoard('pawn' | 'disc')` / `MoveBoard`** (`frontline.tsx`) —
  select-then-move using `legalMoves` for target highlights. Movement/capture
  games. `Piece` renders a pawn or stone coloured by seat.
- **A custom board** for non-grid geometry: SVG hexagons (Hex), an SVG hexagon of
  edges (Sim), absolutely-positioned points (Mu Tōrere's star), a pile + buttons
  (Nim, Square Subtract), two number tiles (Euclid).

**Board rules that are easy to forget:**
- Selection state (`sel`/`from`) must reset when the board changes:
  `useEffect(() => setSel(null), [board])`. Otherwise a stale highlight points at
  the wrong cell after the opponent moves. (`board` is a string, so the effect
  only fires on a real move.)
- Boards must **fill width**, not size to content — see §7.
- Colour pieces by seat using `--arc-p1..p6`; never hard-code hexes.

### Registration

1. Add the game to the `GAMES` array in `games/index.ts`.
2. Slot its `id` into a category in `Launcher.tsx` (`CATEGORIES`).
3. Add an SVG glyph to `icons.tsx` `GLYPHS` (currentColor, 24×24 viewBox) — we
   give **every** game a real icon, not just the emoji fallback.
4. Add a one-line rules entry to `rules.ts` (or set `def.rules`).

---

## 4. How we think about UX

The product is a **family arcade**, mobile-first, playful — not a research demo.
Principles, in priority order:

1. **A locked, obvious flow.** `launcher → setup → play → game-over`. Each screen
   does one thing. The setup screen is where all choices happen.
2. **No mid-game rule changes.** You choose mode/board/players, it loads, and that
   game is fixed until it ends. (This was an explicit product rule — don't add
   controls that mutate the running game.)
3. **Quick-play *and* "go crazy".** Every game ships **presets** — a `⭐ Classic`
   for instant play, plus **wild presets** that crank the knobs: bigger boards,
   higher win-lengths, and **more players** (`🤯 6-Player Mayhem`, `4-Player
   15×15`). Power users open **Customize** for the raw knob sliders. The wild
   flexibility is a headline feature, not an afterthought — if a game *can* be
   cranked (size, win-length, players), expose it.
4. **Per-seat line-ups.** Every seat is independently Human or an AI at its own
   strength (`PlayerKind = 'human' | Difficulty`). Quick-mode buttons (Pass &
   play / vs AI / Watch) and a bulk "AI strength" control are shortcuts that fill
   the seat list; a "Customize players" panel sets each seat individually (so you
   can pit an Easy AI against a Hard one, or 3 humans + 2 AIs). `useGameSession`
   reads each AI seat's own difficulty when it moves. Single-player games (2048)
   use the `solo` flow (You-play + Hint, or Watch-AI). The AI is always treant;
   difficulty = MCTS playout budget + top-K visit-count temperature (value-aware
   weakening in `treant-wasm/src/difficulty.rs`); levels are
   `gameTypes.DEFAULT_DIFFICULTY`, overridable per game via `difficulty` on the
   `GameDefinition` and measured by the self-play harness
   `treant-wasm/examples/calibrate.rs` (results in `plans/ai-calibration-results.md`).
   The line-up encodes in the URL (`s=hd`, `s=emde`) for deep links.
5. **It should look like the thing.** Pawns are pawns, hexes are hexagons, stones
   are stones, territory is filled colour. Pieces are coloured by player. We took
   the time to replace "X/O letters on a circle" with real shapes — that's the bar.
6. **Game feel.** Diff-driven CSS animations (`boardDiff.ts`) and synthesized Web
   Audio SFX (`sound.ts`) on moves/wins. A mute toggle. Keep it cheap and subtle.
7. **Self-explanatory.** Every game has a **📖 How to play** panel (setup screen)
   and a **? Rules** toggle in-game, plus a one-line blurb. A new player should
   never be stuck wondering what the goal is.

### Knobs & presets — design guidance

- Give a knob a clamped, *sane* range that the engine actually supports
  (`MAX_DIM`, `MAX_PLAYERS`, `MAX_PITS`…). The slider should never produce an
  illegal config.
- Presets are curated points in knob-space with playful names + an emoji. Aim for
  ~3–5: one Classic, one or two "interesting", one "🤯 maxed".
- `numPlayers` is a knob like any other (2–6) **only if the engine is genuinely
  N-player** (the symmetric line/sow/race games). Partisan and impartial 2-player
  games (Hex, Domineering, Nim, …) are 2-player by nature — don't fake it.
- Keep `defaultParams` = the `⭐ Classic` preset.

---

## 5. The player-colour system

Six seats, one source of truth per layer. To support a seat you must extend
**all** of these:

- CSS vars `--arc-p1..--arc-p6` (`arcade.module.css`).
- `PLAYER_LABEL` (`GamePlay.tsx`) — Red, Yellow, Green, Purple, Teal, Orange.
- Per-game seat maps: `COLOR`/`SEAT_COLOR`/`DISC`/`SEAT` arrays and the
  `X/O/A/B/C/D` symbol→colour maps in the grid/mancala/pig/shift boards.
- Engine `PLAYER_SYMBOLS` + `MAX_PLAYERS`.

`def.playerLabels` overrides the default labels per game and **must match the
rendered colours** (e.g. Frontline pieces are `--arc-p1`/`--arc-p2`, so its
labels are `['Red','Gold']`, not `['Red','Black']`).

---

## 6. Naming & rules

- **Non-copyrighted names only.** Use the public/abstract name, not the
  trademarked product: *Shift* not Othello, *Frontline* not Breakthrough™,
  *Trap-Three* not Squava™, *No-Tac-Toe* not Notakto™, *First Capture* not
  Atari-Go. Traditional/public-domain names are fine (Hex, Kōnane, Mu Tōrere,
  Mancala, Nim, Sim, Domineering, Col).
- Every game needs concise, kid-readable **rules** in `rules.ts`: what you do on a
  turn and how you win, in 1–3 sentences.
- **Variant tiles are cheap and encouraged.** A named variant with recognition
  value ships as its OWN launcher tile: a config-only GameDefinition
  (name/icon/blurb/rules + default params/flags) over a shared engine — the
  Gomoku-on-TicTacToeWasm pattern. Players find "Giveaway Checkers" by name;
  the tile still exposes the full knob panel. Variants without a recognizable
  name stay knob-only. Never fork engine code for a variant a rule-flag can
  express (see GAME-IDEAS.md §6x for the flag map).

---

## 7. Gotchas (learned the hard way)

- **Board sizing / "it resizes when I place a piece".** Boards must fill width,
  not shrink to content. The root container `.arcade` needs `width: 100%`
  (a flex item with `margin: auto` shrinks to content and *centers* — so an empty
  grid had tiny min-content width and grew on the first move). Board grids use
  `grid-template-columns: repeat(n, 1fr)` and fill their parent.
- **Routing.** Arcade screens are encoded in the URL search string and driven
  through Docusaurus's own router (`useHistory`/`useLocation` in `ArcadeShell`).
  **Never** poke `window.history` directly — it desyncs react-router and the
  browser Back button breaks (jumps out to the docs home). Screens are
  deep-linkable: `?game=&mode=&difficulty=&play=1&p=<json>`.
- **gtag in dev.** The GA4 plugin's pageview hook runs on every client navigation
  but isn't loaded on localhost, throwing "gtag is not a function" and a red error
  overlay. `ArcadeShell` guards it with a no-op (production has the real gtag).
- **AI turn cancellation.** `useGameSession` schedules AI moves on a timer; a
  generation guard + `clearTimeout` cancel stale turns (else double-clicking "Play
  again" on an AI-first game leaves two AI loops racing one handle).
- **`npm install treant-wasm` reorders `package.json`.** It re-alphabetizes deps;
  `git checkout docs/package.json` after each install.
- **Playwright stability vs animations.** The rotating hero / move animations fight
  screenshot/click stability — verify via DOM (`browser_evaluate` on the
  accessibility tree / element state), not screenshots, and use `.click()` /
  dispatched events rather than waiting for "stable".
- **The flex `margin: auto` gotcha bit THREE times.** A `margin-left/right: auto`
  on any child of the flex-column `.play`/`.setup` shrinks it to min-content
  (boards became 92px thumbnails on desktop; the C4 setup preview collapsed to
  a 41px chip because `minmax(0,1fr)` grids have ~zero min-content). Center
  with a capped parent or `width: 100%` + `max-width`, never bare auto margins.
- **Backgrounds on `<body>` tile at ~100vh.** Docusaurus keeps `<body>` at
  viewport height, so a body background gradient gets tiled by the canvas and
  draws a hard seam one screen down. Page-wide backdrops go on a
  `position: fixed` `body::before` (iOS ignores `background-attachment: fixed`).
- **The arcade page does NOT use `@theme/Layout` — never reintroduce it.**
  Docs chrome (navbar/footer) must not exist on this route in ANY loading
  state. Earlier revisions rendered Layout and hid the chrome with a body
  class + CSS; that leaked the full docs navbar during hydration, on slow
  loads, and through stale service-worker shells ("the hamburger menu is
  back"). `src/pages/arcade.tsx` renders its own minimal shell (Head +
  ErrorBoundary + BrowserOnly); `static/arcade-shell.css` is linked via
  `<Head><link>` for the first-paint backdrop (a `<link>`, not inline
  `<style>` — Docusaurus SSG emits helmet link tags but silently drops style
  tags). The `body.arcade-route` CSS rules in custom.css remain as defense in
  depth only.

## 7b. Conventions added by the 2026-07 UX rounds (new games must follow)

- **Dark board palette.** Boards paint with `--arc-card`/`--arc-soft`/`--arc-ink`,
  which the always-on `.neon` skin redefines DARK (Reversi's felt is the house
  style). Never hardcode cream/white surfaces; saturated seat colours
  (`--arc-p1..p6`) are the pieces' contrast. Check small on-board text against
  the dark surface.
- **i18n.** Every display string in a Board goes through `useT()`/`t()`
  (gettext-style: the English string is the key; `{param}` placeholders).
  GameDefinition data fields (name/blurb/presets/knobs/playerLabels/rules) stay
  plain English — render sites translate them. After adding a game run
  `node docs/scripts/i18n-keys.mjs --write` and top up all six
  `i18n/locales/*.json` (verifier must report 0 missing).
- **Undo.** `useGameSession` replays the move log on a fresh handle. Chance
  games (dice, random spawns) must set `noUndo: true` on the GameDefinition or
  undo will reroll their randomness.
- **Setup preview.** The setup screen renders `def.Board` non-interactively
  with a throwaway engine's `getBoard()` at the chosen params. Boards must
  render sanely with an initial board string, `interactive={false}` and empty
  `legalMoves` (captions inside `.shiftCaption` are auto-hidden there).
- **Turn colour cues.** The turn banner shows a seat-coloured dot
  (`--arc-p{n}`), pulsing while the AI thinks. Boards should also tint any
  "whose move" affordance (e.g. Connect Four's drop arrows) by
  `currentPlayer`, not a fixed accent.
- **Last-move marker.** `useGameSession` diffs the board string at move time
  and passes the changed indices as `BoardProps.lastCells`. Flat-grid boards
  MUST append `${last.has(i) ? styles.lastCell : ''}` to their cell class so
  the AI's reply is findable at a glance (SVG boards: TODO equivalent). Boards
  whose rendered cells don't map 1:1 to board-string indices skip it.
- **Board control rows must hide in previews.** Any always-rendered control
  strip inside a Board (drop arrows, swipe buttons) needs a
  `.setupPreview <class> { display: none }` rule — disabled buttons read as
  rendering junk in the static setup preview.
- **Speak the player's language.** Single-human games say "Your turn", never
  third-person; screen-reader cell labels use `BoardProps.playerNames`
  (translated seat names), not raw engine glyphs (X/O).
- **CTA hierarchy.** Solid neon-green is reserved for the one primary action
  per screen; selected toggles are outline pills. Don't add new solid-green
  buttons.
- **Narrate non-goal endings (`resultFlavor` seam).** Games that end by box-in,
  immobilisation, or any "no-glow" condition (nothing lights up to explain the
  result) must supply `GameDefinition.resultFlavor` to narrate *why* the game
  ended — the generic "X wins" banner leaves a boxed-in loss looking arbitrary.
  Adopted by Slimetrail, Toads & Frogs, Pawn Duel, Strand, Len Choa, Trails,
  Joust, Kōnane, and Amazons; new games with non-goal endings follow the same
  seam (see `gameTypes.ts` for the callback's context args).

### Hidden-information games (the pass-screen primitive)

Games with per-seat secrets (Bulls & Cows, and later Salvo/Ambush) set
`GameDefinition.hiddenInfo: true`. That single flag drives a reusable
pass-and-play blackout/per-seat-view primitive built into the shared session +
`GamePlay` flow — no per-game plumbing. The contract:

- **Per-seat views.** The `GameHandle` gains `getBoardFor?(seat)`. When
  `hiddenInfo` is set, `useGameSession` feeds the board `getBoardFor(viewSeat)`
  instead of `getBoard()`: the current mover in pass-and-play, or the lone
  human's *fixed* seat vs an AI (so the human never sees the AI's secret, even on
  the AI's turn). The engine's `get_board_for(seat)` string must contain ONLY
  what `seat` may know — its own secret plus all public info, **never** another
  seat's secret (reveal both only at game over). `getBoard()` (the fallback for
  perfect-information games) still only ever exposes the current seat's view.
- **Seating precondition (enforced).** The viewSeat/handoff logic is only correct
  when **at most one human** shares the table with AI. The three supported
  line-ups: (1) **all-human** — pass-and-play blackouts between every seat;
  (2) **all-AI (watch)** — `viewSeatOf` shows the current mover, no blackouts;
  (3) **exactly-one-human vs AI** — the lone human's *fixed* seat is shown even on
  the AI's turn. A line-up with **2+ humans AND any AI seat** (only possible at
  `numPlayers ≥ 3`) is **unsupported and rejected**: `useGameSession` throws at
  session start (gated behind `hiddenInfo`, so perfect-info games never hit it).
  Left unguarded it would silently skip the between-human blackouts *and* leak
  seat-0's secret to the other human, because `viewSeatOf` returns a single fixed
  seat (`humans[0]`) and `maybeHandoff` only fires when **every** seat is human.
  A future multi-human-plus-AI hidden game (Salvo/Ambush at 3p+) must first
  generalize both: a **per-current-human** view (each human sees only their own
  seat, keyed off the mover) and **blackouts between human turns even when AI
  seats are interleaved** (raise the handoff whenever control passes from one
  human to a *different* human, skipping only human→AI→same-human runs).
- **Blackout handoff.** In pass-and-play (every seat human), `GamePlay` raises a
  fully opaque, **full-viewport** blackout (`position:fixed; inset:0;` z-index
  above the nav) before each seat's turn: "Hand the phone to {name} — nobody else
  look!". It covers the board, counters, and history; only its button dismisses
  it (Escape is intentionally inert — no accidental peeking). vs-AI skips the
  blackout entirely (the AI doesn't peek). The board underneath a blackout is
  already the *incoming* seat's view, so even a render glitch can't leak.
- **Hard secrecy rules.** Secrets never appear in `get_board()`/`get_board_for()`
  for the wrong seat, in the URL/share flow, in `legal_moves` for the wrong seat,
  or in console logs. Set `noUndo: true` (undo replays the move log on a fresh
  engine, which would replay secret-setting moves and leak via the log).
- **AI must not cheat.** A hidden-info AI must decide from public + own-secret
  info only, never the opponent's hidden state. Bulls & Cows does this with a
  closed-form consistent-set deducer (it reads only its own feedback log) rather
  than a tree search over the true state — see the `bullscows.rs` header and its
  `ai_is_a_pure_function_of_feedback` test. (Proper determinized MCTS over
  sampled hidden states is deferred to Salvo/Ambush.) Because self-play win-rate
  is skewed by turn order under strict alternation, the difficulty ladder is
  hand-set (nim precedent), while the game is still registered in `calibrate.rs`
  so the audit fuzzer exercises it.

---

## 8. The add-a-game recipe + checklist

1. **Rust** `treant-wasm/src/<game>.rs`: `GameState`, eval/cfg, wasm class.
   - `mod <game>;` + `pub use <game>::<Game>Wasm;` in `lib.rs` (alphabetical).
   - Unit tests: move generation, a terminal-verdict assertion, `ai_plays`.
2. **React** `docs/src/components/arcade/games/<game>.tsx`: `GameDefinition`
   (reuse a handle + Board; custom board only if needed).
3. Register: `games/index.ts`, a `Launcher.tsx` category, an `icons.tsx` glyph, a
   `rules.ts` entry.
4. **Verify — the whole loop, every time:**
   ```
   cargo clippy -p treant-wasm          # 0 warnings
   cargo test -p treant-wasm <game>     # unit tests pass
   cd treant-wasm && wasm-pack build --target web
   cd ../docs && npm install treant-wasm && git checkout package.json
   npm run build                        # typecheck the whole site
   # restart dev server on 0.0.0.0:3939, then Playwright DOM-verify:
   #   open the game, start vs-AI, make a real move, confirm the AI replies,
   #   check the move/legality is correct, and the console has 0 errors.
   ```
5. **Commit, do not push.** A push triggers a production deploy. Commit locally
   with the `Co-Authored-By` trailer; the human pushes when ready.

Keep `GAME-IDEAS.md`'s "shipped" count in sync.
