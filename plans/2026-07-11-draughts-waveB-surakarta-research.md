# Draughts wave B + Surakarta — sourcing & decision doc

**Date:** 2026-07-11
**Scope:** Can Spanish / Italian / Frisian / Turkish draughts ship over the
existing flag-driven draughts engine (`treant-wasm/src/draughts.rs`, 7 flags),
or do they need new flags / a new engine? Plus a from-scratch assessment of
Surakarta.
**Doctrine:** sourced-rules-or-blocked. Every load-bearing rule below is a
verbatim quote with a URL. Nothing invented.

## The engine we are mapping onto (ground truth)

`Draughts::new(size, men_rows, flying, men_back, max_capture, promote_mid, misere)`.
Confirmed by reading `draughts.rs`:

- **Topology is diagonal-only.** `const ALL4: [(i32,i32);4]` = the four diagonal
  steps. Both quiet-move generation and the recursive `chain()` capture
  generator walk *only* diagonals, on dark squares. There is no orthogonal path
  anywhere in the engine.
- **`max_capture` filters by raw COUNT.** In `available_moves`:
  `let best = caps.iter().map(|m| m.captured.len()).max(); caps.retain(|m| m.captured.len()==best);`
  — pure quantity. No notion of *which* pieces (man vs king) were taken.
- **`DMove.captured` is a `Vec<u16>` of squares.** The man/king identity of each
  captured piece is known *at generation time* (from the working board `bd`) but
  is not stored on the move. Any quality comparator can count kings during
  generation without a data-model change.
- **Kings already capture in all 4 diagonals; men forward-only unless `men_back`**
  (`chain()` comment line ~257). Non-flying kings are step-1 in all 4 diagonals.
- **No per-piece move counter, no fractional weighting** anywhere in state.

Six wave-A variants (American / International / Brazilian / Pool / Russian /
Giveaway) are asserted as a flag table in the module. **Any new flag must default
off and leave those six bit-identical** — the acceptance bar for every verdict
below.

