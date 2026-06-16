# Treant Arcade — Games Roadmap & Ideas

A living backlog for the mcts.dev arcade: what's shipped, ~56 candidate games each
scored for fit / build effort / AI strength / engine reuse / name safety, and a
recommended build order.

> Generated from a brainstorming pass (6 category agents + synthesis) plus a
> fit/naming review. Not gospel — a menu to pull from.

---

## 1. What a candidate game must be

The arcade runs on **treant** (Monte Carlo Tree Search). A game has to be a
turn-based `GameState`:

- `current_player()`, `available_moves()` (a *discrete* set of legal moves),
  `make_move()`, and terminal detection (win/loss/draw or a final score).
- The engine supports **2+ players** (free-for-all turn order), **chance nodes**
  (dice / random spawns), and an **exact solver** + **score-bounded search** for
  small games.

**treant must be a competent AI opponent** (a game is only as fun as the
opponent). So we avoid: hidden information, simultaneous moves, and branching too
large for phone-budget playouts. Small variants of heavy games are fine (7×7 Hex,
6×6 Breakthrough, Atari/capture Go).

The arcade UX: mobile-first, **pass-and-play / vs-AI (Easy-Med-Hard) / Watch-AI**,
plus **solo** for single-player games. Every game is a `GameDefinition`
(presets, knobs, board renderer) over a WASM class.

## 2. Naming rule (the "Shift" lesson)

- ✅ **Safe:** traditional/folk games (public domain by age), academic/combinatorial
  games, and plainly descriptive names.
- ✏️ **Rename:** modern *commercial* board games whose names are trademarks.
- This is a practical read, **not legal advice** — do a quick trademark search
  before committing to a public name.

## 3. Already shipped — 26 games

**Original 6:** Tic-Tac-Toe · Connect Four · Shift · Nim · Mancala (Kalah) · 2048.

**Built since (overnight session, 2026-06-16) — covering every major mechanic:**

| Game | Mechanic | AI |
|---|---|---|
| Order & Chaos | line-of-N, asymmetric (either symbol) | 💪 |
| No-Tac-Toe | misère tic-tac-toe (shared mark) | 🏆 |
| Trap-Three | line: 4 wins / 3 loses | 🏆 |
| Square Up | win on a square of 4 marks | 💪 |
| Connect Six | 2 stones/turn, six-in-a-row | 💪 |
| Gomoku | five-in-a-row (TTT engine preset) | 💪 |
| Dots & Boxes | edge-claiming, bonus turn on a box | 💪 |
| Pig | press-your-luck dice (chance nodes) | 🎲 |
| Frontline (Breakthrough) | pawn movement/capture race | 💪 |
| Hex | connection (BFS shortest-completion) | 💪 |
| Clobber | capture-by-replace (CGT) | 🏆 |
| Kōnane | jump-and-capture removal (last to move) | 🏆 |
| Reversi | disc-flipping (+ passes/draws) | 💪 |
| Trails | light-cycles (move + leave a wall) | 💪 |
| First Capture | groups & liberties (capture-Go) | 💪 |
| NoGo | anti-Go: never capture or self-trap | 🏆 |
| Chomp | eat-the-poison (impartial) | 🏆 |
| Wythoff's Queen | two-heap subtraction (queen on a grid) | 🏆 |
| Mu Tōrere | slide on an 8-point star (kewai rule) | 🏆 |
| Domineering | partisan domino placement (V vs H) | 🏆 |

Each has its own `treant-wasm` GameState + Rust unit tests, an arcade
`GameDefinition`, a custom SVG icon, and was browser-verified. Reusable arcade
infra added: `MarkGridBoard` (placement), `MoveBoard` (select-then-move with
engine-authoritative legal-target highlights via `BoardProps.legalMoves`),
`moveHandle`, per-game `playerLabels`.

CF + TTT share the `treant-games` grid engine; **any "different size / k / players"
game is already just a preset of that engine** (e.g. Gomoku = TTT 15×15, k=5).

## 4. Legend

- **Fit** (arcade/family appeal): ★ niche · ★★ good · ★★★ great
- **Build** (implementation effort on our pattern): 🟢 easy · 🟡 medium · 🔴 hard
- **AI** (how well treant plays it): 🏆 solver-perfect · 💪 strong MCTS ·
  ⚠️ needs a real evaluator + more playouts · 🎲 chance-node (handled)
- **Engine:** *reuse* (extends an existing engine, cheap) vs *new* (new `GameState`)

---

## 5. Recommended build order (the roadmap)

A pragmatic sequence — front-load cheap reuse and high-delight new mechanics,
defer the heavy evaluators.

