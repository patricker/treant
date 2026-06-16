//! Self-play difficulty calibration. Round-robins a ladder of AiConfigs per game
//! using the SHIPPING weak_move, prints a win matrix + each config's field score,
//! and flags adjacent ladder steps where strength really changes.
//! Run: cargo run --release --example calibrate -p treant-wasm
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use treant_wasm::*;

#[derive(Clone, Copy)]
struct Cfg { name: &'static str, playouts: u32, top_k: usize, temp: f64 }

trait Eng {
    fn weak(&mut self, p: u32, k: usize, t: f64, seed: u32) -> Option<String>;
    fn apply(&mut self, m: &str) -> bool;
    fn terminal(&self) -> bool;
    fn result(&self) -> String;
    fn current(&self) -> u32;
}
macro_rules! eng {
    ($t:ty) => {
        impl Eng for $t {
            fn weak(&mut self, p: u32, k: usize, t: f64, seed: u32) -> Option<String> { self.weak_move(p, k, t, seed) }
            fn apply(&mut self, m: &str) -> bool { self.apply_move(m) }
            fn terminal(&self) -> bool { self.is_terminal() }
            fn result(&self) -> String { self.result() }
            fn current(&self) -> u32 { self.current_player() }
        }
    };
}
eng!(ConnectFourWasm); eng!(TicTacToeWasm); eng!(ReversiWasm); eng!(HexWasm); eng!(FrontlineWasm);

fn play(make: &dyn Fn() -> Box<dyn Eng>, a: &Cfg, b: &Cfg, rng: &mut SmallRng) -> u32 {
    let mut g = make();
    for _ in 0..2000 {
        if g.terminal() { break; }
        let c = if g.current() == 0 { a } else { b };
        let seed = rng.gen::<u32>();
        match g.weak(c.playouts, c.top_k, c.temp, seed) {
            Some(m) => { let _ = g.apply(&m); }
            None => break,
        }
    }
    match g.result().as_str() { "1" => 0, "2" => 1, _ => 2 }
}

fn tournament(name: &str, make: &dyn Fn() -> Box<dyn Eng>, pool: &[Cfg], n: u32) {
    let mut rng = SmallRng::seed_from_u64(0xC0FFEE);
    let m = pool.len();
    let mut pts = vec![vec![0.0f64; m]; m];
    let mut field = vec![0.0f64; m];
    let mut games = vec![0.0f64; m];
    for i in 0..m {
        for j in (i + 1)..m {
            for g in 0..n {
                let (x, y) = if g % 2 == 0 { (i, j) } else { (j, i) };
                let (sx, sy) = match play(make, &pool[x], &pool[y], &mut rng) {
                    0 => (1.0, 0.0), 1 => (0.0, 1.0), _ => (0.5, 0.5),
                };
                pts[x][y] += sx; pts[y][x] += sy;
                field[x] += sx; field[y] += sy; games[x] += 1.0; games[y] += 1.0;
            }
        }
    }
    println!("\n==== {name} ====");
    #[allow(clippy::needless_range_loop)]
    for i in 0..m {
        print!("{:>12} ", pool[i].name);
        for j in 0..m {
            if i == j { print!("    ·"); }
            else { print!(" {:>3.0}%", 100.0 * pts[i][j] / n as f64); }
        }
        println!("   field {:>3.0}%", 100.0 * field[i] / games[i]);
    }
    println!("  ladder steps (upper beats lower):");
    for i in 1..m {
        let wr = 100.0 * pts[i][i - 1] / n as f64;
        let tag = if wr >= 70.0 { "  <-- real step" } else if wr <= 58.0 { "  (no real change)" } else { "" };
        println!("    {:>12} vs {:<12} {:>3.0}%{}", pool[i].name, pool[i - 1].name, wr, tag);
    }
}

fn main() {
    let ladder = [
        Cfg { name: "p8 t3 k6",   playouts: 8,    top_k: 6, temp: 3.0 },
        Cfg { name: "p30 t2 k5",  playouts: 30,   top_k: 5, temp: 2.0 },
        Cfg { name: "p100 t1 k4", playouts: 100,  top_k: 4, temp: 1.0 },
        Cfg { name: "p300 t.6 k3",playouts: 300,  top_k: 3, temp: 0.6 },
        Cfg { name: "p1000 t.3",  playouts: 1000, top_k: 2, temp: 0.3 },
        Cfg { name: "p4000 t0",   playouts: 4000, top_k: 1, temp: 0.0 },
    ];
    let n = 40;
    tournament("Tic-Tac-Toe 3x3", &|| Box::new(TicTacToeWasm::new(3, 3, 3, 2)), &ladder, n);
    tournament("Connect Four 7x6", &|| Box::new(ConnectFourWasm::new(7, 6, 4, 2)), &ladder, n);
    tournament("Gomoku 9x9 k5", &|| Box::new(TicTacToeWasm::new(9, 9, 5, 2)), &ladder, n);
    tournament("Reversi 6x6", &|| Box::new(ReversiWasm::new(6, 6)), &ladder, n);
    tournament("Hex 7x7", &|| Box::new(HexWasm::new(7)), &ladder, n);
    tournament("Frontline 6x6", &|| Box::new(FrontlineWasm::new(6, 6)), &ladder, n);
}
