# AI difficulty calibration results

Full self-play round-robin over all 30 harness games (`treant-wasm/examples/calibrate.rs`,
seed `0xC0FFEE`, n=20 games/pair, alternating first move). Method, mechanism, and the
auto-selection rule are documented in [`CALIBRATION.md`](../CALIBRATION.md).

Re-run with `scripts/calibrate.sh` (all games) or `scripts/calibrate.sh <game> <n>` (one game).

Ladders (per-game; branchy games use LIGHT to stay tractable):
- **STD**  = p8/t3 · p30/t2 · p100/t1 · p300/t0.6 · p800/t0.35 · p2000/t0
- **LIGHT** = p8/t3 · p30/t2 · p80/t1 · p200/t0.5 · p500/t0.2  (gomoku, amazons, connect-six)

Tier format: `{playouts, topK, temp}`. Easy = weakest rung; Hard = cheapest rung within
5% of peak field strength (auto-caps solver games low, lets open games climb); Medium = the
rung nearest the Easy↔Hard midpoint.

## Selected difficulty per game

| Game | Easy | Medium | Hard | Source |
|---|---|---|---|---|
| connect-four | {8,6,3} | {30,5,2} | {300,3,0.6} | measured |
| tic-tac-toe | {8,6,3} | {30,5,2} | {100,4,1} | measured |
| gomoku | {8,6,3} | {30,5,2} | {80,4,1} | measured |
| reversi | {8,6,3} | {300,3,0.6} | {2000,1,0} | measured |
| hex | {8,6,3} | {100,4,1} | {2000,1,0} | measured |
| frontline | {8,6,3} | {100,4,1} | {800,2,0.35} | measured |
| mancala | {8,6,3} | {100,4,1} | {2000,1,0} | measured |
| dots-and-boxes | {8,6,3} | {100,4,1} | {2000,1,0} | measured |
| clobber | {30,5,2} | {100,4,1} | {2000,1,0} | measured |
| konane | {8,6,3} | {300,3,0.6} | {2000,1,0} | measured |
| shift | {8,6,3} | {300,3,0.6} | {800,2,0.35} | measured |
| pig | {8,6,3} | {30,5,2} | {2000,1,0} | measured |
| chomp | {8,6,3} | {100,4,1} | {800,2,0.35} | measured |
| wythoff | {8,6,3} | {30,5,2} | {100,4,1} | measured |
| trails | {8,6,3} | {300,3,0.6} | {2000,1,0} | measured |
| first-capture | {8,6,3} | {100,4,1} | {2000,1,0} | measured |
| sim | {30,5,2} | {300,3,0.6} | {2000,1,0} | hand — noise-dominated (avoidance game, ~no MCTS gradient) → monotonic ladder, temp is the only felt lever |
| mu-torere | {8,6,3} | {100,4,1} | {800,2,0.35} | measured |
| domineering | {8,6,3} | {30,5,2} | {300,3,0.6} | measured |
| nogo | {8,6,3} | {30,5,2} | {800,2,0.35} | measured |
| col | {8,6,3} | {100,4,1} | {2000,1,0} | measured |
| amazons | {8,6,3} | {30,5,2} | {200,3,0.5} | measured |
| fox-hounds | {30,5,2} | {300,3,0.6} | {2000,1,0} | measured |
| treblecross | {30,5,2} | {100,4,1} | {2000,1,0} | measured |
| euclid | {8,6,3} | {30,4,1} | {100,1,0} | hand — plateaus at p30 (solver-tiny); auto-emit had easy==medium, so medium/hard split on temp instead |
| connect-six | {8,6,3} | {80,4,1} | {200,3,0.5} | measured |
| trap-three | {8,6,3} | {100,4,1} | {2000,1,0} | measured |
| no-tac-toe | {8,6,3} | {100,4,1} | {2000,1,0} | measured |
| square-up | {8,6,3} | {100,4,1} | {2000,1,0} | measured |
| order-chaos | {8,6,3} | {100,4,1} | {800,2,0.35} | measured |
| nim | {8,6,3} | {30,4,1} | {100,1,0} | hand — no uniform WASM API; by analogy to measured tiny solver games |
| subtract-square | {8,6,3} | {30,4,1} | {100,1,0} | hand — no uniform WASM API; by analogy to measured tiny solver games |

**Not calibrated:** 2048 is single-player (no AI difficulty levels).

## Notes on degenerate matrices