**Wave 1 — Grid Pack** *(near-free: rule tweaks on the existing grid engine)*
Order & Chaos · No-Tac-Toe · Trap-Three · Square Up · Connect Six.
*Why first:* each is a flag or swapped win-check on code we already shipped; adds
5 games for ~1 game's effort and broadens the "TTT family" shelf.

**Wave 2 — Chance done right** *(showcase treant's stochastic search)*
Pig → Climb → Greed.
*Why:* Dice was dropped for having no real decision; these are genuine
press-your-luck games and a flagship for chance nodes. Pig is the smallest.

**Wave 3 — New-engine flagships** *(strong AI, low branching, no draws)*
Frontline (Breakthrough) · Hex (7×7) · Trails (Surround, 2-4p FFA) · Clobber.
*Why:* each is a clean new `GameState` where treant looks smart cheaply; Trails
exercises multiplayer MCTS; Hex is the marquee connection game.

**Wave 4 — Depth & variety**
Reversi · Konane · Oware · Nine Men's Morris · Handoff (Quarto) · Quadline (Teeko).

**Wave 5 — Deep cuts** *(great eventually; need tuned heuristics)*
Amazons · Hedge (Quoridor) · Tablut · Fanorona · Havannah · Lasca · Halma · Sprouts.

---

## 6. The catalogue (~56), by engine family

> Format: **Safe Name** (← trademarked original) · players · Fit · Build · AI ·
> *engine* — one-line rules.

### 6a. Grid line-of-N family — REUSE the `treant-games` grid engine
*The cheapest additions: place a mark / detect a shape, with a rule twist.*

- **Order & Chaos** · 2p · ★★★ · 🟢 · 💪 · *reuse* — both players may place X *or* O on a 6×6; Order wins on 5-in-a-row, Chaos wins if the board fills with none.
- **No-Tac-Toe** (← Notakto) · 2p · ★★ · 🟢 · 🏆 · *reuse* — shared mark; completing any 3-in-a-row **loses** (misère). Play 1–3 boards.
- **Trap-Three** (← Squava) · 2p · ★★ · 🟢 · 🏆 · *reuse* — on 5×5, 4-in-a-row wins but **3-in-a-row loses**; both live at once.
- **Square Up** (← Square It!) · 2-4p · ★★ · 🟡 · 💪 · *reuse* — place stones; win when four of yours form the corners of *any* square (even tilted).
- **Connect Six** · 2p · ★★ · 🟡 · 💪 · *reuse* — place **two** stones per turn (one on the first move); first to six-in-a-row. Locality-prune to tame branching.
- **Pinch-Five** (← Pente; PD *Ninuki-renju*) · 2-4p · ★★ · 🟡 · 💪 · *reuse* — five-in-a-row **or** capture five flanked pairs (sandwich exactly two).
- **Handoff** (← Quarto) · 2p · ★★★ · 🟡 · 🏆 · *reuse-ish* — pieces have 4 binary traits; **your opponent picks the piece you must place**; a line of 4 sharing any trait wins.
- **Stack-Up** (← Gobblet) · 2p · ★★ · 🟡 · 🏆/💪 · *reuse-ish* — 3×3 line win, but bigger pieces "gobble" smaller (yours or theirs); pieces can move.

### 6b. Shift family — place-then-slide (you basically have this)

- **Quadline** (← Teeko) · 2p · ★★ · 🟢 · 🏆 · *reuse* — drop 4 pieces each, then slide one to an adjacent cell; win with a line **or** any 2×2 square.
- **Three Men's Morris** · 2p · ★★ · 🟢 · 🏆 · *reuse* — place 3, then slide to make 3-in-a-row. **≈ Shift already** — fold in as a Shift preset or skip.
- **Picaria** · 2p · ★ · 🟢 · 🏆 · *reuse* — Three Men's Morris on a Zuni board (extra diagonals, no corner-line). Shift-adjacent.

### 6c. Mancala family — REUSE the sowing engine

- **Oware** · 2p · ★★ · 🟡 · 💪 · *reuse* — sow counter-clockwise; capture when your last seed brings an opponent pit to 2 or 3, sweeping backward; must not starve the opponent. (The deeper mancala.)

### 6d. Nim family — impartial subtraction games

- **Wythoff's Nim** (Queen's-move Nim) · 2p · ★ · 🟢 · 🏆 · *reuse-ish* — a queen moves toward (0,0) any distance left/down/diagonal; land on (0,0) to win.
- **Euclid's Game** · 2p · ★ · 🟢 · 🏆 · *new* — from two integers, subtract a multiple of the smaller from the larger; reach 0 to win.
- **Chomp** · 2p · ★ · 🟢 · 🏆 · *new* — eat a brownie + everything right/below it; forced to eat the poisoned corner = you lose.

