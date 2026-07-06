# Phase 3F+3G — Toads & Frogs & Pawn Duel Implementation Plans

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (one implementer per game, sequential; review each). Master-plan Global Constraints + all standing program rules apply (verification non-optional; sourced rules cited + tested; Display/parse single-sourced + round-trip; category + monochrome glyph guards; i18n ×6 0/0; wasm rebuild flow; audit all-ok; calibration via the example directly, scratch untracked; explicit-path commits with the `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` trailer; never push).

## 3F — Toads & Frogs (Conway)

**Identity:** id `toads-frogs`, name `Toads & Frogs`, icon 🐸 + monochrome glyph, category Brain-teasers, playerLabels `['Toads', 'Frogs']` (note: both end in s — the possessive-turn i18n branch handles this; verify the banner reads "Toads' turn").

**Rules (verify against the Wikipedia "Toads and Frogs" page; cite in engine comments):** 1-D row. Toads face right, Frogs face left. On your turn: move one of your creatures one square forward into an empty square, OR hop over exactly one ADJACENT opposing creature into the empty square immediately beyond. No backward moves, no hopping friends, no captures. Mover with no legal move loses (normal play).

**Engine (`treant-wasm/src/toadsfrogs.rs`):** near-trivial. `new(length: u32, toads: u32, frogs: u32, gaps: u32)` — clamps: length 4–15, toads/frogs 1–6 each, gaps 1–3, with `toads + frogs + gaps == length` enforced by deriving length or clamping counts (pick one, document; the UI must never produce an impossible combo — bind knobs accordingly). Start: toads packed left, frogs packed right, gaps between. Move encoding: origin cell index (destination is forced); Display bare index + round-trip test. Solver ON (CGT-solved game — the solver is the showcase). Multi-row "sums" from the knob bible: DEFER (note in tile comment) — ship single-row.

**Tile:** presets `Classic ⭐` (Wikipedia's canonical 3T-2gap-3F… verify the canonical small position; commonly TTT__FFF length 8), `Tiny ⚡` (T_F or TT_FF), `Long March 🤯` (6T 3gaps 6F, length 15). Knobs: toads 1–6, frogs 1–6, gaps 1–3 (length derived = toads+frogs+gaps; display via preview). Board: single row, big creature glyphs facing their direction (🐸 mirrored? use SVG/emoji with CSS flip so facing is visible — facing is the game's whole readability), tap a creature to move (destination forced ⇒ one-tap moves), head-shake on immovable, lastCells + winCells glow (winning move highlight n/a — box-in game: ADOPT the new `resultFlavor` seam from Slimetrail: "🐸 Stuck! {loser} had no move — {winner} wins").

**Tests:** slide + hop legality both directions; no-backward; no-friend-hop; blocked-mover-loses; round-trip; solver proves the canonical small position's known value if cheaply assertable (Wikipedia lists solved outcomes — assert the winner of TTT__FFF... verify from source; if the source's solved tables don't map cleanly, assert a hand-derived 4-cell endgame instead); ai_plays.

## 3G — Pawn Duel

**Identity:** id `pawn-duel`, name `Pawn Duel`, icon ♟️ + monochrome glyph, category Move & capture, playerLabels `['White', 'Black']`... NOTE the arcade convention is colour names matching seat palette — use `['Gold', 'Silver']`? NO: chess pawns read as chess; keep White/Black but ensure the board renders the pieces in the seat palette colours anyway (like Frontline's Red/Gold pawns — follow Frontline's pattern and use `['Red', 'Gold']` labels for palette coherence; the blurb says "chess pawns" for the mental model). FINAL: playerLabels `['Red', 'Gold']`, blurb references chess pawns.

**Rules (chess pawn subset — no external source needed beyond FIDE pawn rules, cite the FIDE handbook pawn article in a comment):** pawns only on an N-files × M-ranks board, one rank of pawns each (row 2 and row M-1 equivalents). Move: one forward to empty; optional two-forward from the starting rank (knob); capture one diagonally-forward; EN PASSANT (knob, default ON): a pawn that just double-stepped may be captured in passing on the immediately following move only. WIN: first promotion (reach the far rank) OR capturing all enemy pawns OR opponent has no legal move (stalemate-as-loss for the mover — document this deviation from chess stalemate; it's the Breakthrough convention and keeps the game decisive).

**Engine (`treant-wasm/src/pawnduel.rs`):** state = board + en-passant file (or none). `new(files: u32, ranks: u32, double_step: u32, en_passant: u32)` — files 4–10, ranks 5–8. Move encoding "from-to" (dash; round-trip test; en-passant target square is the DESTINATION, engine removes the passed pawn). Solver ON for small boards. Study frontline.rs first — reuse its patterns; this is Frontline plus double-step/en-passant/stalemate-rule.

**Tile:** presets `Classic 8×6 ⭐`, `Hexapawn 3×3 🎓` (double_step 0, en_passant 0 — Martin Gardner's teaching game as a preset!), `Wide 10×8 🤯`, `No Frills` (double/ep off). Knobs: files 4–10, ranks 5–8, doubleStep 0/1, enPassant 0/1. Board: reuse `makeMoveBoard('pawn')` from frontline if compatible (select-then-move with targets); en-passant destination must show as a legal target. resultFlavor for the stalemate ending ("No moves — {winner} wins").

**Tests:** double-step only from start rank; en-passant window (available exactly one ply, correct pawn removed); diagonal capture; promotion win; stalemate-as-loss; all-captured win; round-trip; ai_plays; solver proves Hexapawn 3×3 is a second-player win (KNOWN result — Gardner; this is a beautiful solver showcase test).

## Order & checkpoint

3F then 3G (sequential, review each). Then critic checkpoint on the pair (focus: Toads & Frogs facing-readability + the possessive banner; Pawn Duel en-passant comprehensibility + the Hexapawn preset's solver-perfect play), then push.