- **mancala** initially produced an all-50% (flat) matrix: its WASM `result()` returns `"P1"`/`"P2"` while every other engine returns `"1"`/`"2"`, so the harness scored every game a draw. Fixed by stripping a leading `P` in `play()`; the corrected matrix below shows a real 6%→85% gradient.
- **sim** shows no measurable strength gradient (field scores all cluster 42–58%): pure-rollout MCTS gains almost nothing on Sim, an avoidance game decided by late parity. Hand-set to a sensible monotonic ladder rather than the noise-driven auto-pick (which inverted Easy/Hard).
- **euclid** plateaus immediately — everything ≥p30 is statistically equal, only p8 is weak. Auto-pick gave easy==medium; hand-split medium/hard on `temp` (the documented lever for solver games).

## Raw output

```
Finished `release` profile [optimized] target(s) in 0.10s
     Running `target/release/examples/calibrate`

================  connect-four  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      15%       5%       0%       5%       2%       6%
      p30 t2      85%        ·      12%       0%      12%      18%      26%
     p100 t1      95%      88%        ·      38%      40%      38%      60%
   p300 t0.6     100%     100%      62%        ·      48%      42%      70%
  p800 t0.35      95%      88%      60%      52%        ·      55%      70%
    p2000 t0      98%      82%      62%      58%      45%        ·      69%
  >>> paste into docs/src/components/arcade/games/<connect-four>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 300, topK: 3, temp: 0.6 },
  },
  (Hard capped at p300: strength peaked early — extra playouts add nothing.)

================  tic-tac-toe  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      38%      30%      18%      15%      28%      26%
      p30 t2      62%        ·      25%      40%      25%      28%      36%
     p100 t1      70%      75%        ·      60%      42%      50%      60%
   p300 t0.6      82%      60%      40%        ·      50%      50%      56%
  p800 t0.35      85%      75%      58%      50%        ·      50%      64%
    p2000 t0      72%      72%      50%      50%      50%        ·      59%
  >>> paste into docs/src/components/arcade/games/<tic-tac-toe>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 100, topK: 4, temp: 1 },
  },
  (Hard capped at p100: strength peaked early — extra playouts add nothing.)

================  gomoku  (n=20/pair)  ================
     rung\vs       p8      p30      p80     p200     p500    field
       p8 t3        ·      20%       0%       0%       8%       7%
      p30 t2      80%        ·      15%      20%      25%      35%
      p80 t1     100%      85%        ·      48%      50%      71%
   p200 t0.5     100%      80%      52%        ·      48%      70%
   p500 t0.2      92%      75%      50%      52%        ·      68%
  >>> paste into docs/src/components/arcade/games/<gomoku>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 80, topK: 4, temp: 1 },
  },
  (Hard capped at p80: strength peaked early — extra playouts add nothing.)

================  reversi  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      22%       5%       5%       0%       0%       6%
      p30 t2      78%        ·      35%       5%       5%       0%      24%
     p100 t1      95%      65%        ·      15%       5%       0%      36%
   p300 t0.6      95%      95%      85%        ·      10%       0%      57%
  p800 t0.35     100%      95%      95%      90%        ·      20%      80%
    p2000 t0     100%     100%     100%     100%      80%        ·      96%
  >>> paste into docs/src/components/arcade/games/<reversi>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  hex  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      50%       0%       0%       0%       0%      10%
      p30 t2      50%        ·       0%       0%       0%       0%      10%
     p100 t1     100%     100%        ·      15%       5%       0%      44%
   p300 t0.6     100%     100%      85%        ·      20%      15%      64%
  p800 t0.35     100%     100%      95%      80%        ·      20%      79%
    p2000 t0     100%     100%     100%      85%      80%        ·      93%
  >>> paste into docs/src/components/arcade/games/<hex>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  frontline  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      15%       0%       0%       0%       0%       3%
      p30 t2      85%        ·      10%       0%       0%       0%      19%
     p100 t1     100%      90%        ·      15%       5%      15%      45%
   p300 t0.6     100%     100%      85%        ·      20%      20%      65%
  p800 t0.35     100%     100%      95%      80%        ·      55%      86%
    p2000 t0     100%     100%      85%      80%      45%        ·      82%
  >>> paste into docs/src/components/arcade/games/<frontline>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  (Hard capped at p800: strength peaked early — extra playouts add nothing.)

================  mancala  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      45%      10%       0%       0%       0%      11%
      p30 t2      55%        ·      45%      20%       2%       5%      26%
     p100 t1      90%      55%        ·      42%      35%      25%      50%
   p300 t0.6     100%      80%      58%        ·      35%       8%      56%
  p800 t0.35     100%      98%      65%      65%        ·      38%      73%
    p2000 t0     100%      95%      75%      92%      62%        ·      85%
  >>> paste into docs/src/components/arcade/games/<mancala>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  dots-and-boxes  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      20%       0%       0%       0%       0%       4%
      p30 t2      80%        ·       5%       0%       0%       0%      17%
     p100 t1     100%      95%        ·      20%       5%       0%      44%
   p300 t0.6     100%     100%      80%        ·      20%      20%      64%
  p800 t0.35     100%     100%      95%      80%        ·      20%      79%
    p2000 t0     100%     100%     100%      80%      80%        ·      92%
  >>> paste into docs/src/components/arcade/games/<dots-and-boxes>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  clobber  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      55%      35%      20%      25%      15%      30%
      p30 t2      45%        ·      25%      15%      25%      25%      27%
     p100 t1      65%      75%        ·      30%      15%      15%      40%
   p300 t0.6      80%      85%      70%        ·      60%      35%      66%
  p800 t0.35      75%      75%      85%      40%        ·      35%      62%
    p2000 t0      85%      75%      85%      65%      65%        ·      75%
  >>> paste into docs/src/components/arcade/games/<clobber>.tsx:
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  konane  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      45%      25%      20%      20%      10%      24%
      p30 t2      55%        ·      45%      25%      25%      10%      32%
     p100 t1      75%      55%        ·      30%      15%      30%      41%
   p300 t0.6      80%      75%      70%        ·      35%      15%      55%
  p800 t0.35      80%      75%      85%      65%        ·      35%      68%
    p2000 t0      90%      90%      70%      85%      65%        ·      80%
  >>> paste into docs/src/components/arcade/games/<konane>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  shift  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      35%       5%       0%       0%       0%       8%
      p30 t2      65%        ·      30%       0%       0%       0%      19%
     p100 t1      95%      70%        ·       5%       0%       0%      34%
   p300 t0.6     100%     100%      95%        ·       0%       0%      59%
  p800 t0.35     100%     100%     100%     100%        ·      52%      90%
    p2000 t0     100%     100%     100%     100%      48%        ·      90%
  >>> paste into docs/src/components/arcade/games/<shift>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  (Hard capped at p800: strength peaked early — extra playouts add nothing.)

================  pig  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·       5%      10%      15%       0%       5%       7%
      p30 t2      95%        ·      10%      10%      25%      10%      30%
     p100 t1      90%      90%        ·      40%      40%      30%      58%
   p300 t0.6      85%      90%      60%        ·      60%      35%      66%
  p800 t0.35     100%      75%      60%      40%        ·      50%      65%
    p2000 t0      95%      90%      70%      65%      50%        ·      74%
  >>> paste into docs/src/components/arcade/games/<pig>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  chomp  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      50%      15%       0%       0%       0%      13%
      p30 t2      50%        ·      25%       5%       0%       0%      16%
     p100 t1      85%      75%        ·      20%      10%       5%      39%
   p300 t0.6     100%      95%      80%        ·      35%      20%      66%
  p800 t0.35     100%     100%      90%      65%        ·      50%      81%
    p2000 t0     100%     100%      95%      80%      50%        ·      85%
  >>> paste into docs/src/components/arcade/games/<chomp>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  (Hard capped at p800: strength peaked early — extra playouts add nothing.)

================  wythoff  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      45%      20%      15%      25%      35%      28%
      p30 t2      55%        ·      50%      50%      50%      50%      51%
     p100 t1      80%      50%        ·      50%      50%      50%      56%
   p300 t0.6      85%      50%      50%        ·      50%      50%      57%
  p800 t0.35      75%      50%      50%      50%        ·      50%      55%
    p2000 t0      65%      50%      50%      50%      50%        ·      53%
  >>> paste into docs/src/components/arcade/games/<wythoff>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 100, topK: 4, temp: 1 },
  },
  (Hard capped at p100: strength peaked early — extra playouts add nothing.)

================  trails  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      35%      20%      10%       0%       5%      14%
      p30 t2      65%        ·      15%      15%       0%       0%      19%
     p100 t1      80%      85%        ·      10%      15%       0%      38%
   p300 t0.6      90%      85%      90%        ·      15%      15%      59%
  p800 t0.35     100%     100%      85%      85%        ·      25%      79%
    p2000 t0      95%     100%     100%      85%      75%        ·      91%
  >>> paste into docs/src/components/arcade/games/<trails>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  first-capture  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      20%       0%       0%       0%       0%       4%
      p30 t2      80%        ·       0%       0%       0%       0%      16%
     p100 t1     100%     100%        ·      15%       0%       5%      44%
   p300 t0.6     100%     100%      85%        ·      60%      10%      71%
  p800 t0.35     100%     100%     100%      40%        ·      20%      72%
    p2000 t0     100%     100%      95%      90%      80%        ·      93%
  >>> paste into docs/src/components/arcade/games/<first-capture>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  sim  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      55%      40%      45%      45%      55%      48%
      p30 t2      45%        ·      45%      50%      50%      75%      53%
     p100 t1      60%      55%        ·      70%      45%      60%      58%
   p300 t0.6      55%      50%      30%        ·      50%      45%      46%
  p800 t0.35      55%      50%      55%      50%        ·      55%      53%
    p2000 t0      45%      25%      40%      55%      45%        ·      42%
  >>> paste into docs/src/components/arcade/games/<sim>.tsx:
  difficulty: {
    easy: { playouts: 2000, topK: 1, temp: 0 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 30, topK: 5, temp: 2 },
  },
  (Hard capped at p30: strength peaked early — extra playouts add nothing.)

================  mu-torere  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      25%      15%       0%       0%       0%       8%
      p30 t2      75%        ·      25%       0%       0%       0%      20%
     p100 t1      85%      75%        ·       5%       0%       0%      33%
   p300 t0.6     100%     100%      95%        ·      42%      40%      76%
  p800 t0.35     100%     100%     100%      58%        ·      50%      82%
    p2000 t0     100%     100%     100%      60%      50%        ·      82%
  >>> paste into docs/src/components/arcade/games/<mu-torere>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  (Hard capped at p800: strength peaked early — extra playouts add nothing.)

================  domineering  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      60%      20%      10%      45%      20%      31%
      p30 t2      40%        ·      70%      55%      30%      40%      47%
     p100 t1      80%      30%        ·      50%      50%      55%      53%
   p300 t0.6      90%      45%      50%        ·      50%      55%      58%
  p800 t0.35      55%      70%      50%      50%        ·      35%      52%
    p2000 t0      80%      60%      45%      45%      65%        ·      59%
  >>> paste into docs/src/components/arcade/games/<domineering>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 300, topK: 3, temp: 0.6 },
  },
  (Hard capped at p300: strength peaked early — extra playouts add nothing.)

================  nogo  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      45%      30%      45%      25%      40%      37%
      p30 t2      55%        ·      40%      25%      45%      55%      44%
     p100 t1      70%      60%        ·      35%      20%      35%      44%
   p300 t0.6      55%      75%      65%        ·      45%      30%      54%
  p800 t0.35      75%      55%      80%      55%        ·      40%      61%
    p2000 t0      60%      45%      65%      70%      60%        ·      60%
  >>> paste into docs/src/components/arcade/games/<nogo>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  (Hard capped at p800: strength peaked early — extra playouts add nothing.)

================  col  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      30%      50%      35%      35%      15%      33%
      p30 t2      70%        ·      40%      55%      55%      30%      50%
     p100 t1      50%      60%        ·      50%      30%      45%      47%
   p300 t0.6      65%      45%      50%        ·      60%      55%      55%
  p800 t0.35      65%      45%      70%      40%        ·      40%      52%
    p2000 t0      85%      70%      55%      45%      60%        ·      63%
  >>> paste into docs/src/components/arcade/games/<col>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  amazons  (n=20/pair)  ================
     rung\vs       p8      p30      p80     p200     p500    field
       p8 t3        ·      50%      40%      25%      40%      39%
      p30 t2      50%        ·      45%      55%      30%      45%
      p80 t1      60%      55%        ·      25%      40%      45%
   p200 t0.5      75%      45%      75%        ·      40%      59%
   p500 t0.2      60%      70%      60%      60%        ·      62%
  >>> paste into docs/src/components/arcade/games/<amazons>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 30, topK: 5, temp: 2 },
    hard: { playouts: 200, topK: 3, temp: 0.5 },
  },
  (Hard capped at p200: strength peaked early — extra playouts add nothing.)

================  fox-hounds  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      40%      45%      30%      20%      20%      31%
      p30 t2      60%        ·      25%      15%      30%      15%      29%
     p100 t1      55%      75%        ·      40%      45%      15%      46%
   p300 t0.6      70%      85%      60%        ·      45%      25%      57%
  p800 t0.35      80%      70%      55%      55%        ·      35%      59%
    p2000 t0      80%      85%      85%      75%      65%        ·      78%
  >>> paste into docs/src/components/arcade/games/<fox-hounds>.tsx:
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 300, topK: 3, temp: 0.6 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  treblecross  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      35%      55%      50%      55%      30%      45%
      p30 t2      65%        ·      20%      40%      55%      20%      40%
     p100 t1      45%      80%        ·      50%      55%      35%      53%
   p300 t0.6      50%      60%      50%        ·      50%      30%      48%
  p800 t0.35      45%      45%      45%      50%        ·      35%      44%
    p2000 t0      70%      80%      65%      70%      65%        ·      70%
  >>> paste into docs/src/components/arcade/games/<treblecross>.tsx:
  difficulty: {
    easy: { playouts: 30, topK: 5, temp: 2 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  euclid  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      45%      40%      35%      35%      35%      38%
      p30 t2      55%        ·      50%      50%      50%      50%      51%
     p100 t1      60%      50%        ·      50%      50%      50%      52%
   p300 t0.6      65%      50%      50%        ·      50%      50%      53%
  p800 t0.35      65%      50%      50%      50%        ·      50%      53%
    p2000 t0      65%      50%      50%      50%      50%        ·      53%
  >>> paste into docs/src/components/arcade/games/<euclid>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 8, topK: 6, temp: 3 },
    hard: { playouts: 30, topK: 5, temp: 2 },
  },
  (Hard capped at p30: strength peaked early — extra playouts add nothing.)

================  connect-six  (n=20/pair)  ================
     rung\vs       p8      p30      p80     p200     p500    field
       p8 t3        ·      30%       0%       0%       0%       8%
      p30 t2      70%        ·       5%       0%       5%      20%
      p80 t1     100%      95%        ·      32%      28%      64%
   p200 t0.5     100%     100%      68%        ·      50%      79%
   p500 t0.2     100%      95%      72%      50%        ·      79%
  >>> paste into docs/src/components/arcade/games/<connect-six>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 80, topK: 4, temp: 1 },
    hard: { playouts: 200, topK: 3, temp: 0.5 },
  },
  (Hard capped at p200: strength peaked early — extra playouts add nothing.)

================  trap-three  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      10%       5%       5%       0%      10%       6%
      p30 t2      90%        ·      45%      35%      35%      30%      47%
     p100 t1      95%      55%        ·      30%      25%      15%      44%
   p300 t0.6      95%      65%      70%        ·      10%      10%      50%
  p800 t0.35     100%      65%      75%      90%        ·      35%      73%
    p2000 t0      90%      70%      85%      90%      65%        ·      80%
  >>> paste into docs/src/components/arcade/games/<trap-three>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  no-tac-toe  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      10%      20%       0%       0%      10%       8%
      p30 t2      90%        ·      45%       5%      20%      15%      35%
     p100 t1      80%      55%        ·      15%      35%      25%      42%
   p300 t0.6     100%      95%      85%        ·      35%      25%      68%
  p800 t0.35     100%      80%      65%      65%        ·      30%      68%
    p2000 t0      90%      85%      75%      75%      70%        ·      79%
  >>> paste into docs/src/components/arcade/games/<no-tac-toe>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  square-up  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      30%      10%       0%       0%       0%       8%
      p30 t2      70%        ·       0%       0%       0%       0%      14%
     p100 t1      90%     100%        ·      20%       5%       5%      44%
   p300 t0.6     100%     100%      80%        ·      15%      20%      63%
  p800 t0.35     100%     100%      95%      85%        ·      30%      82%
    p2000 t0     100%     100%      95%      80%      70%        ·      89%
  >>> paste into docs/src/components/arcade/games/<square-up>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 2000, topK: 1, temp: 0 },
  },

================  order-chaos  (n=20/pair)  ================
     rung\vs       p8      p30     p100     p300     p800    p2000    field
       p8 t3        ·      35%       0%       0%       0%       0%       7%
      p30 t2      65%        ·       0%       0%       0%       0%      13%
     p100 t1     100%     100%        ·      50%      25%      40%      63%
   p300 t0.6     100%     100%      50%        ·      30%      35%      63%
  p800 t0.35     100%     100%      75%      70%        ·      35%      76%
    p2000 t0     100%     100%      60%      65%      65%        ·      78%
  >>> paste into docs/src/components/arcade/games/<order-chaos>.tsx:
  difficulty: {
    easy: { playouts: 8, topK: 6, temp: 3 },
    medium: { playouts: 100, topK: 4, temp: 1 },
    hard: { playouts: 800, topK: 2, temp: 0.35 },
  },
  (Hard capped at p800: strength peaked early — extra playouts add nothing.)
```