### 6e. Connection (new engine: union-find to edges)

- **Hex** · 2p · ★★★ · 🟡 · 💪/🏆 small · *new* — connect your two opposite edges with one chain; **never draws**. Swap rule balances first move.
- **Y** · 2p · ★★ · 🟡 · 🏆 small · *new* — connect all three sides of a triangular board with one group.
- **Havannah** · 2p · ★★ · 🔴 · ⚠️ · *new* — on a hex-of-hexes, win with a ring, a bridge (two corners), or a fork (three edges). Ring detection is the hard part.
- **Peglink** (← TwixT) · 2p · ★★ · 🔴 · ⚠️ · *new* — place pegs that auto-link a knight's-move apart if the link crosses none; connect your borders.
- **Gather** (← Lines of Action) · 2p · ★★ · 🟡 · 💪/⚠️ · *new* — pieces move exactly as far as there are pieces on that line; collect all yours into one connected group.
- **Reef** (← Atoll) · 2p · ★★ · 🟡 · 🎲 · *new* — roll a die to pick a zone, then place a stone there; connect your borders. (Original coinage; connection + chance.)

### 6f. Territory & capture (new)

- **First Capture** (Capture-Go 6×6) · 2p · ★★ · 🟢 · 💪 · *new* — place stones; a group with no liberties is captured; first capture wins.
- **Reversi** (6×6) · 2p · ★★ · 🟢 · 💪 · *new* — flank a line of enemy discs to flip them; most discs when full wins. (Use Reversi, **not** Othello™.)
- **Dice Reversi** · 2p · ★★ · 🟡 · 🎲 · *new* — Reversi but a die caps each turn's flips; free drop if no legal move.
- **Clobber** (5×5) · 2p · ★★ · 🟢 · 🏆 · *new* — board starts as a full checkerboard; move onto an adjacent enemy to remove it; no move = you lose. Always ends.
- **Snort** · 2p · ★ · 🟢 · 🏆 · *new* — color an empty cell, never adjacent to an *enemy* color; last to place wins.
- **Col** · 2p · ★ · 🟢 · 🏆 · *new* — color an empty cell, never adjacent to your *own* color (spread out); last to place wins. (Mirror of Snort.)
- **Konane** · 2p · ★★ · 🟡 · 💪 · *new* — Hawaiian checkers: after an opening removal, jump an adjacent enemy into the empty space beyond; chains allowed; no jump = lose.
- **Trails** (← Surround) · 2-4p · ★★★ · 🟢 · 💪 · *new* — move your token one step; the vacated cell becomes a permanent wall; can't move = eliminated. Light-cycles; great FFA.

### 6g. Mills / morris (new)

- **Nine Men's Morris** · 2p · ★★ · 🟡 · 💪 · *new* — place 9 then slide; form a "mill" (3-in-a-line) to remove an enemy; reduce them to 2 or block them to win.
- **Dara** · 2p · ★★ · 🟡 · 💪 · *new* — drop 12 each (no 3s while dropping), then slide; making exactly-3 removes an enemy (4+ never counts).

### 6h. Asymmetric hunts & tafl (new)

- **Bagh-Chal** · 2p · ★★ · 🟡 · 💪 · *new* — 4 tigers jump-capture vs 20 goats placed then moved to immobilize them; 5 captures vs blockade.
- **Fox & Geese** · 2p · ★★ · 🟡 · 💪 · *new* — a lone fox jump-captures vs 13 geese cornering it.
- **Tablut** (7×7) · 2p · ★★ · 🔴 · ⚠️ · *new* — king + 8 defenders escape to a corner vs 16 attackers surrounding; rook-like slides + custodial capture. High branching.

### 6i. Draughts lineage / movement-capture (new)

- **Alquerque** (5×5) · 2p · ★★ · 🟡 · 💪 · *new* — slide along lines or short-leap capture; multi-jumps chain; capturing compulsory.
- **Fanorona** (5×5) · 2p · ★★ · 🔴 · ⚠️ · *new* — capture by *approach* or *withdrawal*, removing a whole line; mandatory, chains with direction changes.
- **Seega** (5×5) · 2p · ★★ · 🟡 · 💪 · *new* — drop two men/turn, then move one step; capture by custodial sandwich; center immune.
- **Yote** · 2p · ★★ · 🟡 · 💪 · *new* — drop or step; jump-capture grants a bonus removal of any second enemy man.
- **Lasca** (7×7) · 2p · ★★ · 🔴 · ⚠️ · *new* — checkers, but captured pieces are imprisoned *under* the jumper into columns; only the top acts.

### 6j. Race & movement (new)

