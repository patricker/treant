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
//!   * Easy   = the weakest rung (a casual human should win a fair share).
//!   * Hard   = the CHEAPEST rung that reaches near-peak strength (so solver
//!              games cap low and don't waste time; open games go high).
//!   * Medium = the rung whose field score sits halfway between Easy and Hard.
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
        g("reversi", STD, || Box::new(ReversiWasm::new(8, 8)) as Box<dyn Eng>),
        g("hex", STD, || Box::new(HexWasm::new(7)) as Box<dyn Eng>),
        g("frontline", STD, || Box::new(FrontlineWasm::new(6, 6)) as Box<dyn Eng>),
        g("mancala", STD, || Box::new(MancalaWasm::new(6, 4, 2)) as Box<dyn Eng>),
        g("dots-and-boxes", STD, || Box::new(DotsBoxesWasm::new(3, 3)) as Box<dyn Eng>),
        g("clobber", STD, || Box::new(ClobberWasm::new(5, 5)) as Box<dyn Eng>),
        g("konane", STD, || Box::new(KonaneWasm::new(6, 6)) as Box<dyn Eng>),
        g("shift", STD, || Box::new(ShiftWasm::new(3, 3, 3, 2, 3)) as Box<dyn Eng>),
        g("pig", STD, || Box::new(PigWasm::new(2, 100)) as Box<dyn Eng>),
        g("chomp", STD, || Box::new(ChompWasm::new(5, 4)) as Box<dyn Eng>),
        g("wythoff", STD, || Box::new(WythoffWasm::new(8)) as Box<dyn Eng>),
        g("trails", STD, || Box::new(TrailsWasm::new(6, 6)) as Box<dyn Eng>),
        g("first-capture", STD, || Box::new(CaptureGoWasm::new(6, 6)) as Box<dyn Eng>),
        g("sim", STD, || Box::new(SimWasm::new()) as Box<dyn Eng>),
        g("mu-torere", STD, || Box::new(MuTorereWasm::new()) as Box<dyn Eng>),
        g("domineering", STD, || Box::new(DomineeringWasm::new(6, 6)) as Box<dyn Eng>),
        g("nogo", STD, || Box::new(NoGoWasm::new(5, 5)) as Box<dyn Eng>),
        g("col", STD, || Box::new(ColWasm::new(5, 5)) as Box<dyn Eng>),
        g("amazons", LIGHT, || Box::new(AmazonsWasm::new(6, 6)) as Box<dyn Eng>),
        g("fox-hounds", STD, || Box::new(FoxHoundsWasm::new(8, 8)) as Box<dyn Eng>),
        g("treblecross", STD, || Box::new(TreblecrossWasm::new(11)) as Box<dyn Eng>),
        g("euclid", STD, || Box::new(EuclidWasm::new(25, 16)) as Box<dyn Eng>),
        g("connect-six", LIGHT, || Box::new(Connect6Wasm::new(9, 9)) as Box<dyn Eng>),
        g("trap-three", STD, || Box::new(SquavaWasm::new(5, 5)) as Box<dyn Eng>),
        g("no-tac-toe", STD, || Box::new(NotaktoWasm::new(3, 3)) as Box<dyn Eng>),
        g("square-up", STD, || Box::new(SquareUpWasm::new(6, 6)) as Box<dyn Eng>),
        g("order-chaos", STD, || Box::new(OrderChaosWasm::new(6, 6)) as Box<dyn Eng>),
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
    match g.result().as_str() {
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
    for i in 0..m {
        for j in (i + 1)..m {
            for gi in 0..n {
                let (x, y) = if gi % 2 == 0 { (i, j) } else { (j, i) }; // alternate first move
                let (sx, sy) = match play(game.make, &pool[x], &pool[y], &mut rng) {
                    0 => (1.0, 0.0),
                    1 => (0.0, 1.0),
                    _ => (0.5, 0.5),
                };
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

fn main() {
    let filter = std::env::args().nth(1);
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
