//! Self-play AI difficulty calibration for the Treant Arcade.
//!
//! WHY: MCTS strength is ~logarithmic in the playout count, but *where* the curve
//! is steep vs. flat depends on the game — tiny/solved games (Tic-Tac-Toe) hit
//! peak strength in a few dozen playouts, while open games (Hex, Reversi) keep
//! climbing for thousands. Published numbers come from neural-net engines and do
//! NOT transfer to our pure-rollout engines, so we MEASURE each game here.
//!
//! WHAT: For every game we round-robin a ladder of `weak_move` configs
//! ({playouts, top_k, temp}) against each other, then auto-pick Easy/Medium/Hard:
//!
//! - Easy = the weakest rung (a casual human should win a fair share).
//! - Hard = the CHEAPEST rung that reaches near-peak strength (so solver games
//!   cap low and don't waste time; open games go high).
//! - Medium = the rung whose field score sits halfway between Easy and Hard.
//!
//! It prints the win matrix + a ready-to-paste `difficulty:` block per game.
//!
//! HOW TO RUN:
//!   cargo run --release --example calibrate -p treant-wasm            # all games
//!   cargo run --release --example calibrate -p treant-wasm connect-four   # one game
//! (or use scripts/calibrate.sh). Numbers feed each game's `difficulty` override
//! in docs/src/components/arcade/games/<game>.tsx. Re-run any time to retune.
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use treant_wasm::*;

/// One AI strength setting — exactly the production `pickAiMove` / `weak_move`.
#[derive(Clone, Copy)]
struct Cfg {
    playouts: u32,
    top_k: usize,
    temp: f64,
}

/// Object-safe view of a game engine for the harness.
trait Eng {
    fn weak(&mut self, p: u32, k: usize, t: f64, seed: u32) -> Option<String>;
    fn apply(&mut self, m: &str) -> bool;
    fn terminal(&self) -> bool;
    fn result(&self) -> String;
    fn current(&self) -> u32;
}
macro_rules! eng {
    ($($t:ty),+ $(,)?) => { $(
        impl Eng for $t {
            fn weak(&mut self, p: u32, k: usize, t: f64, seed: u32) -> Option<String> { self.weak_move(p, k, t, seed) }
            fn apply(&mut self, m: &str) -> bool { self.apply_move(m) }
            fn terminal(&self) -> bool { self.is_terminal() }
            fn result(&self) -> String { self.result() }
            fn current(&self) -> u32 { self.current_player() }
        }
    )+ };
}
eng!(
    ConnectFourWasm, TicTacToeWasm, HexWasm, ReversiWasm, FrontlineWasm, MancalaWasm, DotsBoxesWasm,
    ClobberWasm, KonaneWasm, ShiftWasm, PigWasm, ChompWasm, WythoffWasm, TrailsWasm, CaptureGoWasm,
    SimWasm, MuTorereWasm, DomineeringWasm, NoGoWasm, ColWasm, AmazonsWasm, FoxHoundsWasm,
    TreblecrossWasm, EuclidWasm, Connect6Wasm, SquavaWasm, NotaktoWasm, SquareUpWasm, OrderChaosWasm,
    PinchFiveWasm, NineMorrisWasm, QuadlineWasm, OwareWasm, YGameWasm, BaghchalWasm, ClimbWasm,
);

// Ladders, weak -> strong. STD for most games; LIGHT caps playouts for the
// branchy/heavy games (Amazons, Connect Six, Gomoku) so a run stays minutes.
const STD: &[Cfg] = &[
    Cfg { playouts: 8, top_k: 6, temp: 3.0 },
    Cfg { playouts: 30, top_k: 5, temp: 2.0 },
    Cfg { playouts: 100, top_k: 4, temp: 1.0 },
    Cfg { playouts: 300, top_k: 3, temp: 0.6 },
    Cfg { playouts: 800, top_k: 2, temp: 0.35 },
    Cfg { playouts: 2000, top_k: 1, temp: 0.0 },
];
const LIGHT: &[Cfg] = &[
    Cfg { playouts: 8, top_k: 6, temp: 3.0 },
    Cfg { playouts: 30, top_k: 5, temp: 2.0 },
    Cfg { playouts: 80, top_k: 4, temp: 1.0 },
    Cfg { playouts: 200, top_k: 3, temp: 0.5 },
    Cfg { playouts: 500, top_k: 2, temp: 0.2 },
];

