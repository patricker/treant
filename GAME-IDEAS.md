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

## 3. Already shipped — 40 games

**Wave of 2026-07-02:** Pinch-Five (Ninuki-renju: five-in-a-row or capture five
pairs, 2-4p) · Y (three-sided connection) · Bagh-Chal (tigers vs goats) · Climb (push-your-luck dice mountain) · Nine Men's Morris (place/slide/mill, men + flying knobs) ·
Quadline (Teeko: drop four then slide, line or square) · Oware (2s-and-3s
capture sowing, grand-slam + starvation rules).


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
| Treblecross | 1-D shared-mark, complete three to win | 🏆 |
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
| Square Subtract | remove a perfect-square count (Sprague-Grundy) | 🏆 |
| Euclid's Game | reduce the larger of two numbers to zero | 🏆 |
| Mu Tōrere | slide on an 8-point star (kewai rule) | 🏆 |
| Domineering | partisan domino placement (V vs H) | 🏆 |
| Amazons | move a queen then shoot a blocking arrow | 💪 |
| Col | map-colouring: no two own patches touch | 🏆 |
| Sim | edge-colouring on K6, avoid own triangle (misère) | 🏆 |
| Fox & Hounds | asymmetric chase (1 fox vs 4 forward-only hounds) | 💪 |

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

## 6z. Web-research wave (2026-07-05) — new candidates beyond the original catalogue

> From a targeted web pass (traditional games encyclopedias, CGT literature,
> pen-and-paper classics). Two-player, perfect-info, PD-safe unless noted.

- **Surakarta** · 2p · ★★★ · 🟡 · 💪 · *new* — 6×6, 12 pieces each; step any
  direction, but CAPTURE by travelling around one of the eight corner **loops**
  and landing on an enemy. A capture mechanic "not known in any other recorded
  game" — the arced board is a visual showpiece. Traditional Indonesian (PD).
- **Gale** (Shannon switching game; a.k.a. Bridg-It) · 2p · ★★★ · 🟢 · 🏆 small ·
  *reuse-ish* — claim edges on a grid-graph to connect your two sides / cut the
  opponent. Union-find reuse from Hex; pairs with Hex/Y on the connection shelf.
  Academic name "Gale" is safe (avoid "Bridg-It", 1960s trademark).
- **Slimetrail** · 2p · ★★★ · 🟢 · 🏆 · *new* — one shared token; each turn move
  it one step, its old cell becomes permanent slime; you win if the token
  reaches YOUR corner, lose if it reaches theirs. Kid-perfect, tiny engine,
  solver-strong.
- **Toads & Frogs** (Conway) · 2p · ★★ · 🟢 · 🏆 · *new* — 1-D row: toads march
  right, frogs march left; step into the gap or hop one enemy; no move = lose.
  Near-zero code (Treblecross shelf), and knobs galore (row length, piece
  counts, gap count, multiple rows).
- **Strand** (Isolation/Isola mechanic, renamed) · 2p · ★★ · 🟢 · 💪 · *reuse* —
  move your pawn, then remove ANY tile on the board; first pawn with no move
  loses. Engine is Trails plus "remove anywhere" — consider shipping it as a
  Trails *knob* ("walls: behind-you / anywhere / both") instead of a new game.
- **Len Choa** · 2p · ★★ · 🟢 · 🏆 · *new* — Thai hunt: 1 tiger vs 6 leopards on
  a small triangular board; tiger jump-captures, leopards immobilize. Fills the
  hunt shelf (Bagh-Chal, Fox & Hounds) with a smaller, solver-perfect entry.
- **World Threes** · 2p · ★★★ · 🟡 · 🏆 · *reuse* — ONE GameDefinition for the
  global tiny three-in-a-row family, with the **board itself as the knob**:
  Achi (Ghana, 3×3+diagonals), Tapatan (Philippines), Shisima (Kenya, octagon),
  Tant Fant (India, home rows), Nine Holes (England, no-slide-diagonals),
  Tsoro Yematatu (Zimbabwe, triangle), Picaria (Zuni). Peak
  silly-customization: travel the world by turning a knob. Engine = Shift with
  per-board adjacency tables.
- **Cram** · 2p · ★★ · 🟢 · 🏆 · *reuse* — impartial Domineering: either player
  may place a domino in either orientation; last to place wins (+ misère
  variant). Cheapest possible add: a rule FLAG on the existing Domineering
  engine — a customization win as much as a new game.
