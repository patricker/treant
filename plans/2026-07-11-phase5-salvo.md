# Phase 5B — Salvo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. TWO implementer tasks (5B-1 engine, 5B-2 UI/tile), sequential, each reviewed; critic checkpoint 7; push. Master-plan Global Constraints + ALL standing program rules apply (verification non-optional; sourced rules; single-sourced encodings + round-trip; guards; i18n ×6; wasm rebuild flow; audit; explicit-path commits with trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`; never push).

**Goal:** Salvo — the public-domain Battleships paper game (WWI-era; "Battleship" is Hasbro's trademark for the name only, never use it in user strings; "Salvo" is the 1931 paper-era name — cite the Wikipedia Battleship (game) history section) — as the second hidden-info game, inheriting the 5A primitive wholesale (hiddenInfo blackout, getBoardFor per-seat views, noUndo, secrecy gates, seating guard, pre-handoff summary beat).

## Task 5B-1 — engine (`treant-wasm/src/salvo.rs`)

**Rules (verify from the Wikipedia page + one paper-rules source; quote load-bearing sentences):** each player secretly places a fleet on an n×n grid; players alternate firing; feedback per shot: miss / hit / (on last cell of a ship) "sunk + ship length"; win = all opponent ships sunk. SALVO variant (authentic 1931): you fire one shot per SURVIVING own ship, all called before feedback — verify the exact feedback protocol for salvo mode (per-shot or after-the-volley) and encode the attested form.

**Ctor:** `new(size: u32 /*6-15*/, s1: u32, s2: u32, s3: u32, s4: u32, s5: u32 /*fleet counts by length, each 0-6*/, touch: u32 /*ships may touch y/n*/, salvo: u32 /*0 = 1 shot/turn; 1 = per-surviving-ship volley*/)`. Validate feasibility: total ship cells ≤ ~55% of grid AND a constructive placement must exist (clamp/adjust with a documented policy — the engine must NEVER accept an unplaceable fleet; add a feasibility test at knob extremes).

**Moves:** placement `"p:<ship-index>:<cell>:<h|v>"`; shot `"s:<cell>"`; salvo volley = one move per shot (strict alternation bookkeeping handles multi-shot turns — current_player stays until the volley completes; this keeps the move surface uniform for undo-less replay and the session loop). Display/parse single-sourced + round-trip.

**Secrecy (the 5A bar):** `get_board_for(seat)` = own fleet + own shot map + received-shots map + PUBLIC feedback; opponent code... opponent SHIP CELLS never present pre-sink (sunk ships MAY reveal their cells — attested and desirable; verify). Never-cheats gunner: placement sampling + shot selection read ONLY the feedback history — the identical-feedback⇒identical-distribution test (5A pattern, adapted: assert the sampled candidate placements + heat map equal across two games with different true fleets but identical feedback).

**Gunner (determinized, honest):** sample K placements consistent with all feedback (constructive sampler with rejection; K scales with `playouts`), build a hit-probability heat map, fire per topK/temp sampling over the map (Easy = wide/sloppy, Hard = argmax with parity targeting). Hunt mode after unresolved hits (target adjacent cells) emerges naturally from consistency sampling — verify it does in a test (after a lone hit, the heat map must concentrate on its neighbors). Document honestly in the header: intelligence = determinization + heat map through the weak_move surface (master-plan sanctioned).

**Tests (minimum):** placement validation (bounds, overlap, touch rule both settings); feedback correctness incl. sunk-report; salvo volley bookkeeping (shot count = surviving ships, updates mid-game as ships sink); never-cheats; hunt-concentration; feasibility clamps; round-trip; full-game termination both modes; ai_plays. Audit-register (document if the fuzzer needs a fixed-seed placement mode). Difficulty: hand-set ladder rationale like 5A if calibration can't measure hidden-info (expected), with Hard pinned by a test (Hard sinks a classic 10×10 fleet in ≤ some sourced-or-derived bound — derive honestly from the heat-map policy, don't invent).

## Task 5B-2 — tile + UI

**Tile:** id `salvo`, name `Salvo`, icon 🚢 (+ monochrome glyph), category Family classics, `hiddenInfo: true`, `noUndo: true`, `variantOf` none (standalone flagship). Knobs: size 6–15, ships1–ships5 each 0–6 (the FLEET EDITOR — five knobs is a lot; group them under a "Fleet" heading if the knob renderer supports it, else clear labels "Dinghies (1)", "Boats (2)"... judgment), touch 0/1, salvo 0/1. Presets: `Classic ⭐` (10, fleet 0/1/2/1/1 — the classic 5-ship set: verify composition from source; carrier5/battleship4/2×cruiser3... encode the attested 1990 Milton-Bradley-adjacent PAPER set, cite), `Quick ⚡` (6×6 small fleet), `Dinghy Swarm 🤯` (12×12, s1=6, silly flagship), `True Salvo 🎩` (classic + salvo=1).

**Placement UI (the big build):** per-player secret placement phase behind blackouts — select a ship from a tray (lengths grouped, count badges), tap a cell to place, tap again to rotate, tap placed ship to pick it up; auto-random "🎲 Place for me" button (uses an engine helper — add `random_placement()` returning placement moves, seeded per game); "Lock in fleet 🔒" submits. Must be comfortable at 390px up to 15×15 (cells shrink; the tray must not overlap).

**Play UI:** dual view — "their waters" (your shot map, tap to fire) primary, "your fleet" (with received hits) secondary/toggleable; salvo mode shows remaining-shots-this-volley counter; hits/misses/sunk rendered distinctly (💥/·/wreck); resultFlavor win/loss ("⚓ {winner} sank the whole fleet!"); the 5A pre-handoff summary shows your volley's results before the blackout.

**Verification:** the 5A secrecy gates verbatim (opponent fleet absent from DOM pre-sink at every phase — the load-bearing grep), blackout coverage, full pass-and-play game, vs-AI Easy/Hard feel (Hard hunts after hits — Playwright-observable), fleet editor at extremes (feasibility clamps surface sanely in the preview), i18n ×6, guards, build.

## Gates

5B-1 review: adversarial secrecy + the sampler's consistency math + salvo bookkeeping + hunt test genuineness. 5B-2 review: placement UX cold, dual-grid legibility, secrecy DOM gates re-run, fleet-editor feasibility binding. Critic checkpoint 7: play it like a family (placement cold, volley flow, blackouts), try to cheat again (incl. sunk-reveal timing), Easy/Hard feel, Dinghy Swarm silliness verdict, whatever nobody asked. Then push.
