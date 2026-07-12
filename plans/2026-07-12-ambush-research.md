# "Ambush" — rules + feasibility research (Stratego-family hidden-identity duel)

*Read-only research for the arcade master plan. 2026-07-12. Grounding read:
`treant-wasm/src/salvo.rs` (determinized gunner), `treant-wasm/src/bullscows.rs`
(closed-form deducer), `ARCADE.md` §7b (pass-screen / hidden-info primitive),
`GAME-IDEAS.md` §6n + §7A/B (Ambush entries).*

---

## 0. Verdict — GO WITH REDUCED SCOPE (and rename)

Ship it, but as a deliberately small L'Attaque-derivative, **not** under the name
"Ambush", and with **honest, hand-set difficulty** — its determinized AI will be
a competent-amateur bluffer, not a champion, and that's fine for the arcade.

- **Rules basis is clean.** The mechanics come from L'Attaque (patented 1908,
  public domain for a century) and are game *mechanics* (rank-beats-rank, flag
  capture, immovable bombs, spy-beats-top, scout long-move) which are not
  copyrightable. Only the **name** "Stratego" and Jumbo's specific *expression*
  (their rulebook text, piece art, "Stratego Quick"/"Barrage" trade dress) are
  protected. We avoid all of that trivially.
- **"Ambush" is the wrong name.** It collides head-on with *Ambush!* (Avalon
  Hill / Victory Games, 1983 — a still-catalogued, Origins-Award-winning war
  boardgame). Same product category, exact same word. **Rename.** Recommended:
  **"Colours"** (capturing the enemy colours = the flag-capture win; evocative,
  single-word, arcade-house-style like Salvo/Strand/Clobber) or **"Vanguard"**.
  Fallbacks: "Sappers", "Skirmish", "Redoubt". (Name-collision detail in §2.)
- **Reduced spec (the thing to build):** 8×8 board, two 1×2 central lakes,
  **12 pieces per side**, flag-capture / immobilisation win. Piece table in §4.
  A 6×7 "mini" preset and an 8×8 "standard" preset off one engine + an army
  composition editor (the §7A knob Peter already sketched).
- **Engine size:** ~1.4–1.8× `salvo.rs` (est. 1800–2400 lines). It's the most
  complex hidden-info game in the backlog: two-phase (placement + play), a real
  board with movement/combat, *and* determinized-with-lookahead AI. Not a
  weekend game.
- **Three highest risks:** (1) determinized-search **move quality** — PIMC over
  a real board with bluffing is theoretically pathological (strategy fusion);
  (2) **perf** — lookahead per sample in wasm under 1s/move is tight; (3)
  **placement-phase UX** — a 12-piece tray on a phone is fiddlier than Salvo's
  ships. All three are manageable at reduced scope; see §7.

If forced to one line: **build "Colours", 8×8, 12 pieces, determinized-PIMC AI
at hand-set difficulty, budget it as a large game.**

---

## 1. Rules lineage — what is public domain (sourced)

### L'Attaque (the public-domain basis)