Reconciliation with `GAME-IDEAS.md`: the Phase-4 roadmap (§5) *assumed* Spanish /
Italian / Frisian would be config-only tiles on this engine ("up to 9
named-variant tiles … Spanish / Italian / Frisian"). **This research corrects
that assumption** — see per-game verdicts. Surakarta is already catalogued in
§6z (★★★, 🟡, 💪, new); Turkish "Dama" in §6o (checkerboard shelf). This doc
extends those entries with engine shape; it does not restate them.

---

## 1. Spanish draughts (Damas) — VERDICT: config + **1 new flag**

Cheapest wave-B win. Diagonal topology, forward-only men, flying kings — all
already in the engine. The *only* gap is the quality tiebreak (most kings).

### Sourced rules

King is a flying (long-range) king:
> "They can move also backward. They are flying: they can move jumping over
> several empty squares along the diagonal line." … "They may capture an
> opponent piece by jumping over one or several squares, as long as there is
> only one opponent piece in the route and the other squares are empty."
> — https://www.ludoteka.com/games/spanish-draughts/rules

Men move and capture forward only:
> "Men move usually one cell forward. They can never move backward." … "Men
> capture by jumping over the opponent piece and landing on the next square
> behind, that must be empty." — ludoteka (same URL)

Capture is compulsory; priority is quantity **then** quality (most kings):
> "Whenever is possible, it is compulsory to play a capturing move." … "Quantity
> Rule: as many pieces as possible must be captured. Quality Rule: between
> capture moves that take the same amount of pieces, as much kings as possible
> must be captured." — ludoteka (same URL)

Wikipedia's Draughts article states the same rule and the board orientation
(orientation is human-notation only; it does not affect the game tree on our
internal coordinates):
> "A sequence must capture the maximum possible number of pieces, and the
> maximum possible number of kings from all such sequences."
> "Light square is on right, but double corner is on left, as play is on the
> light squares." — https://en.wikipedia.org/wiki/Draughts

(Note: the dedicated `en.wikipedia.org/wiki/Spanish_draughts` title 404s / is a
redirect; the rule content lives in the `Draughts` variants section, corroborated
by ludoteka.)

### Flag mapping

| Rule | Existing flag | Value |
|---|---|---|
| 8×8, 3 rows | size, men_rows | 8, 3 |
| Flying kings | flying_kings | 1 |
| Men capture forward only | men_capture_back | 0 |
| Compulsory maximum capture | max_capture | 1 |
| No mid-chain promotion | promote_mid_chain | 0 |
| Not misère | misere | 0 |
| **Quality: most kings among longest** | **— NONE —** | **new flag** |

### New flag: `capture_quality_priority` (call it mode 1 = "Spanish")

Semantics: after `max_capture` reduces to the longest chains, further `retain`
those that capture the greatest number of *kings*. Implementation is a
lexicographic key `(captured.len(), kings_captured)` in the same `retain` step,
where `kings_captured` is counted against the pre-capture board during
generation. **Mechanical extension of the existing count-filter — not a topology
change.** Defaults off ⇒ the six wave-A variants stay bit-identical.

### Estimate

- **New flags:** 1 (a capture-priority mode; see Italian — better modelled as a
  small enum shared with Italian).
- **Move-generation impact:** the priority rule *does* change which moves
  `available_moves` returns (it must filter to maximal-quality captures), but the
  change is confined to the post-generation `retain` — the `chain()` walker is
  untouched. Extension is mechanical.
- **Test surface:** ~6–8 tests — flag-table assertion row, plus crafted positions
  where two longest chains tie on count but differ on kings-captured, plus
  re-assert wave-A bit-identity with the flag off.

---

## 2. Italian draughts (Dama Italiana) — VERDICT: config + **2 new flags**

Same diagonal topology, but adds (a) *men may not capture kings* and (b) a
four-level capture priority that is a strict superset of Spanish's. Italian kings
are **non-flying** (step-1), unlike Spanish.

### Sourced rules

Board / pieces:
> "played on a board consisting of sixty-four squares … There are twenty-four
> pieces: twelve white and twelve black." — https://en.wikipedia.org/wiki/Italian_draughts

Men cannot capture kings:
> "Men cannot jump kings." — https://en.wikipedia.org/wiki/Draughts

Full ordered capture priority (verbatim, Italian_draughts "Capturing" section):
> "capture the greatest quantity of pieces" → "he must do so with the king" →
> "capture the greatest number of kings possible" → "capture wherever the king
> occurs first" — https://en.wikipedia.org/wiki/Italian_draughts

Same hierarchy, expanded, from the Draughts article:
> "A sequence must capture the maximum possible number of pieces. If more than
> one sequence qualifies, the capture must be done with a king instead of a man.
> If more than one sequence qualifies, the one that captures a greater number of
> kings must be chosen. If there are still more sequences, the one that captures
> a king first must be chosen." — https://en.wikipedia.org/wiki/Draughts

Orientation / men backward:
> "Italian draughts is played with the light square on the left (opposite
> Spanish), and men cannot jump backward." — https://en.wikipedia.org/wiki/Draughts

Italian kings are one-step (the "dama" moves one square diagonally, any
direction — *not* flying); this is the standard distinction from Spanish and is
why `flying_kings = 0` for Italian. (Corroborated across rules sites; Wikipedia's
Italian page is thin on king movement — flagged as an open item to pin with a
FID Italiana quote before shipping.)

### Flag mapping

| Rule | Existing flag | Value |
|---|---|---|
| 8×8, 3 rows | size, men_rows | 8, 3 |
| **Non-flying (step-1) kings** | flying_kings | 0 |
| Men capture forward only | men_capture_back | 0 |
| Compulsory maximum capture | max_capture | 1 |
| No mid-chain promotion | promote_mid_chain | 0 |
| Not misère | misere | 0 |
| **Men may not capture kings** | **— NONE —** | **new flag** |
| **4-level quality priority** | **— NONE —** | **capture-priority mode 2** |

### New flags

1. **`men_cannot_capture_kings`** (bool). In `chain()`, when the moving piece is
   a man and the jumped square holds a king, skip that continuation. A ~2-line
   guard in the capture walker. Interaction: independent of every existing flag;
   defaults off ⇒ wave-A untouched.