struct Game {
    name: &'static str, // arcade id (also the CLI filter token)
    ladder: &'static [Cfg],
    make: fn() -> Box<dyn Eng>,
}

fn games() -> Vec<Game> {
    fn g(name: &'static str, ladder: &'static [Cfg], make: fn() -> Box<dyn Eng>) -> Game {
        Game { name, ladder, make }
    }
    vec![
        g("connect-four", STD, || Box::new(ConnectFourWasm::new(7, 6, 4, 2)) as Box<dyn Eng>),
        g("tic-tac-toe", STD, || Box::new(TicTacToeWasm::new(3, 3, 3, 2)) as Box<dyn Eng>),
        g("gomoku", LIGHT, || Box::new(TicTacToeWasm::new(13, 13, 5, 2)) as Box<dyn Eng>),
        g("reversi", STD, || Box::new(ReversiWasm::new(8, 8, 0)) as Box<dyn Eng>),
        g("anti-reversi", STD, || Box::new(ReversiWasm::new(8, 8, 1)) as Box<dyn Eng>),
        g("hex", STD, || Box::new(HexWasm::new(7, 1)) as Box<dyn Eng>),
        g("frontline", STD, || Box::new(FrontlineWasm::new(6, 6)) as Box<dyn Eng>),
        g("mancala", STD, || Box::new(MancalaWasm::new(6, 4, 2)) as Box<dyn Eng>),
        g("dots-and-boxes", STD, || Box::new(DotsBoxesWasm::new(3, 3)) as Box<dyn Eng>),
        g("clobber", STD, || Box::new(ClobberWasm::new(5, 5)) as Box<dyn Eng>),
        g("konane", STD, || Box::new(KonaneWasm::new(6, 6)) as Box<dyn Eng>),
        g("shift", STD, || Box::new(ShiftWasm::new(3, 3, 3, 2, 3)) as Box<dyn Eng>),
        g("pig", STD, || Box::new(PigWasm::new(2, 100)) as Box<dyn Eng>),
        g("chomp", STD, || Box::new(ChompWasm::new(5, 4)) as Box<dyn Eng>),
        g("wythoff", STD, || Box::new(WythoffWasm::new(8)) as Box<dyn Eng>),
        g("trails", STD, || Box::new(TrailsWasm::new(6, 6, 0)) as Box<dyn Eng>),
        g("joust", STD, || Box::new(TrailsWasm::new(6, 6, 1)) as Box<dyn Eng>),
        g("first-capture", STD, || Box::new(CaptureGoWasm::new(6, 6)) as Box<dyn Eng>),
        g("sim", STD, || Box::new(SimWasm::new()) as Box<dyn Eng>),
        g("mu-torere", STD, || Box::new(MuTorereWasm::new()) as Box<dyn Eng>),
        g("domineering", STD, || Box::new(DomineeringWasm::new(6, 6, 0)) as Box<dyn Eng>),
        g("cram", STD, || Box::new(DomineeringWasm::new(6, 6, 1)) as Box<dyn Eng>),
        g("nogo", STD, || Box::new(NoGoWasm::new(5, 5)) as Box<dyn Eng>),
        g("col", STD, || Box::new(ColWasm::new(5, 5, 0)) as Box<dyn Eng>),
        g("snort", STD, || Box::new(ColWasm::new(5, 5, 1)) as Box<dyn Eng>),
        g("amazons", LIGHT, || Box::new(AmazonsWasm::new(6, 6)) as Box<dyn Eng>),
        g("fox-hounds", STD, || Box::new(FoxHoundsWasm::new(8, 8)) as Box<dyn Eng>),
        g("treblecross", STD, || Box::new(TreblecrossWasm::new(11)) as Box<dyn Eng>),
        g("euclid", STD, || Box::new(EuclidWasm::new(25, 16)) as Box<dyn Eng>),
        g("connect-six", LIGHT, || Box::new(Connect6Wasm::new(9, 9)) as Box<dyn Eng>),
        g("trap-three", STD, || Box::new(SquavaWasm::new(5, 5)) as Box<dyn Eng>),
        g("no-tac-toe", STD, || Box::new(NotaktoWasm::new(3, 3)) as Box<dyn Eng>),
        g("square-up", STD, || Box::new(SquareUpWasm::new(6, 6)) as Box<dyn Eng>),
        g("order-chaos", STD, || Box::new(OrderChaosWasm::new(6, 6)) as Box<dyn Eng>),
        // 7 new games (2026-07). Classic-preset params; LIGHT for the branchy 13×13 board.
        g("pinch-five", LIGHT, || Box::new(PinchFiveWasm::new(13, 13, 5, 2)) as Box<dyn Eng>),
        g("nine-morris", STD, || Box::new(NineMorrisWasm::new(9, 0, 0)) as Box<dyn Eng>),
        g("lasker-morris", STD, || Box::new(NineMorrisWasm::new(10, 0, 1)) as Box<dyn Eng>),
        g("quadline", STD, || Box::new(QuadlineWasm::new(5, 1)) as Box<dyn Eng>),
        g("oware", STD, || Box::new(OwareWasm::new(6, 4)) as Box<dyn Eng>),
        g("y", STD, || Box::new(YGameWasm::new(8)) as Box<dyn Eng>),
        // bagh-chal is asymmetric: seat 0 = goats (move first), seat 1 = tigers.
        g("bagh-chal", STD, || Box::new(BaghchalWasm::new(20, 5)) as Box<dyn Eng>),
        // climb is a chance (push-your-luck dice) game — calibrated like pig.
        g("climb", STD, || Box::new(ClimbWasm::new(2, 3)) as Box<dyn Eng>),
    ]
}

