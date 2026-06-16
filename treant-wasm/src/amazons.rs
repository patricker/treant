//! Amazons — the move-and-shoot territory game. On your turn you move one of
//! your amazons any number of empty squares in a straight line (like a chess
//! queen), then from its new square shoot an arrow the same way; the arrow
//! burns its landing square, which is blocked for the rest of the game. The
//! player who cannot move loses. Squares only ever get burnt, so the game
//! always terminates and there are no draws.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

const ARROW: i8 = 2;
const DIRS: [(isize, isize); 8] =
    [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AzMove {
    from: u16,
    to: u16,
    arrow: u16,
}
impl std::fmt::Display for AzMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}-{}", self.from, self.to, self.arrow)
    }
}

#[derive(Clone)]
struct Amazons {
    cols: usize,
    rows: usize,
    grid: Vec<i8>, // -1 empty, 0/1 amazon, 2 arrow (burnt)
    current: u8,
}
impl Amazons {
    fn new(cols: usize, rows: usize) -> Self {
        let mut grid = vec![-1i8; cols * rows];
        // Two amazons per side, symmetric, on the top and bottom rows.
        let c1 = (cols - 1) / 3;
        let c2 = cols - 1 - c1;
        grid[c1] = 0;
        grid[c2] = 0;
        grid[(rows - 1) * cols + c1] = 1;
        grid[(rows - 1) * cols + c2] = 1;
        Self { cols, rows, grid, current: 0 }
    }
    /// Walk from (r,c) in direction (dr,dc), calling `f` for each empty square
    /// reached until blocked. `ignore` is treated as empty (the amazon's own
    /// origin square when shooting the arrow).
    fn ray<F: FnMut(usize)>(&self, r: usize, c: usize, dr: isize, dc: isize, ignore: isize, mut f: F) {
        let mut nr = r as isize + dr;
        let mut nc = c as isize + dc;
        while nr >= 0 && nr < self.rows as isize && nc >= 0 && nc < self.cols as isize {
            let idx = nr as usize * self.cols + nc as usize;
            if self.grid[idx] != -1 && idx as isize != ignore {
                break;
            }
            f(idx);
            nr += dr;
            nc += dc;
        }
    }
    fn gen(&self) -> Vec<AzMove> {
        let mut v = Vec::new();
        let me = self.current as i8;
        for from in 0..self.grid.len() {
            if self.grid[from] != me {
                continue;
            }
            let fr = from / self.cols;
            let fc = from % self.cols;
            for (dr, dc) in DIRS {
                self.ray(fr, fc, dr, dc, -1, |to| {
                    let tr = to / self.cols;
                    let tc = to % self.cols;
                    for (ar, ac) in DIRS {
                        self.ray(tr, tc, ar, ac, from as isize, |arrow| {
                            v.push(AzMove { from: from as u16, to: to as u16, arrow: arrow as u16 });
                        });
                    }
                });
            }
        }
        v
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.gen().is_empty() {
            Some(ProvenValue::Loss) // current can't move -> loses
        } else {
            None
        }
    }
}
impl GameState for Amazons {
    type Move = AzMove;
    type Player = u8;
    type MoveList = Vec<AzMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<AzMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &AzMove) {
        self.grid[m.from as usize] = -1;
        self.grid[m.to as usize] = self.current as i8;
        self.grid[m.arrow as usize] = ARROW;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct AzEval;
impl Evaluator<AzCfg> for AzEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Amazons, m: &Vec<AzMove>, _: Option<SearchHandle<AzCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Amazons, _: &i64, _: SearchHandle<AzCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct AzCfg;
impl MCTS for AzCfg {
    type State = Amazons;
    type Eval = AzEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct AmazonsWasm {
    manager: MCTSManager<AzCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl AmazonsWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            manager: MCTSManager::new(Amazons::new(cols, rows), AzCfg, AzEval, UCTPolicy::new(1.4), ()),
            cols,
            rows,
        }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap()
    }
    /// One char per cell, row-major: ' '=empty, 'X'=player 0, 'O'=player 1,
    /// '#'=burnt (arrow).
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .grid
            .iter()
            .map(|&v| match v {
                0 => 'X',
                1 => 'O',
                2 => '#',
                _ => ' ',
            })
            .collect()
    }
    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current as u32
    }
    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().term().is_some()
    }
    pub fn result(&self) -> String {
        let s = self.manager.tree().root_state();
        match s.term() {
            Some(ProvenValue::Loss) => format!("{}", (1 - s.current) + 1),
            _ => String::new(),
        }
    }
    pub fn legal_moves(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .available_moves()
            .iter()
            .map(|m| format!("{m}"))
            .collect::<Vec<_>>()
            .join(",")
    }
    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let parts: Vec<&str> = mov.split('-').collect();
        if parts.len() != 3 {
            return false;
        }
        let (from, to, arrow): (u16, u16, u16) = match (parts[0].parse(), parts[1].parse(), parts[2].parse()) {
            (Ok(a), Ok(b), Ok(c)) => (a, b, c),
            _ => return false,
        };
        let m = AzMove { from, to, arrow };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, AzCfg, AzEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Amazons::new(self.cols, self.rows), AzCfg, AzEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_setup_has_two_amazons_each() {
        let g = Amazons::new(6, 6);
        assert_eq!(g.grid.iter().filter(|&&v| v == 0).count(), 2);
        assert_eq!(g.grid.iter().filter(|&&v| v == 1).count(), 2);
        assert!(!g.gen().is_empty());
    }

    #[test]
    fn move_relocates_amazon_and_burns_arrow_square() {
        let mut g = Amazons::new(6, 6);
        // pick a move whose arrow lands somewhere other than the vacated origin,
        // so we can cleanly check that the origin square is now empty.
        let m = *g.gen().iter().find(|m| m.arrow != m.from).unwrap();
        g.make_move(&m);
        assert_eq!(g.grid[m.from as usize], -1);
        assert_eq!(g.grid[m.to as usize], 0);
        assert_eq!(g.grid[m.arrow as usize], ARROW);
        assert_eq!(g.current, 1);
    }

    #[test]
    fn arrow_may_land_on_the_amazons_old_square() {
        // The origin square is vacated before shooting, so shooting back onto it
        // must be legal. Find such a move to confirm the `ignore` handling.
        let g = Amazons::new(6, 6);
        let any_back = g.gen().iter().any(|m| m.arrow == m.from);
        assert!(any_back);
    }

    #[test]
    fn ai_plays() {
        let mut g = AmazonsWasm::new(6, 6);
        g.playout_n(400);
        assert!(g.best_move().is_some());
    }
}