2. **`capture_quality_priority = 2` ("Italian")**. Extends the Spanish
   comparator to the full lexicographic key
   `(count, capturing_piece_is_king, kings_captured, king_captured_earliest)`.
   "capturing_piece_is_king" and "king first in the chain" are both derivable
   from the generated `DMove` + pre-capture board. Best modelled as **one enum
   flag shared with Spanish** (0=none, 1=Spanish=most-kings-only, 2=Italian=full
   4-level), so Spanish + Italian together cost **2 net new flags**, not 3.

### Estimate

- **New flags:** 2 net for Spanish **+** Italian combined
  (`men_cannot_capture_kings` + the shared `capture_priority` enum).
- **Move-generation impact:** the priority is again a post-generation `retain`;
  `men_cannot_capture_kings` is a guard *inside* `chain()`. Neither changes the
  diagonal topology. The 4-level comparator is the most intricate piece of logic
  but is pure ranking over already-generated chains — no new search machinery.
- **Test surface:** ~8–12 tests — the man-can't-jump-king guard (positions where
  a man's only "capture" is blocked by a king and must therefore play a quiet
  move), each priority tier with a crafted tie, plus wave-A bit-identity.

---

## 3. Frisian draughts — VERDICT: **structurally incompatible** (skip wave B)

Diagonal *movement*, but **orthogonal + diagonal capture in 8 directions** for
men *and* kings, a **fractional** king weighting, and a **stateful
consecutive-king-move limit**. Three independent things the current engine cannot
express. This is a new/heavily-generalized engine, not a flag.

### Sourced rules (all https://en.wikipedia.org/wiki/Frisian_draughts)

> "The game is played on a board with 10x10 squares … Each player has 20 pieces."

Men move diagonally forward but capture in **eight** directions:
> "Ordinary pieces move one square diagonally forward to an unoccupied square."
> "Enemy pieces can and must be captured by jumping over the enemy piece, two
> squares forward or backward to an unoccupied square immediately beyond in any
> direction (a choice of eight) along the horizontal, vertical and diagonal
> lines."

King movement + consecutive-move restriction:
> "Crowned pieces, called kings, can move freely multiple steps in any diagonal
> direction." "A king may only move three times in a row unless it makes a
> capture. Otherwise the player must move another piece."

Fractional quality weighting:
> "It is compulsory to make the highest shot value. Each king is worth a man and
> a half."

Crowning:
> "A piece is crowned if it stops on the far edge of the board at the end of its
> turn."

### Why it does not fit

| Rule | Engine reality | Gap |
|---|---|---|
| Capture in 8 directions | `chain()` walks only `ALL4` diagonals | **New capture topology** — a direction table the walker doesn't have. Core rewrite of the capture generator. |
| King = 1.5 men in shot value | filter is integer `captured.len()` | Comparator must weight kings ×1.5 (integer trick: `2·men + 3·kings`). *This part alone is easy* — but it rides on the 8-dir generator. |
| ≤3 consecutive king moves | no per-piece move history in state | **New state field** (consecutive-king-move counter, reset on capture / other-piece move) + generation must respect it. Novel stateful rule. |

Note the asymmetry that makes it *especially* awkward: men **move** diagonally
forward (fits existing quiet-move gen) but **capture** in 8 directions — so you
cannot just widen a single direction constant; move-gen and capture-gen diverge.

**Verdict:** Frisian is a dedicated engine (or a major generalization adding an
8-direction capture mode + shot-value comparator + king-move counter). Highest
risk of the four. **Defer past wave B.** If pursued later, do it as the anchor
for a "generalized-capture draughts" engine that could also host Turkish.

---

## 4. Turkish draughts (Dama) — VERDICT: **new engine** (own topology; tractable)

Orthogonal game on **all 64 squares**, not the diagonal dark-square engine.
Cannot be a config of `draughts.rs`. But it is a *clean, self-contained* engine —
arguably simpler than diagonal draughts (no double-corner geometry, immediate
removal). Already anticipated in `GAME-IDEAS.md` §6o as a checkerboard-shelf
candidate.

### Sourced rules (all https://en.wikipedia.org/wiki/Turkish_draughts)

> "On an 8×8 board, 16 men are lined up on each side, in two rows. The back rows
> are vacant." (i.e. men start on the 2nd and 3rd ranks; **all** squares are
> used, not just dark ones.)