/// Play one game; seat 0 uses `a`, seat 1 uses `b`. Returns 0 (seat0 win),
/// 1 (seat1 win), or 2 (draw / unfinished after the ply cap).
fn play(make: fn() -> Box<dyn Eng>, a: &Cfg, b: &Cfg, rng: &mut SmallRng) -> u32 {
    let mut g = make();
    for _ in 0..3000 {
        if g.terminal() {
            break;
        }
        let c = if g.current() == 0 { a } else { b };
        let seed = rng.gen::<u32>();
        match g.weak(c.playouts, c.top_k, c.temp, seed) {
            Some(m) => {
                let _ = g.apply(&m);
            }
            None => break,
        }
    }
    // Most engines report the winner as "1"/"2"; Mancala uses "P1"/"P2". Strip
    // any leading 'P' so both conventions land in the same buckets.
    match g.result().trim_start_matches('P') {
        "1" => 0,
        "2" => 1,
        _ => 2,
    }
}

fn run(game: &Game, n: u32) {
    let pool = game.ladder;
    let m = pool.len();
    let mut rng = SmallRng::seed_from_u64(0xC0FFEE);
    let mut pts = vec![vec![0.0f64; m]; m];
    let mut field = vec![0.0f64; m];
    let mut played = vec![0.0f64; m];
    // Seat-0 wins vs decided games — ~50% for symmetric games, reveals a
    // structural seat advantage for asymmetric ones (bagh-chal: seat 0 = goats).
    let mut seat0_wins = 0.0f64;
    let mut decided = 0.0f64;
    for i in 0..m {
        for j in (i + 1)..m {
            for gi in 0..n {
                let (x, y) = if gi % 2 == 0 { (i, j) } else { (j, i) }; // alternate first move
                let outcome = play(game.make, &pool[x], &pool[y], &mut rng);
                let (sx, sy) = match outcome {
                    0 => (1.0, 0.0),
                    1 => (0.0, 1.0),
                    _ => (0.5, 0.5),
                };
                if outcome != 2 {
                    decided += 1.0;
                    if outcome == 0 {
                        seat0_wins += 1.0;
                    }
                }
                pts[x][y] += sx;
                pts[y][x] += sy;
                field[x] += sx;
                field[y] += sy;
                played[x] += 1.0;
                played[y] += 1.0;
            }
        }
    }
    let fscore: Vec<f64> = (0..m).map(|i| field[i] / played[i]).collect();

    println!("\n================  {}  (n={n}/pair)  ================", game.name);
    print!("{:>12}", "rung\\vs");
    for c in pool {
        print!("{:>9}", format!("p{}", c.playouts));
    }
    println!("{:>9}", "field");
    for i in 0..m {
        print!("{:>12}", format!("p{} t{}", pool[i].playouts, pool[i].temp));
        #[allow(clippy::needless_range_loop)]
        for j in 0..m {
            if i == j {
                print!("{:>9}", "·");
            } else {
                print!("{:>8.0}%", 100.0 * pts[i][j] / n as f64);
            }
        }
        println!("{:>8.0}%", 100.0 * fscore[i]);
    }
    if decided > 0.0 {
        println!("  seat-0 win rate (decided games): {:.0}%  [~50% = balanced]", 100.0 * seat0_wins / decided);
    }

    // --- auto-select Easy / Medium / Hard from the field scores ---
    let peak = fscore.iter().cloned().fold(0.0f64, f64::max);
    // Hard = cheapest rung within 5% of peak strength (rungs are playout-ascending).
    let hard = (0..m).find(|&i| fscore[i] >= peak - 0.05).unwrap_or(m - 1);
    // Easy = weakest rung.
    let easy = (0..m).min_by(|&a, &b| fscore[a].partial_cmp(&fscore[b]).unwrap()).unwrap_or(0);
    // Medium = the rung between them whose field score is closest to the midpoint.
    let mid_target = 0.5 * (fscore[easy] + fscore[hard]);
    let lo = easy.min(hard);
    let hi = easy.max(hard);
    let medium = if hi > lo + 1 {
        (lo + 1..hi)
            .min_by(|&a, &b| {
                (fscore[a] - mid_target).abs().partial_cmp(&(fscore[b] - mid_target).abs()).unwrap()
            })
            .unwrap_or(lo)
    } else {
        lo
    };

    let fmt = |i: usize| {
        let c = pool[i];
        format!("{{ playouts: {}, topK: {}, temp: {} }}", c.playouts, c.top_k, c.temp)
    };
    println!("  >>> paste into docs/src/components/arcade/games/<{}>.tsx:", game.name);
    println!("  difficulty: {{");
    println!("    easy: {},", fmt(easy));
    println!("    medium: {},", fmt(medium));
    println!("    hard: {},", fmt(hard));
    println!("  }},");
    if hard < m - 1 {
        println!(
            "  (Hard capped at p{}: strength peaked early — extra playouts add nothing.)",
            pool[hard].playouts
        );
    }
}

