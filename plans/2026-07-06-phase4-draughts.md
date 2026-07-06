# Phase 4 — Draughts Flagship Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. TWO implementer tasks (4A engine, 4B tiles+UI), sequential, each reviewed; then critic checkpoint + push. Master-plan Global Constraints + ALL standing program rules apply (verification non-optional; sourced rules cited + tested; single-sourced move encoding + round-trip; guards; i18n ×6; wasm rebuild flow; audit; calibration via example; explicit-path commits with trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`; never push).

**Goal:** ONE draughts engine with rule flags, shipping SIX named national variants as config-only tiles (wave A): **Draughts (American) ⭐, International (10×10), Brazilian, Pool, Russian, Giveaway 🙃**. Spanish/Italian (capture-priority tie-breaks) and Frisian (orthogonal captures, weighted priorities) are explicitly WAVE B — their flags are designed-for but not implemented; do not attempt them.

**Source of truth for variant rules:** the FMJD variants guide PDF + per-variant Wikipedia pages (American/English draughts, International draughts, Brazilian, Russian, Pool checkers, Suicide checkers). WebFetch and cite per rule. The knob bible (plans master + GAME-IDEAS §6x) lists the flag mapping — verify each against sources before encoding.

## Task 4A — the engine (`treant-wasm/src/draughts.rs`)

**Flags (ctor):** `new(size: u32 /*8|10|12*/, men_rows: u32 /*2-4*/, flying_kings: u32, men_capture_back: u32, max_capture: u32 /*0 off, 1 most-pieces*/, promote_mid_chain: u32, misere: u32)`. Wave-A variant table (verify each cell from sources, encode as a tested preset-config in comments):

| Variant | size | rows | flying | menBack | maxCap | midChain | misère |
|---|---|---|---|---|---|---|---|
| American ⭐ | 8 | 3 | no | no | no (forced, any) | n/a (no) | no |
| International | 10 | 4 | yes | yes | yes | no (pass-through doesn't promote) | no |
| Brazilian | 8 | 3 | yes | yes | yes | no | no |
| Pool | 8 | 3 | yes | yes | NO (any capture ok) | no | no |
| Russian | 8 | 3 | yes | yes | no | YES (promotes mid-chain, continues as king) | no |
| Giveaway 🙃 | 8 | 3 | no | no | no | n/a | YES (lose all pieces / be blocked = WIN) |

**Core machinery (the real work):** capture-chain generation — a capture is a full atomic path; when captures exist they are FORCED (all wave-A variants); `max_capture` filters to maximum-length chains (piece-count; kings-priority tie-breaks are wave B); flying kings slide any distance and capture at distance landing anywhere beyond; captured pieces are removed only at chain end (International rule — a jumped piece can't be jumped twice but still blocks; verify + test this "Turkish stroke" subtlety); promotion at far row (mid-chain per flag); misère inverts the terminal only (mover with no move: normal = loses, misère = wins; fewest-pieces framing comes free).

**Move encoding:** full path, dash-joined (`"12-19"` step, `"12-19-26"` chain, long chains longer) — atomic and replay-safe; Display/parse single-sourced + round-trip test over generated chains. Board string: 'm'/'M' dark-square men, 'k'/'K' kings? NO — follow the arcade convention: one char per cell over the FULL grid, ' ' empty, 'x'/'X' seat-0 man/king, 'o'/'O' seat-1 man/king (document; the Board maps case→crown).

**Termination:** captures/promotions strictly progress; king-shuffle endgames can cycle → draw after 40 plies without a capture or man-move (attested-adjacent; cite the real 25/40-move conventions and document our choice). Solver OFF (long games; document). Eval: material (man=2, king=3 or source-informed) + mobility — a REAL eval this time (rollouts alone are weak at draughts; keep it simple but non-zero; calibrate honestly).

**Tests (minimum):** per-variant config table asserts; chain generation incl. multi-jump forced + max-capture filtering; the can't-jump-twice-but-still-blocks rule; flying-king capture geometry; mid-chain promotion (Russian) vs not (International — a man passing the far row mid-chain does NOT promote; verify + test); misère terminal; round-trip over chains; 40-ply draw cap; ai_plays per variant; a known tactical position per variant if sources give one cheaply.

## Task 4B — tiles + board UI

Six config-only tiles (the §6 policy — findable by name): ids `draughts` (American, name "Draughts", blurb says checkers), `international-draughts`, `brazilian-draughts`, `pool-checkers`, `russian-draughts`, `giveaway-checkers`. Family: `variantOf: 'draughts'` on the five children (first ≥4 family — the bottom-sheet path of renderFamily MAY trigger: the seam was stubbed to fall back inline; either implement the sheet now or verify the inline fallback is acceptable at 5 children and note it — coordinator's call at review). Category: Family classics (parent only). Each tile: correct flags in create(), 2-3 presets (board-size variations where the variant attests them), knobs exposing the flags that make sense per tile (e.g. the parent exposes ALL flags for go-crazy custom; children pin their identity flags and expose size only — judgment, document). Icons: parent glyph + small variant differentiators. rules.ts per variant (short, name the signature rule). i18n ×6.

**Board UI:** dark-square board rendering (checkerboard!); select-then-move with CHAIN input — tapping a piece shows its legal chain first-steps; tapping a step continues showing forced continuations until the chain completes (build the full path, submit atomically); a chain-in-progress indicator + cancel (learn from Strand's two-phase pattern); kings get crowns; capture animations optional. lastCells covers the full chain. resultFlavor for blocked endings ("No moves — {winner} wins") and Giveaway's inverted framing ("🙃 {winner} gave everything away first!").

**Calibration:** each of the six registered in calibrate.rs; run the example per variant; paste sane ladders (the eval should make these non-degenerate; if Hard plateaus early per the documented policy, Hard = max rung anyway).

## Gates

4A review scrutiny: chain machinery vs sources (spot-verify International's blocking rule + Russian mid-chain against Wikipedia), encoding round-trip, draw-cap soundness, eval sanity. 4B review: variant-flag fidelity per tile, chain UX cold-comprehensibility, family-at-5 rendering, i18n. Critic checkpoint 5: play American + International + Giveaway vs Hard; judge chain input as a first-timer; family sheet/inline at 5 children; whatever nobody asked. Then the Phase-4 push.