- **Frontline** (← Breakthrough 6×6) · 2p · ★★★ · 🟢 · 💪 · *new* — pawns step/capture diagonally forward; reach the back row (or capture all) to win. **No draws** → solver proves mid-game wins.
- **Hedge** (← Quoridor 7×7) · 2-4p · ★★ · 🔴 · ⚠️ · *new* — move your pawn or drop a wall to block; race to the far edge; walls may never fully seal a path (BFS check).
- **Halma** (8×8) · 2-4p · ★★ · 🟡 · ⚠️ · *new* — step or jump-chain (no captures); move all your pieces to the opposite corner. Weak mid-game signal → needs a distance evaluator.
- **Amazons** (6×6) · 2p · ★★ · 🔴 · ⚠️ · *new* — move a queen, then shoot a square away forever; last to move wins. High branching, decomposes late.
- **Castle Run** (← Camelot) · 2p · ★★ · 🟡 · ⚠️ · *new* — canter over your own pieces (chainable, no capture) or jump enemies; get two pieces into their castle.

### 6k. Tiny solver gems (new, but trivial AI)

- **Triangle Trap** (← Sim) · 2p · ★★ · 🟢 · 🏆 · *new* — color edges of a 6-dot graph; making a monochrome triangle **loses**. Draw-free (Ramsey), depth ≤ 15 — near-zero code, perfect AI.
- **Mu Torere** · 2p · ★ · 🟢 · 🏆 · *new* — slide a pawn into the lone empty point on an 8-star; trap the opponent. Retrograde-solvable.
- **Pong Hau K'i** · 2p · ★ · 🟢 · 🏆 · *new* — 2 pawns each on a 5-point board; slide into the empty point; lock the opponent. <100 positions.
- **Sprouts** (3-spot) · 2p · ★ · 🔴 · 🏆 · *new* — draw non-crossing lines + a dot; no dot exceeds 3 lines; no move = lose. Planar-map state is hard.

### 6l. Dice / press-your-luck (new, chance nodes)

- **Pig** · 2-4p · ★★★ · 🟢 · 🎲 · *new* — roll to build a turn-total; bank it, but a 1 wipes the turn. First to 100. The 30-second classic; known-optimal policy to check against.
- **Climb** (← Can't Stop) · 2-4p · ★★★ · 🟡 · 🎲 · *new* — roll 4 dice, pair them to advance runners; a roll with no legal pairing busts the turn. Top 3 columns wins.
- **Greed** (← Farkle) · 2-4p · ★★ · 🟡 · 🎲 · *new* — roll 6 dice, set aside scorers; a scoreless roll Farkles the turn; clear all six to reroll. First to a target.
- **Cross-Out** (← Quixx) · 2-4p · ★★ · 🟡 · 🎲 · *new* — on a shared roll, cross off a number strictly right of your last on a color track; skips are lost.
- **Race-Off** (Backgammon-lite) · 2p · ★★ · 🟢 · 🎲 · *new* — 3 checkers each on an 8-point one-way track (no hitting); roll 2 dice, bear off first.
- **Hackenbush Sprout** · 2-4p · ★ · 🟡 · 🎲 · *new* — roll a die to chop a colored edge; disconnected edges fall; no chop = lose.

### 6m. Open-info "card" duels (new; kept perfect-info on purpose)

- **21 Duel** (open-deck Blackjack) · 2p · ★★ · 🟢 · 🏆 · *new* — both draw from a single **face-up** deck, alternating Hit/Stand; closest to 21 without busting. Deterministic once shuffled.
- **Knock-Out Whist** (open hands) · 2-4p · ★ · 🟡 · 🎲 · *new* — **face-up** hands; play tricks, follow suit, trumps win; fewest tricks knocked out. Only chance is the deal.

---

## 7. Tally

- **~15 reuse an existing engine** (cheap): the Grid line-of-N pack (~8), Shift
  family (~3, two near-duplicates of Shift), Oware (Mancala), and the Nim-family
  trio.
- **~41 are new mechanics** — connection, territory/capture, mills, hunts,
  draughts-lineage, race, tiny solver gems, and dice/press-your-luck.

## 8. Open questions / notes

- Confirm trademark status before publishing any renamed game's public name.
- Multiplayer FFA games (Trails, Climb, Pig, Square Up, Pente) are good tests of
  the existing seat/mode system beyond 2 players.
- The ⚠️ games (Amazons, Hedge, Tablut, Fanorona, Lasca, Havannah, Halma) each
  need a hand-written evaluator/heuristic to make the AI feel strong — budget for
  tuning, don't ship them on raw MCTS.
- Several "🏆 solver-perfect" tiny games (Triangle Trap, No-Tac-Toe, Clobber,
  Mu Torere) double as great showcases of treant's exact solver, like Nim/TTT
  today.