> "Men move orthogonally forwards or sideways one square, capturing by means of a
> jump; they cannot move or capture backwards or diagonally."

> "Kings can move any number of empty squares orthogonally forwards, backwards or
> sideways." (rook-like flying king)

> "If a jump is available it must be taken. If there is more than one way to jump,
> the one capturing the most number of pieces must be taken." (compulsory + max
> capture)

> "Pieces are removed from the board immediately after being jumped." (**note:**
> immediate removal — the *opposite* of the diagonal engine's Turkish-stroke
> "remove only at chain end". Ironic naming; different rule.)

> "When a man reaches the back row, it promotes to a king." … "A player wins if
> the opponent has no legal move, either because all his pieces are captured or he
> is completely blocked."

### Engine shape

- **State:** 8×8 dense (all squares), 2 colors, man/king. No dark-square mask.
- **Directions:** orthogonal step table `[(0,±1),(±1,0)]`; men use forward+side
  (3 dirs), kings use all 4 at range.
- **Capture removal:** immediate — simpler than the diagonal engine's deferred
  removal (no "keep captured pieces as blockers" bookkeeping).
- **Max-capture filter:** identical `retain(longest)` logic to reuse conceptually.
- **Termination:** win = no legal move for opponent; add the same 40-ply
  no-progress draw cap for king-shuffle endgames (sources give no draw rule).

