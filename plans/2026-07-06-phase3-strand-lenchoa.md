# Phase 3H+3I — Strand & Len Choa Implementation Plans (Phase-3 closers)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (one implementer per game, sequential; review each). Master-plan Global Constraints + ALL standing program rules apply (verification non-optional; sourced rules cited + tested; Display/parse single-sourced + round-trip; category + monochrome glyph guards; i18n ×6 0/0; wasm rebuild flow; audit all-ok; calibration via the example directly, scratch untracked; explicit-path commits with trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`; never push).

## 3H — Strand (the Isolation mechanic, renamed)

**Identity:** id `strand`, name `Strand`, icon 🏝️ + monochrome glyph, **`variantOf: 'trails'`** (the locked family decision — Strand joins the Trails family chip; do NOT add it to a CATEGORIES array; the guard accepts variantOf children). playerLabels `['Red', 'Gold']`. IP note: mechanic is from Isolation/Isola (1972) — the RULES are unprotectable and our name/theme differ; never use "Isolation"/"Isola" in user strings (code-comment citation fine).

**Rules (cite the Wikipedia Isolation (board game) page):** two pawns on a grid (classic 6×8; we default 7×7 for board symmetry — document the choice). Each turn has TWO parts: (1) move your pawn one step in any of 8 directions to an adjacent remaining tile; (2) remove ANY remaining empty tile from the board. A player who cannot move on their turn loses.

**Engine (`treant-wasm/src/strand.rs`):** state = two pawn positions + tile bitmask. Move encoding: `"<from>-<to>-<removed>"` (THREE parts, dash-separated — new encoding shape; Display/parse single-sourced + round-trip test mandatory). Branching = moves(≤8) × removable tiles(~45) ≈ 300 — fine for MCTS; solver ON for small boards only if cheap (it's fine to leave solver off if playouts are strong — decide by calibration and document; note Trails has solver ON, match it if feasible). `new(cols: u32, rows: u32)` clamps 5–8 each. Pawn starts: middle of opposite edges (classic Isolation) — cite.

**Tile:** presets `Classic 7×7 ⭐`, `Tight 5×5 ⚡`, `Grand 8×8 🤯`; knobs cols/rows 5–8. adopt `resultFlavor` ("🏝️ Marooned! {loser} had nowhere to step — {winner} wins"). **Board UI (the real work):** two-phase input — tap a highlighted destination to move (phase 1), then the board switches to "remove a tile" mode where all remaining empty tiles highlight and tapping one completes the move (phase 2). Requirements: a mid-move caption ("Now remove any tile 🕳️"); a way to CANCEL phase 1 before committing (tap your pawn again); the AI's two-part move must render comprehensibly — lastCells covers from/to/removed (three cells — verify the session diff picks up the removed tile as a change), and removed tiles render as holes (visually distinct from Trails' walls — different texture/darkness so the two family games read differently). Undo works (no chance).

**Tests:** two-part legality (can't remove occupied/pawn tiles; can't skip removal); blocked-mover-loses; round-trip on the 3-part encoding; full-game termination; ai_plays; if solver on, a small proven endgame.

## 3I — Len Choa (Thai tiger hunt)

**Identity:** id `len-choa`, name `Len Choa`, icon 🐅 + monochrome glyph, category Move & capture (hunt shelf with Bagh-Chal / Fox & Hounds). playerLabels `['Tiger', 'Leopards']` (note: Leopards ends in s — possessive banner branch handles it).

**Rules (cite Wikipedia Len Choa page + cyningstan's Len Choa page; they're short — encode exactly what they attest, note any conflicts):** triangular board of 10 points (verify point count from sources!). One tiger vs six leopards. Leopards enter by placement (one per turn) then move; tiger moves along lines one step, and CAPTURES by jumping a single adjacent leopard to the empty point beyond (chain jumps: verify from sources — encode what's attested; if unattested, single jumps only + note). Tiger starts on the apex (verify). Leopards win by immobilizing the tiger; tiger wins by capturing N leopards (sources vary between "enough that leopards can't win" — commonly capturing reduces below the immobilization threshold; verify and encode the attested number, likely all-but-… pick the attested form; if sources conflict, most-cited + note).

**Engine (`treant-wasm/src/lenchoa.rs`):** graph board (like worldthrees — reuse its per-board table + get_layout pattern; cite it as precedent). Asymmetric seats (like baghchal/foxhounds — study baghchal.rs for the hunt conventions: seat order, placement-then-move phase for the pack, capture bookkeeping). Move encoding: placement `"<point>"`, moves/jumps `"<from>-<to>"`; round-trip test. Solver ON (10-point board — tiny; another solver showcase). Draw-cap defensively.

**Tile:** presets `Classic ⭐` (attested setup), maybe `Patient Leopards` variant only if a knob is attested — otherwise NO knobs (an honest zero-knob tile is fine; the game is fixed-form). resultFlavor for the immobilization ending ("🐅 Cornered! The tiger can't move — Leopards win"). Board: SVG triangle from get_layout; tiger visually distinct (bigger/striped glyph vs leopard dots).

**Tests:** per-source point/adjacency asserts; placement phase; jump removes the leopard; tiger-capture win at the attested count; immobilization win; round-trip; ai_plays; solver endgame if cheap.

## Order & checkpoint

3H then 3I. Reviews as usual (rules spot-verification with WebFetch for Len Choa is mandatory — it's a folk game with thin sources; the reviewer must check the encoded board against BOTH cited pages). Then the critic's checkpoint 4 (focus: Strand's two-phase input comprehensibility + hole-vs-wall visual distinction from Trails; Len Choa's asymmetric fairness feel + whether Tiger vs Leopards reads instantly), then push. This closes Phase 3; next is the Phase-4 Draughts flagship plan.
