//! Reversi (public-domain; not "Othello"). Place a disc bracketing a line of
//! enemy discs to flip them. Pass if you have no flipping move; the game ends
//! when neither can move. Most discs wins (draws possible).
use crate::gridlib::idx;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

const DIRS: [(i32, i32); 8] = [(0, 1), (0, -1), (1, 0), (-1, 0), (1, 1), (1, -1), (-1, 1), (-1, -1)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RvMove {
    Place(u16),
    Pass,
}
impl std::fmt::Display for RvMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RvMove::Place(i) => write!(f, "{i}"),
            RvMove::Pass => write!(f, "pass"),
        }
    }
}

#[derive(Clone)]
struct Reversi {
    board: Vec<i8>,
    cols: usize,
    rows: usize,
    current: u8,
    /// Anti-Reversi (misère): fewest discs wins at game end.
    anti: bool,
}
impl Reversi {
    fn new(cols: usize, rows: usize, anti: bool) -> Self {
        let mut board = vec![-1i8; cols * rows];
        let (mr, mc) = (rows / 2, cols / 2);
        board[idx(mr - 1, mc - 1, cols)] = 0;
        board[idx(mr, mc, cols)] = 0;
        board[idx(mr - 1, mc, cols)] = 1;
        board[idx(mr, mc - 1, cols)] = 1;
        Self { board, cols, rows, current: 0, anti }
    }
    fn flips(&self, cell: usize, player: i8) -> Vec<usize> {
        if self.board[cell] != -1 {
            return vec![];
        }
        let enemy = 1 - player;
        let (r, c) = (cell / self.cols, cell % self.cols);
        let mut all = Vec::new();
        for (dr, dc) in DIRS {
            let mut line = Vec::new();
            let (mut rr, mut cc) = (r as i32 + dr, c as i32 + dc);
            while rr >= 0 && rr < self.rows as i32 && cc >= 0 && cc < self.cols as i32 {
                let i = idx(rr as usize, cc as usize, self.cols);
                if self.board[i] == enemy {
                    line.push(i);
                } else {
                    if self.board[i] == player && !line.is_empty() {
                        all.extend(&line);
                    }
                    break;
                }
                rr += dr;
                cc += dc;
            }
        }
        all
    }
    fn has_move(&self, player: i8) -> bool {
        (0..self.board.len()).any(|i| self.board[i] == -1 && !self.flips(i, player).is_empty())
    }
    fn count(&self, player: i8) -> i64 {
        self.board.iter().filter(|&&v| v == player).count() as i64
    }
    fn corners(&self, player: i8) -> i64 {
        let last = self.board.len() - 1;
        let tr = self.cols - 1;
        let bl = (self.rows - 1) * self.cols;
        [0, tr, bl, last].iter().filter(|&&i| self.board[i] == player).count() as i64
    }
    fn mobility(&self, player: i8) -> i64 {
        (0..self.board.len()).filter(|&i| self.board[i] == -1 && !self.flips(i, player).is_empty()).count() as i64
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.has_move(self.current as i8) || self.has_move(1 - self.current as i8) {
            return None;
        }
        // neither can move -> game over
        let (me, you) = (self.count(self.current as i8), self.count(1 - self.current as i8));
        Some(match me.cmp(&you) {
            // Anti-Reversi (misère): fewest discs wins, so the comparison flips.
            std::cmp::Ordering::Greater => if self.anti { ProvenValue::Loss } else { ProvenValue::Win },
            std::cmp::Ordering::Less => if self.anti { ProvenValue::Win } else { ProvenValue::Loss },
            std::cmp::Ordering::Equal => ProvenValue::Draw,
        })
    }
}
impl GameState for Reversi {
    type Move = RvMove;
    type Player = u8;
    type MoveList = Vec<RvMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<RvMove> {
        let mine: Vec<RvMove> = (0..self.board.len())
            .filter(|&i| self.board[i] == -1 && !self.flips(i, self.current as i8).is_empty())
            .map(|i| RvMove::Place(i as u16))
            .collect();
        if !mine.is_empty() {
            return mine;
        }
        if self.has_move(1 - self.current as i8) {
            return vec![RvMove::Pass]; // must pass
        }
        vec![] // terminal
    }
    fn make_move(&mut self, m: &RvMove) {
        match m {
            RvMove::Place(i) => {
                let cell = *i as usize;
                let flips = self.flips(cell, self.current as i8);
                self.board[cell] = self.current as i8;
                for f in flips {
                    self.board[f] = self.current as i8;
                }
            }
            RvMove::Pass => {}
        }
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct RvEval;
impl Evaluator<RvCfg> for RvEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &Reversi, m: &Vec<RvMove>, _: Option<SearchHandle<RvCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], s.eval0())
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &Reversi, _: &i64, _: SearchHandle<RvCfg>) -> i64 {
        s.eval0()
    }
}
impl Reversi {
    fn eval0(&self) -> i64 {
        // Material + corner control are good in Reversi but BAD in Anti-Reversi
        // (fewest discs wins), so sign-flip them under `anti`. Mobility keeps its
        // sign either way — having moves available is always an advantage.
        let material = (self.count(0) - self.count(1)) + 12 * (self.corners(0) - self.corners(1));
        let signed = if self.anti { -material } else { material };
        signed + 2 * (self.mobility(0) - self.mobility(1))
    }
}
#[derive(Default)]
struct RvCfg;
impl MCTS for RvCfg {
    type State = Reversi;
    type Eval = RvEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
}