**Verdict:** viable **new engine**, moderate build, no reuse from `draughts.rs`
beyond patterns. Recognizable name, PD. Reasonable wave-B *stretch* if slack
remains after Surakarta — but it competes with Surakarta for "the one new engine"
slot and offers less novelty (it's still a checkers cousin).

---

## 5. Surakarta — VERDICT: **new engine** (unique showpiece; recommended)

Not a reuse candidate for anything shipped. The arc-capture board is a genuine
visual showpiece (§6z already rates it ★★★). Clean small engine; the only real
design decision (move encoding) resolves cleanly.

### Sourced rules (all https://en.wikipedia.org/wiki/Surakarta_(game))

Board & pieces:
> "Pieces always rest on the points of intersection of the board's grid lines."
> "Whereas the game Surakarta is smaller with a 6x6 grid and only 12 pieces."
> "Players begin the game with 12 pieces each."

Non-capturing move:
> "a player either moves one of their pieces a single step in any direction
> (forwards, backwards, sideways, or diagonally) to an unoccupied point"

Capture (the loop mechanic):
> "A capturing move consists of traversing along an inner or outer circuit …
> around at least one of the eight corner loops of the board, followed by landing
> on an enemy piece, capturing it." "Only unoccupied points may be travelled
> over; jumping over pieces is not permitted." "Any number of unoccupied points
> may be travelled over, before or after traversing a loop. An unoccupied point
> may be travelled over more than once during the capturing piece's journey."

Win:
> "A game is won when a player captures all 12 of the opponent's pieces."

No draw / repetition rule appears in the article, and no first-player-advantage
or solved result is stated.

### Engine shape

- **State:** 6×6 grid of *points* (36), 2 colors. ~50-bit state; trivially small.
- **Move encoding — RESOLVED as `(from, to)`, unambiguous for state:**
  - A non-capturing move lands on an **empty** adjacent point; a capture lands on
    an **enemy** point. The target's occupancy distinguishes the two.
  - A capture always removes *exactly one* piece: **the piece at `to`**. So the
    captured piece is fully determined by `to`.
  - The *arc path* is not part of the resulting state — intermediate points must
    be empty and are left unchanged. Two distinct arcs from the same `from` to
    the same enemy `to` (inner vs outer circuit, or multi-loop routes) yield the
    **identical board result**. Therefore `(from, to)` is a complete, unambiguous
    move encoding. The arc is a *derived rendering artifact*, recomputed on demand
    for animation — it never needs to be spelled out in the move.
  - **Legality**, however, does depend on ≥1 clear arc existing → move generation
    must do a circuit-reachability search (walk the 4 loop tracks, requiring all
    traversed points empty, until landing on the first enemy). Generation is the
    work; the move object stays `(from, to)`.
- **Directions (quiet):** 8-neighbour step to an empty point.
- **Terminal:** win when opponent has 0 pieces (capture-all). Add a
  repetition / no-capture-N-ply **draw cap** (sources give none, and
  non-capturing shuffles can cycle indefinitely — the same reasoning that forced
  the 40-ply cap in `draughts.rs`).
- **MCTS suitability:** branching ≈ (12 pieces × up-to-8 steps) + a handful of
  captures ⇒ ~20–40 moves/turn, moderate depth. Well within phone playout budget;
  material-count eval (each piece = 1) is a natural, cheap heuristic. No
  hidden info, no chance, deterministic — a good treant fit. No published solve
  or first-player-win result found, so no correctness oracle beyond self-play.

### Kid-legibility / UI

- **Render:** 6×6 points on lines, plus the **8 corner loop arcs** as two nested
  rounded circuits (an inner and an outer track) drawn as SVG paths hugging the
  board edges — the signature look. Pieces sit on intersections.
- **Capture animation is the teaching tool:** on a capture, animate the moving
  piece *travelling along the arc* into the target. This is what makes the
  otherwise-mysterious "you can take that piece from across the board" legible to
  a child — show the road the piece drives.
- **Legal-target highlighting** reuses the existing `BoardProps.legalMoves`
  primitive: tap a piece → highlight its empty step-neighbours *and* every enemy
  reachable via a clear arc. The engine already owns legality, so highlights are
  authoritative.
- **Interaction risk:** the arc path can be long and can loop the board — an
  instant "jump the piece to the target" would read as teleport-magic and
  confuse. **Mitigation is mandatory arc animation**, not optional polish. Second
  risk: two visually-different arcs to the same target are the "same move" to the
  engine; pick one canonical arc to animate (shortest clear route) so the UI is
  deterministic.

### Open item

Wikipedia's fetched text confirms 12 pieces each but does **not** verbatim pin
*which two rows* they start on. The universally-used setup is each player's two
nearest rows (rows 1–2 and 5–6 of the 6×6). Treat as attested-by-convention;
confirm against a rules PDF or a second encyclopedia before hard-coding.

---

## 6. Recommended wave-B scope

**Ship: Spanish + Italian (config, +2 net flags) and Surakarta (one new engine).
Hold Turkish as a stretch. Defer Frisian.**

Rationale — family value per unit engine risk:

1. **Spanish + Italian together** are the best value in the program right now.
   They reuse the entire diagonal engine and cost **2 net new flags**
   (`men_cannot_capture_kings` + a shared `capture_priority` enum), both
   defaulting off so the six shipped variants stay bit-identical. They add two of
   the most *recognizable* national draughts to the family shelf. Build them as a
   pair because Italian's priority comparator is a strict superset of Spanish's —
   one comparator, two modes. Risk: low; the intricate part (the 4-level Italian
   ranking) is pure post-generation `retain`, not new search machinery.

2. **Surakarta** is the wave-B *new-engine* pick. It is a small, clean,
   deterministic engine whose move encoding resolves to a plain `(from,to)`, and
   its arc board is a delight no other tile offers — maximum novelty per line of
   engine. The only real risk is UI (the arc overlay + mandatory capture
   animation), which is bounded and reuses `legalMoves`.

3. **Turkish (Dama)** — a viable, tractable *new orthogonal engine*, but it
   competes with Surakarta for the single new-engine slot and offers less novelty
   (another checkers cousin). Take it only if there is slack after Surakarta, or
   fold it into a future orthogonal-capture engine alongside Frisian.

4. **Frisian** — **defer past wave B.** It is three independent engine changes
   (8-direction capture topology, fractional shot-value comparator, stateful
   ≤3-consecutive-king-move rule). Highest risk, no cheap path. Revisit only as
   the anchor of a dedicated generalized-capture engine.

**Net wave-B deliverable:** 2 config tiles (Spanish ⭐, Italian) at +2 flags on
the existing engine, plus 1 new engine (Surakarta) with an arc-board renderer.
This corrects the Phase-4 roadmap's assumption (§5) that Spanish/Italian/Frisian
were all free config tiles: Spanish/Italian are cheap-but-not-free (+2 flags),
and Frisian is not a config tile at all.