- **Sumito** (Abalone mechanic, renamed) · 2p · ★★ · 🔴 · ⚠️ · *new* — hex
  marble-pushing (2-vs-1 / 3-vs-2 shoves, push six off to win). Gorgeous but
  needs an evaluator + careful branching control; deep-cut shelf with Amazons.
  ("Abalone" is a live trademark — needs a clean name and rules-from-scratch.)

Modern commercial games reviewed and *rejected* on trademark/IP grounds despite
good fit: Hive, Onitama, Santorini, Tak, Quoridor (already renamed as Hedge in
§6j), Isolation (mechanic kept, renamed Strand above).

## 6n. Hidden-information pass-and-play (2026-07-05) — a NEW capability, not just new games

> Peter: "games that require actual passing, as in there is 'secret'
> information, like Battleship." These need one shared UX primitive plus an
> honest per-game AI story.

**The UX primitive (build once):** a blackout **pass screen** ("Hand the phone
to Gold — tap when ready 👀") between turns, plus per-player secret views
(your fleet vs your shots). Reusable by every game below; also unlocks
simultaneous-pick games later. Native-app tie-in: works even better in the
Capacitor build (no browser chrome to peek around).

**The AI story (be honest):** treant is a perfect-information MCTS engine.
Hidden-info games get AI via **determinization** — sample hidden states
consistent with what the AI has observed, run treant on each sample, vote on
the move. This is a real, respectable technique (and treant's chance nodes
help), but strength varies by game; ship each with calibrated expectations.
The AI must also *not cheat* — its playouts see sampled worlds, never the
human's true secrets.

- **Salvo** (Battleships mechanic) · 2p · ★★★ · 🟡 · 💪 determinized · *new* —
  the WWI public-domain paper game; Hasbro's trademark covers only the name
  "Battleship" ("Salvo" was the 1931 paper-era name and today names the
  N-shots-per-turn variant). Placement + calling shots; AI = placement
  sampling + hit-probability heat map, which is genuinely strong. Knobs go
  fully silly: grid 6–15, fleet composition editor (one mega-carrier vs
  a swarm of 12 dinghies), salvo size 1–5, hit-feedback rules (classic /
  "hot-cold" / silent-running), ships-may-touch toggle.
- **Bulls & Cows** (Mastermind's public-domain ancestor) · 2p · ★★ · 🟢 ·
  🏆 · *new* — each sets a secret code; alternate guesses; bulls = right
  digit+place, cows = right digit. Duel format is perfect pass-and-play; AI
  (entropy-greedy consistent-set search) is near-perfect. Knobs: code length
  2–6, symbol set (digits/colors/emoji), repeats allowed, race vs turn-count
  scoring.
- **Ambush** (L'Attaque mechanic, 1908 — Stratego® is the modern trademark) ·
  2p · ★★ · 🔴 · ⚠️ determinized · *new* — hidden ranks, capture reveals both.
  Start as a 6×7 "mini" (Stratego Duel-sized) to keep determinized MCTS sharp.
  Flag: do a trademark pass on the chosen name; rules themselves are ancient.
- **Spooks** (hidden-identity ghosts, Geister-like) · 2p · ★★ · 🟡 · ⚠️ ·
  *new* — 4 good + 4 bad ghosts each; win by exiting a good ghost or feeding
  the opponent your bad ones. ⚠️ IP caution: Geister is a 1982 commercial
  design (Alex Randolph); mechanic-with-rename is likely fine but this one is
  closer to the line than the traditional games — decide deliberately.
- **Liar's Dice** (traditional; Perudo® is the trademark) · 2p+ · ★★ · 🔴 ·
  ⚠️ · *new* — hidden dice + bluff calls; AI needs Bayesian opponent modeling
  more than tree search. Deep-cut shelf; the pass-screen makes it possible.

## 6o. Checkerboard shelf (2026-07-05) — more life from the 8×8 board

> We ship Frontline, Clobber, Kōnane, Fox & Hounds, Amazons, Reversi… but not
> the checkerboard's own game. Peter: "I don't know if I want chess, but there
> might be more ideas on a checkerboard."

- **Draughts (Checkers)** · 2p · ★★★ · 🟡 · 💪 · *new* — the glaring gap: the
  most recognized board game we don't have, and public domain everywhere. It's
  also a KNOB GOLDMINE: forced-capture on/off, flying kings (international
  rules), misère **Giveaway Checkers** (a real historical variant — lose all
  your men to win), board 8/10/12, rows-of-men 2–4, huffing. One engine,
  five+ classic named variants as presets (American / Russian / International
  / Giveaway / Sparse).
- **Dama (Turkish draughts)** · 2p · ★★ · 🟡 · 💪 · *new* — draughts but
  orthogonal (forward/sideways), full-board 16 men, kings slide like rooks.
  Distinct feel from diagonal checkers; PD traditional.
- **Latrunculi** · 2p · ★★ · 🟡 · 💪 · *new* — Roman soldiers' game:
  rook-slides + custodial (sandwich) capture. Ancient PD; rules are a
  scholarly reconstruction — say so in the rules text, pick the standard
  Kowalski reconstruction.
- **Pawn Duel** · 2p · ★★ · 🟢 · 🏆 small · *new* — chess pawns only ("chess
  without chess"): double-step, en passant, first promotion wins. Hexapawn's
  grown-up sibling; tiny engine, solver-strong on ≤6 ranks, and a gentle
  gateway for chess-curious kids. Knobs: files 4–10, ranks 5–8, en-passant
  toggle, pawns-per-side.
- Already in the backlog, same shelf: Alquerque, Fanorona, Seega, Yote, Lasca,
  Dara (§6g/6i), Gather (Lines of Action), Halma, Castle Run (Camelot).
- **Trails "Joust" knob** [ENG] — movement-pattern knob (king-step / knight
  leap) turns Trails into the classic knights-on-a-burning-board game with one
  flag; pairs with the "walls: behind-you / anywhere (Strand)" knob from §6z.

## 6y. Knob-expansion audit (2026-07-05) — customization headroom in the 40 shipped games

> Per-game audit of GameDefinitions vs their Rust constructors. `[UI]` = engine
> already supports it, knob/preset change only; `[ENG]` = needs a Rust change.
> Full renderer caveat: grids are CSS-flexible to ~19 wide; phone cell size and
> per-game difficulty constants (NOT retuned per size — giant boards silently
> play easier) are the honest limits. Wild presets keep the 🤯 convention.

### Tier 1 — engine headroom the UI never exposes (pure [UI], ~zero risk)

- **fox-hounds**: NO knobs today; engine takes (cols, rows) unclamped, hounds
  auto-scale with width. Add Width/Height 6–12 → a 12-wide board IS the 8-hound
  fantasy. Presets "Hound Wall 12×8" 🐕, "Thunderdome 12×12" 🤯. [ENG] 2 foxes.
- **wythoff**: NO knobs; engine clamps 4–12. Add a Size knob. [ENG] rectangular
  boards; multi-queen.
- **tic-tac-toe / gomoku**: engine MAX_DIM=15 but UI stops at 10/15-k-6. Widen
  dims to 15, k to 15/8 — "15×15 k=2" slapstick, "k=8 on 15×15" 🤯. [ENG] misère.
- **no-tac-toe**: UI max 6, engine (gridlib) takes 12. **Best single-knob change
  in the arcade**: misère TTT on 12×12. Also trap-three, square-up, order-chaos
  → widen to 12. [ENG] order-chaos target-line-length knob (5 hardcoded).
- **connect-six**: [ENG] expose stones-per-turn (Connect(k,p) family, place 3!).
- **mancala**: engine takes stones 1–8, pits 2–8; UI stops at 2–6/3–8. 1-stone
  Kalah is a weird puzzle; "Overflow" 8×8×4p. [ENG] capture-rule toggle.
- **pig**: engine target 20–200; UI 50–150. "Sprint to 20" lottery preset.
  [ENG] bust-on-1-or-2 "Two-Pig"; 2-dice variant.
- **shift**: engine 10×10/self-clamping pieces; UI 8×8/4. Widen; "1-piece duel".

### Tier 2 — unclamped engines, conservative UIs ([UI] widen + sanity check)

col →10-12 ([ENG] 3-color/3-player) · domineering →12 + skew "Corridor 3×12"
([ENG] Cram flag — both players either orientation, ties into §6z) · konane →12
+ "Runway 4×12" · nogo →9 · treblecross →24 (check 1×N phone overflow) ·
subtract-square →100 "Century" · euclid →99 · amazons 8×8 "Sprawl" preset
(honest: hard AI mushy; [ENG] 4 amazons/side) · baghchal goats→30 "Goat
Tsunami" ([ENG] tiger count 2–8) · nim stones→60 ([ENG] max-take knob;
**multi-heap Nim + misère** — canonical, MCTS near-perfect).

### Tier 3 — UI at engine clamp; one-line Rust clamp bumps ([ENG], trivial)

capture-go →13 + capture-target-N knob · chomp →10×10 · clobber →10 + 2×8
strip preset · dots-boxes →7 (hard AI weakens — label honestly) + 3-4 players ·
frontline →12 (+ "Thermopylae 4×10" preset works TODAY) · trails →12 · hex →13
+ size-4 "Baby Hex" + **pie/swap rule** · y →15 · reversi →12/14 +
**anti-Reversi misère flag** (fewest discs wins — one sign flip) + "Letterbox
4×10" preset · quadline win-length 3–5 · connect-four k→2 ("Connect-2 chaos"),
MAX_DIM→12, **Pop Out variant** (remove own bottom disc) · pinch-five pairs→10,
players→6 · nine-morris lasker-morris flag · climb →6p, toWin→8 · oware
seeds→8, pits→10.

### Tier 4 — zero-param games (each [ENG] is that game's only possible knob)

mu-torere ring size 8/10/12 · sim vertex count 5–7 (K7 is a real research
object) · 2048 grid 3×3 (brutal)/5×5 (zen) + spawn-4 probability + target tile.

### Top 10 cheapest-silliest, ranked

1. no-tac-toe →12×12 [UI] — one number, transforms the game
2. fox-hounds Width/Height knobs [UI] — unlocks 8+ hounds, zero Rust
3. tic-tac-toe/gomoku →15, k→8+ [UI]
4. wythoff size knob [UI] (knobless game, engine fully parameterized)
5. mancala stones 1–8, pits 2 [UI]
6. pig target 20–200 [UI]
7. nim multi-heap + max-take [ENG]
8. reversi anti/misère flag [ENG] — one sign flip, endless family arguments
9. connect-four k=2 + Pop Out [ENG]
10. hex pie rule [ENG] — the variant Hex players ask for first

## 6x. Knob design bible (2026-07-05) — variants research → knob sets

> Second research pass: the *named variants* of each game family in the
> literature are a pre-validated knob menu. Three outputs: (A) knob sets
> designed up-front for the proposed new games, (B) a variant-collapse map —
> backlog/catalogue games that become FLAGS on engines we already ship, (C)
> notes. Rule of thumb honoured throughout: a knob is only worth shipping if
> the AI still plays the variant credibly at phone budgets.

### A. Knob sets for the proposed new games

- **Draughts** — the deepest knob game we could ever ship; every knob below is
  a *real named variant*, and the variant presets are knob bundles:
  American ⭐ / Brazilian / Pool / Russian / Spanish / Italian / Frisian /
  **Giveaway** 🙃. Knobs: board 8/10/12 · men-rows 2–4 · forced capture
  off/on/maximum (Italian: max-pieces-then-kings priority) · men capture
  backwards y/n · flying kings y/n · promotion-mid-chain y/n (Russian: yes,
  Brazilian: no — a famously sneaky difference) · orthogonal captures
  (Frisian) · misère (Giveaway) · huffing 🙃 (the historical "steal the lazy
  piece" penalty — pure silly toggle). One engine, ~8 flags, 9 named games.
- **Salvo** — grid 6–15 · **fleet editor** (counts per ship length 1–5; one
  mega-carrier vs twelve dinghies) · ships-may-touch y/n · shots-per-turn 1–5
  (1 = classic, 3+ = the actual 1931 "Salvo" rules) · feedback style: classic
  hit/miss+sunk / silent-running (no "sunk" calls) / hot-cold 🙃 · moving-ships
  variant (one unhit ship may relocate per turn — advanced paper rule).
- **Bulls & Cows** — code length 2–6 · symbol set digits/colors/emoji 🙃 ·
  repeats y/n · win format: first-to-crack vs fewest-guesses match.
- **Surakarta** — piece rows 2–3 · win: annihilation vs first-to-N captures ·
  min-loops-per-capture 1–2 🙃 (2 makes captures spectacular and rare).
- **Gale** — grid size 3–7 dots · pie rule.
- **Slimetrail** — board size 5–9 · square vs hex adjacency · goal placement
  (opposite corners / random corners).
- **Toads & Frogs** — row length 5–15 · toads ≠ frogs (asymmetric armies!) ·
  gap count 1–3 · rows 1–3 played as a CGT *sum* (move in any one row) 🤯.
- **World Threes** — THE BOARD IS THE KNOB: Achi / Tapatan / Shisima /
  Tant Fant / Nine Holes / Tsoro Yematatu / Picaria / Pong Hau K'i (absorbs
  the §6k entry) · pieces 3–4 (Achi plays 4).
- **Pawn Duel** — files 4–10 · ranks 5–8 · double-step y/n · en passant y/n ·
  pawn rows 1–2. (files=3, no double-step = literally Hexapawn, free preset.)
- **Ambush** — board 6×7 mini → 8×8 · army composition editor 🙃 (eight bombs,
  zero scouts — why not) · scout long-move y/n.
- **Len Choa** — leopard count 5–7 · tiger jump chains y/n.
- **Dama (Turkish)** — board 8/10 · men-rows 2–3 · flying kings y/n.
- **Latrunculi** — board 8×8/8×12 (both attested) · piece counts · king
  (dux) moves y/n — reconstructions differ, so the knobs ARE the scholarship.

### B. Variant-collapse map — catalogue/backlog games that become flags

| Flag on existing engine | Games it absorbs | Effort |
|---|---|---|
| Col: adjacency polarity own↔enemy | **Snort** (§6f, whole entry) | [ENG] tiny |
| Domineering: either-orientation + misère | **Cram** (§6z) | [ENG] tiny |
| Trails: walls vacated-cell ↔ any-tile | **Strand/Isolation** (§6z) | [ENG] small |
| Trails: movement king ↔ knight | **"Joust"** knights-on-burning-board | [ENG] small |
| Gomoku: ruleset knob — freestyle / exact-five+overline-void / Caro (unblocked-5) / Renju forbidden-forks / swap2 opening | **Renju, Caro, Omok** (3 named games) | [ENG] medium (Renju fork detection is the hard one; ship overline/Caro/swap2 first) |
| Connect Four: pop-out move / cylinder wrap / prefilled-edges | **Pop Out, Cylinder C4, Hasbro "5-in-a-Row"** | [ENG] small each |
| Mancala: capture rule Kalah ↔ empty-capture ↔ Oware-style; multi-lap sowing | closes the Kalah↔Oware gap into one continuum | [ENG] medium |
| Nim (once multi-heap lands): max-take k / Moore's ≤k-heaps / Fibonacci (≤2× last take) / misère | **Moore's Nim, Fibonacci Nim** — canonical CGT set | [ENG] small each |
| Nine Men's Morris: 10 men + place-or-move | **Lasker Morris** (solved in the literature — AI calibratable) | [ENG] small |
| Pig: dice 1–2 / doubles rules / Hog mode (choose N dice, one throw) | **Two-Dice Pig, Big Pig, Hog** | [ENG] small each |
| Dots & Boxes: pre-drawn borders (Swedish/Icelandic) | named opening-theory boards | [ENG] tiny |
| Misère/anti flags: Reversi (Anti-Reversi), Hex (**Rex**), Checkers (Giveaway), Clobber | 4 named misère games | [ENG] tiny each (sign flips; NOTE: misère can invert difficulty calibration — retest AI levels) |
| World Threes board knob | Achi, Tapatan, Shisima, Tant Fant, Nine Holes, Tsoro Yematatu, Picaria, Pong Hau K'i, Three Men's Morris | one new engine absorbs NINE catalogue entries |

### C. Notes

- Collapse math: ~20 catalogue/named games become ~15 flags on 10 existing
  engines + 1 new engine (World Threes). Cheaper than building any 3 of them
  standalone.
- Difficulty blocks are per-game constants; variant flags (especially misère)
  can invert what "hard" means — each shipped flag needs a quick calibration
  pass (the autonomous audit harness from commit 3790639 can self-play these).
- Dots-and-Triangles needs a triangular-grid renderer — defer; Swedish boards
  are free.
- Renju's forbidden-fork detection (3-3/4-4/overline for Black only) is real
  work; Caro + overline + swap2 give 80% of the family for 20% of the effort.

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