#[wasm_bindgen]
pub struct ReversiWasm {
    manager: MCTSManager<RvCfg>,
    cols: usize,
    rows: usize,
    anti: bool,
}
#[wasm_bindgen]
impl ReversiWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32, anti: u32) -> Self {
        let cols = ((cols as usize) & !1).clamp(4, 10); // even dims only
        let rows = ((rows as usize) & !1).clamp(4, 10);
        let anti = anti != 0;
        Self { manager: MCTSManager::new(Reversi::new(cols, rows, anti), RvCfg, RvEval, UCTPolicy::new(1.4), ()), cols, rows, anti }
    }
    pub fn cols(&self) -> u32 {
        self.cols as u32
    }
    pub fn rows(&self) -> u32 {
        self.rows as u32
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .board
            .iter()
            .map(|&v| match v {
                0 => 'X',
                1 => 'O',
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
            Some(ProvenValue::Win) => format!("{}", s.current + 1),
            Some(ProvenValue::Loss) => format!("{}", (1 - s.current) + 1),
            Some(ProvenValue::Draw) => "Draw".into(),
            _ => String::new(),
        }
    }
    /// Comma-separated legal cell indices, or "pass" if a pass is forced.
    pub fn legal_moves(&self) -> String {
        let m = self.manager.tree().root_state().available_moves();
        if m == vec![RvMove::Pass] {
            return "pass".into();
        }
        m.iter()
            .filter_map(|mv| match mv {
                RvMove::Place(i) => Some(i.to_string()),
                RvMove::Pass => None,
            })
            .collect::<Vec<_>>()
            .join(",")
    }
    /// Comma-separated "score0,score1".
    pub fn scores(&self) -> String {
        let s = self.manager.tree().root_state();
        format!("{},{}", s.count(0), s.count(1))
    }
    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m = if mov == "pass" {
            RvMove::Pass
        } else {
            match mov.parse::<u16>() {
                Ok(v) if (v as usize) < self.cols * self.rows => RvMove::Place(v),
                _ => return false,
            }
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, RvCfg, RvEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Reversi::new(self.cols, self.rows, self.anti), RvCfg, RvEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_has_four_legal_moves() {
        let g = ReversiWasm::new(6, 6, 0);
        assert_eq!(g.legal_moves().split(',').count(), 4);
        assert_eq!(g.scores(), "2,2");
    }

    #[test]
    fn placing_flips_a_disc() {
        let g0 = Reversi::new(6, 6, false);
        // find a legal move for player 0 and verify a flip occurs
        let mv = g0.available_moves()[0];
        let mut g = g0.clone();
        let before = g.count(0);
        g.make_move(&mv);
        // player 0 placed 1 + flipped >=1 -> count grew by >=2
        assert!(g.count(0) >= before + 2);
        assert_eq!(g.current, 1);
    }

    #[test]
    fn ai_plays() {
        let mut g = ReversiWasm::new(6, 6, 0);
        g.playout_n(300);
        let m = g.best_move().unwrap();
        assert!(g.apply_move(&m));
    }

    #[test]
    fn anti_reversi_awards_the_win_to_fewer_discs() {
        let mut g = ReversiWasm::new(4, 4, 1);
        // Drive to terminal with any legal sequence (reuse the pattern from the
        // existing full-game test), then assert result is the LOW-count seat.
        while !g.is_terminal() {
            let m = g.legal_moves().split(',').next().unwrap().to_string();
            assert!(g.apply_move(&m));
        }
        let b = g.get_board();
        let x = b.chars().filter(|&c| c == 'X').count();
        let o = b.chars().filter(|&c| c == 'O').count();
        let expect = if x < o { "1" } else if o < x { "2" } else { "Draw" };
        assert_eq!(g.result(), expect);
    }
}