// ============================================================ Audit mode =====
// `cargo run --release --example calibrate -- audit`
//
// Autonomous bug-sweep. Two passes, no human in the loop:
//   1. Self-play invariant fuzzer over every registered game: play semi-random
//      legal moves (engine-supplied) to a terminal, asserting the game actually
//      terminates, the engine never offers a move its own apply_move rejects, the
//      result string is well-formed, the player index stays in range, and nothing
//      panics. Catches broken win/terminal detection, illegal-move generation,
//      and crashes.
//   2. Grid placement/clamp check: for the free-placement grid games, build at
//      the *largest board the UI offers* and assert (a) the engine board has
//      cols*rows cells (no silent clamp vs the React layer — the Gomoku bug) and
//      (b) every cell is placeable on a fresh board (no apply_move rejection of
//      un-searched cells).

fn audit_one(make: fn() -> Box<dyn Eng>, trial: u32) -> Option<String> {
    let mut g = make();
    let mut rng = SmallRng::seed_from_u64(0xA0D17 + trial as u64);
    let cap = 4000u32;
    let mut plies = 0u32;
    while !g.terminal() {
        if plies >= cap {
            return Some(format!("did not terminate within {cap} plies (win/terminal detection?)"));
        }
        let cur = g.current();
        if cur >= 8 {
            return Some(format!("current_player out of range: {cur}"));
        }
        let seed = rng.gen::<u32>();
        match g.weak(60, 12, 4.0, seed) {
            Some(m) => {
                if !g.apply(&m) {
                    return Some(format!("engine offered move '{m}' but apply_move rejected it"));
                }
            }
            None => return Some("weak_move returned None on a non-terminal position".into()),
        }
        plies += 1;
    }
    let r = g.result();
    if r.is_empty() {
        return Some("result() is empty at a terminal position".into());
    }
    let body = r.strip_prefix('P').unwrap_or(&r);
    if r != "Draw" && !body.chars().all(|c| c.is_ascii_digit()) {
        return Some(format!("result() format unexpected: '{r}'"));
    }
    None
}