From **Wikipedia, "L'Attaque"** (https://en.wikipedia.org/wiki/L%27Attaque):

> "On November 26 1908 Frenchwoman Hermance Edan filed a patent for a *'jeu de
> bataille avec pièces mobiles sur damier'*" — the patent was released in 1909
> and the game "began being sold by French game manufacturer Au Jeu Retrouvé in
> 1910 under the name 'L'Attaque'." Edan developed the concept in the 1880s.

Board & pieces (verbatim): *"a 9×10 board with three 2-square lakes in the
centre"*; each player gets *"36 standing rectangular cardboard pieces of various
ranks."* Ranks run **10 (Commander-in-chief) down to 1 (Spy), plus the Flag and
Mines.**

Combat (verbatim): *"the one with the lower rank is captured and removed from the
board. If the pieces are of equal rank, both are removed."* Special pieces:
Spy *"can capture the Commander-in-chief"*; Mines — *"any piece (except a Sapper)
attacking a Mine is removed from the game"*; Scouts *"can move any distance in a
non-diagonal straight line."*

Relationship to Stratego (verbatim): *"Stratego is a nearly identical game to
L'Attaque"* and *"the idea for it was likely derived from or transmitted as a
result of L'Attaque."*

**A 1908 French patent is long expired** (patents run ~20 years; this one lapsed
~1928). The design has been public domain for ~a century. Every mechanic we want
is attested here, pre-Stratego.

### The Stratego trademark (what we must never touch)

From **Wikipedia, "Stratego"** (https://en.wikipedia.org/wiki/Stratego):

> The name *"was registered as a trademark in 1942 by the Dutch company Van
> Perlstein & Roeper Bosch N.V."* Hausemann and Hotte later held it and
> sublicensed Milton Bradley for the US (1961). *"In 2009, Hausemann and Hotte
> was succeeded by Koninklijke Jumbo B.V. in the Netherlands"* — **Jumbo holds
> the trademark today** (Hasbro distributes in the US).

So: **the *word* "Stratego" is a live Jumbo trademark → NEVER user-facing**
(same rule as "Battleship" for Salvo, per that engine's header). The *mechanics*
predate it by 34 years and are uncopyrightable. Piece *names* like Marshal /
Scout / Spy / Miner / Bomb / Flag are generic English military words — not
individually protected — but we'll use **original names anyway** (§4) both for
flavour and to keep maximal daylight from Jumbo's rulebook expression.

### The deeper lineage (context, not needed for shipping)

The military-chess idea is much older — the Chinese **Jun Qi / Luzhanqi** (陸戰棋,
"land battle chess") family and older banqi-style hidden-piece games use the same
"pieces of hidden rank, higher rank captures lower, some pieces are bombs/mines,
capture the flag" skeleton. This reinforces that the *mechanics* are ancient folk
game-design, not any one company's IP. We don't need to source Luzhanqi in detail
— L'Attaque alone is a sufficient, well-documented public-domain basis.

---

## 2. Name collision assessment — drop "Ambush"

From search (BGG + Wikipedia): **"Ambush!"** is Avalon Hill / Victory Games'
1983 solitaire man-to-man WWII wargame (Eric Lee Smith / John Butterfield; won
the 1984 Origins Award for Best 20th-Century Boardgame; still catalogued, active
fan community, expansions "Move Out!" etc.).
(https://boardgamegeek.com/boardgame/1608/ambush ,
https://en.wikipedia.org/wiki/Ambush! )

- **Same category** (board wargame), **identical word**, a game with commercial
  recognition and an award. Even though the genres differ (solitaire wargame vs
  2p hidden-rank duel), reusing the bare title in the tabletop space is exactly
  the kind of avoidable collision the master plan's naming rule exists to
  prevent. "Ambush" is also an extremely common word (many apps/games use it),
  which cuts both ways but doesn't make it *safe* to adopt as our title.
- **Recommendation: do not ship as "Ambush."** Reconcile GAME-IDEAS.md, which
  itself flags this ("do a trademark pass on the chosen name" — §6n; and the
  §7A backlog entry uses "Ambush" only as a working label).

**Proposed names** (single evocative word, arcade house style):
1. **Colours** — *capture the enemy colours* = the flag win; period-military,
   clean, no known board-game collision in this space. **Top pick.**
2. **Vanguard** — strong, on-theme; check-but-likely-clear.
3. Fallbacks: **Sappers** (the bomb-defuser mechanic), **Skirmish**, **Redoubt**.

Whatever is picked, do the same live-mark sniff test that Salvo/Strand got before
it becomes user-facing (BGG search + basic USPTO/EUIPO glance for the *tabletop
games* class). Keep the working file/label internal until then.

---

## 3. Attested smaller variants (sizing evidence)

Classic Stratego is 10×10 / 40 pieces — far too big for a phone family game.
Attested smaller forms (from the Wikipedia "Stratego" variants section — cited
for *sizing precedent only*; we copy no rulebook text):

- **L'Attaque itself: 9×10 / 36 pieces** — already smaller than Stratego.
- **Stratego Duel: "10 pieces per side on an 8×10 board."**
- **Stratego Barrage / "Quick-Stratego": 8 pieces per side** (Jumbo's set:
  Flag, Marshal, General, 1 Bomb, 1 Miner, 2 Scouts, Spy), *"approximately 5
  minutes."*
- **Ultimate Lightning: "only 20 pieces" per side.**

Takeaway: **8–12 pieces per side on an 8×8–8×10 board is well-attested territory**
for a fast game. We land at **8×8 / 12 pieces** as the standard preset and a
**6×7 / ~8 pieces** mini. NOTE: the *existence* of Barrage's 8-piece count is a
fact we can cite; its *specific composition list and the name "Barrage/Quick"*
are Jumbo trade dress — we design our own composition (§4) and never reproduce
theirs.

---

## 4. Proposed rule set — reduced family version

**Board:** 8×8, two impassable 1×2 lakes centred on the middle two rows (rows
3–4 conceptually), splitting the no-go zone the way L'Attaque's three 2-square
lakes do. Each side's back **three** rows (24 squares) hold its 12 pieces at
setup. Mini preset: 6×7, one 1×2 lake, 8 pieces, back two rows.

**All names below are original.** The "maps to" column is design documentation,
not user-facing — do not surface the Stratego equivalents in the UI.

| Our name (original) | Count | Rank | Moves | Special mechanic | Maps to (internal only) |
|---|---|---|---|---|---|
| **Marshal** | 1 | 6 (top) | 1 orth. | — | (top commander) |
| **Captain** | 1 | 5 | 1 orth. | — | (mid officer) |
| **Sergeant** | 2 | 4 | 1 orth. | — | (mid officer) |
| **Trooper** | 2 | 3 | 1 orth. | — | (line soldier) |
| **Scout** | 1 | 2 | any dist. straight, non-diag, no jump | reveals as scout once it moves 2+ | Scout |
| **Sapper** | 2 | 1 | 1 orth. | **defuses Bombs** (captures them) | Miner |
| **Spy** | 1 | 0 (weakest) | 1 orth. | **beats the Marshal when *attacking*** (loses to all else) | Spy |
| **Bomb** | 1 | — | immovable | destroys ANY attacker except a Sapper | Bomb |
| **Standard** | 1 | — | immovable | **captured ⇒ that player loses** | Flag |

Total = 1+1+2+2+1+2+1+1+1 = **12 pieces.** (Composition is a knob — see §7A note
"army composition editor". The counts above are the default preset.)

**Core mechanics (all attested in §1, all uncopyrightable):**
- Movable pieces move **one orthogonal step** onto an empty square; the **Scout**
  moves any distance orthogonally through empty squares (no jumping).
- **Attack** = moving onto an enemy-occupied square. **Higher rank wins, loser
  removed; equal ranks, both removed.** (verbatim L'Attaque rule).
- **Both identities are revealed publicly on any combat** — this is the public
  feedback channel the AI and the pass-screen "combat summary" beat depend on.
- **Spy beats Marshal on the attack** (attested); Marshal beats Spy if Marshal
  attacks. **Sapper defuses Bomb** (attested). **Bomb** destroys any non-Sapper
  attacker. **Bomb and Standard are immovable.**
- **Win** by (a) capturing the enemy **Standard**, or (b) the opponent having
  **no legal move** (all movables gone / boxed in) — an immobilisation win, so
  this game needs the `resultFlavor` seam (§7b of ARCADE.md) to narrate it.
- **Movement inference is a rules consequence, not a special rule:** a piece that
  has ever moved cannot be a Bomb or Standard; a piece that moved 2+ in a line is
  a Scout. This is public knowledge → it constrains the AI's sampler (§5) and is
  the crux of the difficulty.
- **Optional knobs (all attested-mechanic or trivially neutral):** two-squares
  repetition rule (no shuffling a piece back and forth forever — needed for
  terminal guarantees), scout-long-move on/off, army composition editor, board
  6×7 / 8×8, no more-than-N-consecutive-advances (chase rule) to bound length.

---

## 5. AI feasibility — the honest assessment

**This is materially harder than Salvo, and the difference is the whole story.**

### Salvo vs Colours: what the sampler infers
- **Salvo** infers *placement*: "where are the ships." Its sampler rejection-fills
  fleets consistent with the shot log, then builds a **one-shot hit-probability
  heat map** and fires at the argmax. **No lookahead** — the value of a shot is
  just P(hit). That's why `salvo.rs`'s "search" is a documented no-op.
- **Colours** infers *identity assignment*: "which hidden rank is each enemy
  piece." The consistent-state space is the set of **assignments of the hidden
  rank multiset to the enemy's hidden piece positions**, constrained by:
  (a) **revealed identities** — every piece involved in past combat is publicly
  known; (b) **movement evidence** — a moved piece isn't a Bomb/Standard; a
  2+-straight mover is a Scout; (c) **multiset conservation** — captured pieces
  reduce the remaining pool.
- **Sampling is easy; evaluation is not.** Drawing a consistent assignment is a
  straightforward constructive/rejection sample (identical machinery to Salvo's
  constraint-respecting placer, but over a *permutation of ranks* rather than a
  *placement of ships*). The hard part: **the value of a move can't be read off a
  heat map** — it depends on how the game plays out (do I attack this unknown
  with my Captain? only if lookahead says the expected trade is good). **Colours
  needs search, Salvo did not.**

### The approach: determinized PIMC (Perfect-Information Monte Carlo) + vote
Standard, respectable technique and exactly what §6n/ARCADE.md anticipated
("sample hidden states … run treant on each sample, vote on the move"):

1. Sample **K** rank-assignments consistent with the seat's public+own info.
2. For each sample, the world becomes **perfect-information** → run a **shallow
   treant search** (or short rollouts) from the current position.
3. **Vote / average** move values across the K determinized worlds; play the
   best. This is *ensemble determinization* (Cowling et al., ISMCTS 2012) applied
   to a small board. The AI reads only public feedback + its own secret when
   building samples → the non-cheating property Salvo/Bulls&Cows already prove
   (`gunner_never_cheats`, `ai_is_a_pure_function_of_feedback`) carries over as a
   `colours_ai_never_peeks` test.

### The honest caveats (cite these in the difficulty tile)
- **PIMC is theoretically flawed.** It suffers **strategy fusion** (it assumes it
  can choose a *different* move in each determinized world, though in reality it
  must commit one move under uncertainty) and **non-locality**. See **Frank &
  Basin, "Search in games with incomplete information" (Artificial Intelligence,
  1998)** and **Long, Sturtevant, Buro & Furtak, "Understanding the Success of
  Perfect Information Monte Carlo Sampling in Game Tree Search" (AAAI 2010)** —
  the latter shows PIMC works *well* precisely in games with **low disambiguation
  factor and shallow "leaf correlation"**, i.e. where hidden info gets resolved
  steadily by play. Colours combat **publicly reveals both pieces every fight**,
  which steadily collapses the hidden state — this is favourable for PIMC. So the
  arcade toy sits on the *good* side of the PIMC applicability line, even though
  full Stratego does not.
- **DeepMind's DeepNash abandoned search — but that's the 10×10 game.** From the
  DeepMind blog (https://deepmind.google/blog/mastering-stratego-the-classic-game-of-imperfect-information/):
  Stratego's setup has *>10^66* states and a game tree of **10^535** (vs Go's
  10^360); *"a very successful AI technique called 'game tree search' … is not
  sufficiently scalable for Stratego,"* so DeepNash uses **model-free RL
  (Regularised Nash Dynamics)**, *not* MCTS (paper: arXiv:2206.15378). **This does
  NOT condemn our plan** — it condemns *full-size* search. On an **8×8 / 12-piece**
  board the tree is many orders of magnitude smaller, and prior *search-based*
  bots — **Probe** and **Master of the Flag II** (2009 Computer Stratego World
  Championship winner) — reached *amateur/decent-human* level on full Stratego
  with heuristic search + opponent modelling. DeepNash beat those older bots
  **≥97%**, but they are still perfectly respectable *family-arcade* opponents,
  and our board is far smaller than the one they struggled on.
- **Consequence:** ship with **hand-set difficulty** (nim/Bulls&Cows precedent,
  ARCADE.md §7b), not win-rate calibration. Register in `calibrate.rs` for the
  audit fuzzer, but don't promise championship strength. Difficulty ladder = K
  (samples) × search depth/rollout count × move-selection temperature. Easy = few
  noisy samples, shallow, softmax; Hard = many samples, deeper, argmax.

### Perf budget (the real constraint)
Target **<1s/move in wasm on a phone.** Rough envelope: K ≈ 16–40 samples ×
(shallow treant search: a few hundred playouts *or* depth-limited search of a few
plies) must fit ~1s. That's tight but plausible on an 8×8 board with ≤24 pieces —
Salvo already runs a 20k-node placer 160× per move. **Levers if it's slow:**
smaller K on Easy, cap search depth, prune obviously-safe moves, reuse samples
across a turn. **Risk item #2** — needs a wasm timing spike before committing.

---

## 6. UX shape — reuse map

The pass-screen / hidden-info primitive (ARCADE.md §7b) was **built for exactly
this**; most of the plumbing already exists.

| Concern | Reuse (existing primitive) | New work |
|---|---|---|
| Per-seat views | `hiddenInfo:true` → `getBoardFor(seat)`; engine `get_board_for` emits own ranks + public info, enemy hidden pieces as anonymous backs, revealed pieces show rank | `get_board_for` for a *board* (Salvo did fleet+shots; here it's an 8×8 with per-piece "known?" flags) |
| Pass-and-play blackout | Full-viewport handoff between human seats, `noUndo`, Escape-inert — all automatic | none (combat reveals are public, so the incoming seat legitimately sees them) |
| vs-AI (no blackout) | Lone-human fixed-seat view, AI never peeks | none |
| Combat reveal beat | — | **New:** a "combat summary" moment (attacker rank vs defender rank, who's removed). Public info; show it to both seats — the one genuinely new UX element |
| Placement phase | Salvo's **tray-placement pattern** (drag pieces into your setup zone) is the closest analogue | **New:** 12 distinct ranked pieces into a 3-row zone is fiddlier than Salvo's ships; needs a rank palette + auto-fill/randomise button. **Risk item #3** |
| Board rendering | Flat-grid board conventions (dark palette, `lastCells` marker, seat-colour tint) | New board component (pieces show rank glyph to owner, back to opponent) |
| Result narration | `resultFlavor` seam | **New:** narrate Standard-capture vs immobilisation wins |
| Non-cheating proof | `ai_is_a_pure_function_of_feedback` / `gunner_never_cheats` test pattern | New `colours_ai_never_peeks` test (two games, different hidden ranks, identical public history → identical AI move) |
| Difficulty | Hand-set ladder + `calibrate.rs` registration (nim/Bulls&Cows precedent) | new tile copy |

**Cadence note:** blackout **every turn** in pass-and-play (each player's own
piece identities are secret every turn), but because combat is public, the
between-turns blackout should *first* show the shared combat summary, *then* hand
off. The §7b "2+ humans AND any AI" rejection still applies — Colours is 2p so
it's fine, but if a 3p+ variant is ever wanted, the generalisation caveat in §7b
must be done first.

---

## 7. Scope verdict — precise reduced spec + risks

**GO, at reduced scope, renamed.** Concretely, build:

- **Name:** "Colours" (or "Vanguard") — NOT "Ambush", NOT anything with
  "Stratego". Live-mark check before user-facing.
- **Board:** 8×8 standard preset, two 1×2 lakes; **6×7 mini** preset (1 lake).
- **Pieces:** 12/side per §4 table (Marshal, Captain, 2 Sergeant, 2 Trooper,
  Scout, 2 Sapper, Spy, Bomb, Standard); mini ≈ 8/side. Army-composition editor
  as a knob.
- **Win:** capture enemy Standard, or opponent immobilised.
- **AI:** determinized PIMC — sample K consistent rank-assignments, shallow
  treant search per sample, vote; non-cheating; **hand-set difficulty**;
  registered in `calibrate.rs`.
- **Engine size:** ~**1.4–1.8× `salvo.rs`** (≈1800–2400 lines) — two-phase game,
  real board + movement/combat, and lookahead-bearing determinization. Treat as a
  **large** game, the capstone of the hidden-info trilogy (Bulls & Cows → Salvo →
  Colours), each strictly harder than the last.

**Top three risks (ranked):**
1. **Determinized-search move quality / bluffing coherence.** PIMC's strategy
   fusion can make the AI play incoherently around unknowns (won't bluff or
   probe like a human). *Mitigation:* lean on the favourable PIMC regime (public
   combat reveals collapse hidden state fast), ship at hand-set difficulty, be
   honest in the tile. Accept "competent amateur", not "expert".
2. **Wasm perf < 1s/move.** K samples × per-sample lookahead is the cost centre.
   *Mitigation:* timing spike first; scale K/depth by difficulty; prune safe
   moves; reuse samples per turn.
3. **Placement-phase UX on a phone.** 12 ranked pieces into a 3-row zone is more
   complex than Salvo's tray. *Mitigation:* rank palette + "randomise/auto-fill"
   + preset armies; validate against the setup-preview convention (§7b).

**Do NOT** attempt full 10×10/40. **Do NOT** ship as "Ambush". **Do** build it as
the third and hardest hidden-info game once Salvo is proven in production.

---

## 8. Reconciliation with GAME-IDEAS.md

- **§6n** lists *"Ambush (L'Attaque mechanic, 1908 — Stratego® is the modern
  trademark) · 2p · ★★ · 🔴 · ⚠️ determinized … Start as a 6×7 'mini'
  (Stratego Duel-sized) … Flag: do a trademark pass on the chosen name."* This
  research **confirms** the lineage and difficulty flags and **discharges the
  trademark-pass action item: the working name "Ambush" fails** (Ambush!, Avalon
  Hill 1983) → rename to Colours/Vanguard. The "6×7 mini, Duel-sized" instinct is
  well-founded (Stratego Duel = 10 pieces/8×10; we go 8/6×7 mini + 12/8×8
  standard).
- **§7A** lists *"Ambush — board 6×7 mini → 8×8 · army composition editor 🙃
  (eight bombs, zero scouts — why not) · scout long-move y/n."* This research
  **adopts all three knobs** into §4/§7 and confirms the 6×7→8×8 progression.
- Net: GAME-IDEAS needs a one-line update at ship time — replace "Ambush" with
  the chosen name and drop the "do a trademark pass" flag (done, it failed).
  (Not edited here — this is read-only research; flag it for the build task.)

---

## Sources
- L'Attaque — https://en.wikipedia.org/wiki/L%27Attaque
- Stratego (history, ruleset, variants, Jumbo trademark) — https://en.wikipedia.org/wiki/Stratego
- Ambush! (1983 Avalon Hill/Victory Games) — https://boardgamegeek.com/boardgame/1608/ambush , https://en.wikipedia.org/wiki/Ambush!
- DeepMind, "Mastering Stratego, the classic game of imperfect information"
  (10^535 tree, MCTS not scalable, R-NaD) — https://deepmind.google/blog/mastering-stratego-the-classic-game-of-imperfect-information/
- Perolat et al., "Mastering the Game of Stratego with Model-Free Multiagent
  Reinforcement Learning" — https://arxiv.org/abs/2206.15378 (DeepNash beats
  Probe / Master of the Flag / Demon of Ignorance etc. ≥97%)
- PIMC theory (cited from literature, not URL-sourced here): Frank & Basin,
  "Search in games with incomplete information", *Artificial Intelligence* 1998;
  Long, Sturtevant, Buro & Furtak, "Understanding the Success of Perfect
  Information Monte Carlo Sampling in Game Tree Search", AAAI 2010; Cowling,
  Powley & Whitehouse, "Information Set Monte Carlo Tree Search", IEEE TCIAIG 2012.