/// First-move placeability + clamp check for one free-placement grid board.
fn grid_check(name: &str, cols: usize, rows: usize, make: &dyn Fn() -> (String, Vec<bool>)) {
    let (board, placeable) = make();
    let cells = board.chars().count();
    if cells != cols * rows {
        println!("  FLAG  {name}: board has {cells} cells, expected {cols}×{rows}={} (silent clamp?)", cols * rows);
    }
    let bad: Vec<usize> = placeable.iter().enumerate().filter(|(_, &ok)| !ok).map(|(i, _)| i).collect();
    if !bad.is_empty() {
        let show: Vec<usize> = bad.iter().take(6).copied().collect();
        println!("  FLAG  {name}: {}/{} cells unplaceable on a fresh board, e.g. {show:?}", bad.len(), cols * rows);
    }
    if cells == cols * rows && bad.is_empty() {
        println!("  ok    {name} ({cols}×{rows})");
    }
}

fn audit() {
    println!("=== Pass 1: self-play invariant fuzzer (all games) ===");
    let mut flags = 0;
    for game in games() {
        let mut issues: Vec<String> = Vec::new();
        for trial in 0..6u32 {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| audit_one(game.make, trial))) {
                Ok(Some(issue)) => issues.push(issue),
                Ok(None) => {}
                Err(_) => issues.push(format!("PANIC during self-play (trial {trial})")),
            }
        }
        issues.sort();
        issues.dedup();
        if issues.is_empty() {
            println!("  ok    {}", game.name);
        } else {
            for i in &issues {
                flags += 1;
                println!("  FLAG  {}: {i}", game.name);
            }
        }
    }

    println!("\n=== Pass 2: grid placement / clamp check (largest UI board) ===");
    // (name, cols, rows, builder) — dims are each game's biggest UI preset/knob.
    // Build at those dims, return (board string, per-cell first-move placeability).
    fn ttt(cols: usize, rows: usize, k: u32) -> (String, Vec<bool>) {
        let g = TicTacToeWasm::new(cols as u32, rows as u32, k, 2);
        let board = g.get_board();
        let placeable = (0..cols * rows)
            .map(|i| TicTacToeWasm::new(cols as u32, rows as u32, k, 2).apply_move(&i.to_string()))
            .collect();
        (board, placeable)
    }
    fn c6(cols: usize, rows: usize) -> (String, Vec<bool>) {
        let g = Connect6Wasm::new(cols as u32, rows as u32);
        let board = g.get_board();
        let placeable = (0..cols * rows)
            .map(|i| Connect6Wasm::new(cols as u32, rows as u32).apply_move(&i.to_string()))
            .collect();
        (board, placeable)
    }
    fn cap(cols: usize, rows: usize) -> (String, Vec<bool>) {
        let g = CaptureGoWasm::new(cols as u32, rows as u32);
        let board = g.get_board();
        let placeable = (0..cols * rows)
            .map(|i| CaptureGoWasm::new(cols as u32, rows as u32).apply_move(&i.to_string()))
            .collect();
        (board, placeable)
    }
    grid_check("tic-tac-toe", 10, 10, &|| ttt(10, 10, 4));
    grid_check("gomoku", 15, 15, &|| ttt(15, 15, 5));
    grid_check("connect-six", 12, 12, &|| c6(12, 12));
    grid_check("first-capture", 9, 9, &|| cap(9, 9));

    println!("\n{flags} self-play flag(s). Review any FLAG lines above.");
}

fn main() {
    let mut args = std::env::args();
    let first = args.nth(1);
    if first.as_deref() == Some("audit") {
        audit();
        return;
    }
    let filter = first;
    let n: u32 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(20);
    for game in games() {
        if let Some(f) = &filter {
            if game.name != f {
                continue;
            }
        }
        run(&game, n);
    }
}
