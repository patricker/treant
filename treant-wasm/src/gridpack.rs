//! "Grid pack" — five line/shape games sharing the grid helpers in `gridlib`.
//! Each is its own GameState (distinct terminal rules), but they all expose a
//! Tic-Tac-Toe-style WASM surface so the arcade can reuse one board renderer.
use crate::gridlib::*;
use crate::types;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

const SYMBOLS: [char; 4] = ['X', 'O', 'A', 'B'];

fn board_string(board: &[i8]) -> String {
    board
        .iter()
        .map(|&c| {
            if c < 0 {
                ' '
            } else {
                SYMBOLS.get(c as usize).copied().unwrap_or('?')
            }
        })
        .collect()
}

/// 2-player result string from a terminal value (current player's perspective).
fn result_str(term: Option<ProvenValue>, current: u8) -> String {
    match term {
        Some(ProvenValue::Win) => format!("{}", current + 1),
        Some(ProvenValue::Loss) => format!("{}", (1 - current) + 1),
        Some(ProvenValue::Draw) => "Draw".into(),
        _ => String::new(),
    }
}

// ============================================================ Connect Six =====
#[derive(Clone)]
struct Connect6 {
    board: Vec<i8>,
    cols: usize,
    rows: usize,
    current: u8,
    placed_turn: u8,
    total: u32,
}
impl Connect6 {
    fn new(cols: usize, rows: usize) -> Self {
        Self { board: vec![-1; cols * rows], cols, rows, current: 0, placed_turn: 0, total: 0 }
    }
    fn term(&self) -> Option<ProvenValue> {
        if let Some(w) = line_symbol(&self.board, self.cols, self.rows, 6) {
            Some(if w as u8 == self.current { ProvenValue::Win } else { ProvenValue::Loss })
        } else if is_full(&self.board) {
            Some(ProvenValue::Draw)
        } else {
            None
        }
    }
}
impl GameState for Connect6 {
    type Move = u16;
    type Player = u8;
    type MoveList = Vec<u16>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<u16> {
        if self.term().is_some() {
            return vec![];
        }
        (0..self.board.len()).filter(|&i| self.board[i] < 0).map(|i| i as u16).collect()
    }
    fn make_move(&mut self, m: &u16) {
        let quota = if self.total == 0 { 1 } else { 2 };
        self.board[*m as usize] = self.current as i8;
        self.total += 1;
        self.placed_turn += 1;
        if self.placed_turn >= quota {
            self.current = 1 - self.current;
            self.placed_turn = 0;
        }
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct C6Eval;
impl Evaluator<C6Cfg> for C6Eval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &Connect6, m: &Vec<u16>, _: Option<SearchHandle<C6Cfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], window_eval(&s.board, s.cols, s.rows, 0, 6) - window_eval(&s.board, s.cols, s.rows, 1, 6))
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &Connect6, _: &i64, _: SearchHandle<C6Cfg>) -> i64 {
        window_eval(&s.board, s.cols, s.rows, 0, 6) - window_eval(&s.board, s.cols, s.rows, 1, 6)
    }
}
#[derive(Default)]
struct C6Cfg;
impl MCTS for C6Cfg {
    type State = Connect6;
    type Eval = C6Eval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
}

// =========================================================== Order & Chaos ====
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OcMove {
    cell: u16,
    sym: u8,
}
impl std::fmt::Display for OcMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{},{}", self.cell, self.sym)
    }
}
#[derive(Clone)]
struct OrderChaos {
    board: Vec<i8>,
    cols: usize,
    rows: usize,
    current: u8, // 0 = Order, 1 = Chaos
}
impl OrderChaos {
    fn new(cols: usize, rows: usize) -> Self {
        Self { board: vec![-1; cols * rows], cols, rows, current: 0 }
    }
    fn term(&self) -> Option<ProvenValue> {
        if line_symbol(&self.board, self.cols, self.rows, 5).is_some() {
            // Order (player 0) wins on a 5-line of either symbol.
            Some(if self.current == 0 { ProvenValue::Win } else { ProvenValue::Loss })
        } else if is_full(&self.board) {
            // Chaos (player 1) wins on a full board with no line.
            Some(if self.current == 1 { ProvenValue::Win } else { ProvenValue::Loss })
        } else {
            None
        }
    }
}
impl GameState for OrderChaos {
    type Move = OcMove;
    type Player = u8;
    type MoveList = Vec<OcMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<OcMove> {
        if self.term().is_some() {
            return vec![];
        }
        let mut v = Vec::new();
        for i in 0..self.board.len() {
            if self.board[i] < 0 {
                v.push(OcMove { cell: i as u16, sym: 0 });
                v.push(OcMove { cell: i as u16, sym: 1 });
            }
        }
        v
    }
    fn make_move(&mut self, m: &OcMove) {
        self.board[m.cell as usize] = m.sym as i8;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct OcEval;
impl Evaluator<OcCfg> for OcEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &OrderChaos, m: &Vec<OcMove>, _: Option<SearchHandle<OcCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], line_threat(&s.board, s.cols, s.rows, 5))
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e } // Order likes threats, Chaos hates them
    }
    fn evaluate_existing_state(&self, s: &OrderChaos, _: &i64, _: SearchHandle<OcCfg>) -> i64 {
        line_threat(&s.board, s.cols, s.rows, 5)
    }
}
#[derive(Default)]
struct OcCfg;
impl MCTS for OcCfg {
    type State = OrderChaos;
    type Eval = OcEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

// ================================================================== Squava ====
#[derive(Clone)]
struct Squava {
    board: Vec<i8>,
    cols: usize,
    rows: usize,
    current: u8,
}
impl Squava {
    fn new(cols: usize, rows: usize) -> Self {
        Self { board: vec![-1; cols * rows], cols, rows, current: 0 }
    }
    fn term(&self) -> Option<ProvenValue> {
        // 4-in-a-row wins; otherwise 3-in-a-row loses (for whoever made it).
        if let Some(w) = line_symbol(&self.board, self.cols, self.rows, 4) {
            Some(if w as u8 == self.current { ProvenValue::Win } else { ProvenValue::Loss })
        } else if let Some(w) = line_symbol(&self.board, self.cols, self.rows, 3) {
            // The player with a 3-line loses, so the *other* wins.
            Some(if w as u8 == self.current { ProvenValue::Loss } else { ProvenValue::Win })
        } else if is_full(&self.board) {
            Some(ProvenValue::Draw)
        } else {
            None
        }
    }
}
impl GameState for Squava {
    type Move = u16;
    type Player = u8;
    type MoveList = Vec<u16>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<u16> {
        if self.term().is_some() {
            return vec![];
        }
        (0..self.board.len()).filter(|&i| self.board[i] < 0).map(|i| i as u16).collect()
    }
    fn make_move(&mut self, m: &u16) {
        self.board[*m as usize] = self.current as i8;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct SqEval;
impl Evaluator<SqCfg> for SqEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &Squava, m: &Vec<u16>, _: Option<SearchHandle<SqCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], squava_safety(&s.board, s.cols, s.rows, 0) - squava_safety(&s.board, s.cols, s.rows, 1))
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &Squava, _: &i64, _: SearchHandle<SqCfg>) -> i64 {
        squava_safety(&s.board, s.cols, s.rows, 0) - squava_safety(&s.board, s.cols, s.rows, 1)
    }
}
#[derive(Default)]
struct SqCfg;
impl MCTS for SqCfg {
    type State = Squava;
    type Eval = SqEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

// ================================================================= Notakto ====
#[derive(Clone)]
struct Notakto {
    board: Vec<i8>,
    cols: usize,
    rows: usize,
    current: u8,
}
impl Notakto {
    fn new(cols: usize, rows: usize) -> Self {
        Self { board: vec![-1; cols * rows], cols, rows, current: 0 }
    }
    fn term(&self) -> Option<ProvenValue> {
        // Shared mark (symbol 0); completing a 3-line loses -> current (who did
        // NOT just move) wins.
        if has_line(&self.board, self.cols, self.rows, 0, 3) {
            Some(ProvenValue::Win)
        } else if is_full(&self.board) {
            Some(ProvenValue::Draw)
        } else {
            None
        }
    }
}
impl GameState for Notakto {
    type Move = u16;
    type Player = u8;
    type MoveList = Vec<u16>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<u16> {
        if self.term().is_some() {
            return vec![];
        }
        (0..self.board.len()).filter(|&i| self.board[i] < 0).map(|i| i as u16).collect()
    }
    fn make_move(&mut self, m: &u16) {
        self.board[*m as usize] = 0; // everyone places the same mark
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct NkEval;
impl Evaluator<NkCfg> for NkEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Notakto, m: &Vec<u16>, _: Option<SearchHandle<NkCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Notakto, _: &i64, _: SearchHandle<NkCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct NkCfg;
impl MCTS for NkCfg {
    type State = Notakto;
    type Eval = NkEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

// ================================================================ Square Up ====
#[derive(Clone)]
struct SquareUp {
    board: Vec<i8>,
    cols: usize,
    rows: usize,
    current: u8,
}
impl SquareUp {
    fn new(cols: usize, rows: usize) -> Self {
        Self { board: vec![-1; cols * rows], cols, rows, current: 0 }
    }
    fn term(&self) -> Option<ProvenValue> {
        for p in 0..2i8 {
            if has_square(&self.board, self.cols, self.rows, p) {
                return Some(if p as u8 == self.current { ProvenValue::Win } else { ProvenValue::Loss });
            }
        }
        if is_full(&self.board) {
            Some(ProvenValue::Draw)
        } else {
            None
        }
    }
}
impl GameState for SquareUp {
    type Move = u16;
    type Player = u8;
    type MoveList = Vec<u16>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<u16> {
        if self.term().is_some() {
            return vec![];
        }
        (0..self.board.len()).filter(|&i| self.board[i] < 0).map(|i| i as u16).collect()
    }
    fn make_move(&mut self, m: &u16) {
        self.board[*m as usize] = self.current as i8;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct SuEval;
impl Evaluator<SuCfg> for SuEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &SquareUp, m: &Vec<u16>, _: Option<SearchHandle<SuCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], center_eval(&s.board, s.cols, s.rows, 0) - center_eval(&s.board, s.cols, s.rows, 1))
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &SquareUp, _: &i64, _: SearchHandle<SuCfg>) -> i64 {
        center_eval(&s.board, s.cols, s.rows, 0) - center_eval(&s.board, s.cols, s.rows, 1)
    }
}
#[derive(Default)]
struct SuCfg;
impl MCTS for SuCfg {
    type State = SquareUp;
    type Eval = SuEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

// ================================================================ WASM API ====

/// Parse a plain cell-index move ("12"), validating it against the board size.
fn parse_cell(mov: &str, cols: usize, rows: usize) -> Option<u16> {
    mov.parse::<u16>().ok().filter(|&v| (v as usize) < cols * rows)
}

/// Parse an Order & Chaos "cell,sym" move (sym is 0 = X or 1 = O).
fn parse_oc_move(mov: &str, cols: usize, rows: usize) -> Option<OcMove> {
    let (c, s) = mov.split_once(',')?;
    let cell: u16 = c.parse().ok()?;
    let sym: u8 = s.parse().ok()?;
    if (cell as usize) < cols * rows && sym < 2 {
        Some(OcMove { cell, sym })
    } else {
        None
    }
}

// One WASM surface for every grid-pack game. `$movety` + `$parse` (a
// `fn(&str, cols, rows) -> Option<$movety>`) are the only things that vary:
// most games take a plain cell index, Order & Chaos takes a (cell, symbol) pair.
macro_rules! cell_game_wasm {
    ($wasm:ident, $game:ident, $cfg:ident, $eval:ident, $c:expr, $movety:ty, $parse:expr) => {
        #[wasm_bindgen]
        pub struct $wasm {
            manager: MCTSManager<$cfg>,
            cols: usize,
            rows: usize,
        }
        #[wasm_bindgen]
        impl $wasm {
            #[wasm_bindgen(constructor)]
            pub fn new(cols: u32, rows: u32) -> Self {
                let cols = (cols as usize).clamp(3, MAX_DIM);
                let rows = (rows as usize).clamp(3, MAX_DIM);
                Self {
                    manager: MCTSManager::new($game::new(cols, rows), $cfg, $eval, UCTPolicy::new($c), ()),
                    cols,
                    rows,
                }
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
                let stats = types::build_stats(&self.manager, |_| None);
                serde_wasm_bindgen::to_value(&stats).unwrap_or(JsValue::NULL)
            }
            pub fn get_board(&self) -> String {
                board_string(&self.manager.tree().root_state().board)
            }
            pub fn current_player(&self) -> u32 {
                self.manager.tree().root_state().current as u32
            }
            pub fn is_terminal(&self) -> bool {
                self.manager.tree().root_state().term().is_some()
            }
            pub fn result(&self) -> String {
                let s = self.manager.tree().root_state();
                result_str(s.term(), s.current)
            }
            pub fn best_move(&self) -> Option<String> {
                self.manager.best_move().map(|m| format!("{m}"))
            }
            pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
                crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
            }
            pub fn apply_move(&mut self, mov: &str) -> bool {
                let parse: fn(&str, usize, usize) -> Option<$movety> = $parse;
                let m: $movety = match parse(mov, self.cols, self.rows) {
                    Some(v) => v,
                    None => return false,
                };
                let mut s = self.manager.tree().root_state().clone();
                if !s.available_moves().contains(&m) {
                    return false;
                }
                s.make_move(&m);
                self.manager = MCTSManager::new(s, $cfg, $eval, UCTPolicy::new($c), ());
                true
            }
            pub fn reset(&mut self) {
                self.manager = MCTSManager::new($game::new(self.cols, self.rows), $cfg, $eval, UCTPolicy::new($c), ());
            }
        }
    };
}

cell_game_wasm!(Connect6Wasm, Connect6, C6Cfg, C6Eval, 1.6, u16, parse_cell);
cell_game_wasm!(SquavaWasm, Squava, SqCfg, SqEval, 1.4, u16, parse_cell);
cell_game_wasm!(NotaktoWasm, Notakto, NkCfg, NkEval, 1.4, u16, parse_cell);
cell_game_wasm!(SquareUpWasm, SquareUp, SuCfg, SuEval, 1.4, u16, parse_cell);
// Order & Chaos differs only in its (cell, symbol) move — "cell,sym", sym 0/1.
cell_game_wasm!(OrderChaosWasm, OrderChaos, OcCfg, OcEval, 1.5, OcMove, parse_oc_move);

#[cfg(test)]
mod tests {
    use super::*;

    // The pre-fix Squava evaluator: pure center control, blind to the misère
    // 3-in-a-row trap. Kept here only so the self-play test can prove the new
    // safety-aware eval is genuinely stronger, not merely different.
    struct SqEvalOldCenter;
    impl Evaluator<SqCfgOldCenter> for SqEvalOldCenter {
        type StateEvaluation = i64;
        fn evaluate_new_state(
            &self,
            s: &Squava,
            m: &Vec<u16>,
            _: Option<SearchHandle<SqCfgOldCenter>>,
        ) -> (Vec<()>, i64) {
            (vec![(); m.len()], center_eval(&s.board, s.cols, s.rows, 0) - center_eval(&s.board, s.cols, s.rows, 1))
        }
        fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
            if *p == 0 { *e } else { -*e }
        }
        fn evaluate_existing_state(&self, s: &Squava, _: &i64, _: SearchHandle<SqCfgOldCenter>) -> i64 {
            center_eval(&s.board, s.cols, s.rows, 0) - center_eval(&s.board, s.cols, s.rows, 1)
        }
    }
    #[derive(Default)]
    struct SqCfgOldCenter;
    impl MCTS for SqCfgOldCenter {
        type State = Squava;
        type Eval = SqEvalOldCenter;
        type NodeData = ();
        type ExtraThreadData = ();
        type TreePolicy = UCTPolicy;
        type TranspositionTable = ();
        fn solver_enabled(&self) -> bool {
            true
        }
    }

    fn squava_best_new(state: &Squava, playouts: u64) -> Option<u16> {
        let mut m = MCTSManager::new(state.clone(), SqCfg, SqEval, UCTPolicy::new(1.4), ());
        m.playout_n(playouts);
        m.best_move()
    }
    fn squava_best_old(state: &Squava, playouts: u64) -> Option<u16> {
        let mut m = MCTSManager::new(state.clone(), SqCfgOldCenter, SqEvalOldCenter, UCTPolicy::new(1.4), ());
        m.playout_n(playouts);
        m.best_move()
    }

    // Play one 5x5 game; `new_first` = the new eval is player 0. Returns the
    // winning player (0/1), or None for a draw.
    fn squava_play(new_first: bool, playouts: u64) -> Option<u8> {
        let mut s = Squava::new(5, 5);
        while s.term().is_none() {
            let new_to_move = (s.current == 0) == new_first;
            let mv = if new_to_move {
                squava_best_new(&s, playouts)
            } else {
                squava_best_old(&s, playouts)
            };
            match mv {
                Some(m) => s.make_move(&m),
                None => break,
            }
        }
        match s.term() {
            Some(ProvenValue::Win) => Some(s.current),
            Some(ProvenValue::Loss) => Some(1 - s.current),
            _ => None,
        }
    }

    #[test]
    fn squava_safety_eval_beats_center_eval() {
        // Head-to-head: the new safety-aware eval should clearly out-play the old
        // center-control eval at equal search. Alternate who moves first to cancel
        // any first-player edge. (When written, the new eval swept 16-0.)
        let playouts = 600;
        let pairs = 8;
        let (mut new_wins, mut old_wins) = (0, 0);
        for i in 0..pairs {
            for &new_first in &[true, false] {
                let new_player: u8 = if new_first { 0 } else { 1 };
                match squava_play(new_first, playouts + i) {
                    Some(w) if w == new_player => new_wins += 1,
                    Some(_) => old_wins += 1,
                    None => {}
                }
            }
        }
        assert!(
            new_wins > old_wins,
            "safety eval should beat center eval: new={new_wins} old={old_wins}"
        );
    }

    #[test]
    fn squava_three_in_a_row_loses() {
        // Player 0 makes a 3-line on a 5x5 -> player 0 loses -> result is player 2.
        let mut g = SquavaWasm::new(5, 5);
        // 0:(0,0) 1:(4,4) 0:(0,1) 1:(4,3) 0:(0,2) -> 0 has 3-in-row top -> loses
        for m in ["0", "24", "1", "23", "2"] {
            assert!(g.apply_move(m), "move {m}");
        }
        assert!(g.is_terminal());
        assert_eq!(g.result(), "2"); // player 0 lost, so player 2 (index 1) wins
    }

    #[test]
    fn notakto_completing_line_loses() {
        // 3x3, shared mark; player 0 completes top row -> player 0 loses -> "2".
        let mut g = NotaktoWasm::new(3, 3);
        for m in ["0", "3", "1", "4", "2"] {
            assert!(g.apply_move(m), "move {m}");
        }
        assert!(g.is_terminal());
        assert_eq!(g.result(), "2");
    }

    #[test]
    fn order_wins_on_a_line() {
        // 5x5 (min for a 5-line); Order forces 5 X's in the top row.
        let mut g = OrderChaosWasm::new(5, 5);
        // Order places X across the top; Chaos plays elsewhere. cells 0..4 top row.
        let seq = [("0", 0), ("10", 1), ("1", 0), ("11", 1), ("2", 0), ("12", 1), ("3", 0), ("13", 1), ("4", 0)];
        for (cell, sym) in seq {
            assert!(g.apply_move(&format!("{cell},{sym}")), "{cell},{sym}");
        }
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1"); // Order = player 1 (index 0)
    }

    #[test]
    fn square_up_detects_a_square() {
        // Player 0 forms an axis-aligned 2x2 square on a 4x4: cells 0,1,4,5.
        let mut g = SquareUpWasm::new(4, 4);
        for m in ["0", "8", "1", "9", "4", "10", "5"] {
            assert!(g.apply_move(m), "move {m}");
        }
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1");
    }

    #[test]
    fn result_is_empty_while_in_progress() {
        // The canonical contract: result() is "" until a terminal position. Pin it
        // for a macro-generated game both fresh and mid-game (never asserted before).
        let mut g = NotaktoWasm::new(3, 3);
        assert_eq!(g.result(), "", "fresh board");
        assert!(!g.is_terminal());
        assert!(g.apply_move("0"));
        assert_eq!(g.result(), "", "after one move");
        assert!(!g.is_terminal());
    }

    #[test]
    fn connect6_result_maps_to_bare_winner_digit() {
        // Connect6's 2-stones-per-turn `current` toggling makes the terminal
        // mapping non-obvious, so assert it directly. Black (seat 0) builds a
        // 6-in-a-row across the top row on a 9×9; white dumps stones in row 4
        // (max run 5, no line). result() must be the bare digit "1", never "P1".
        let mut g = Connect6Wasm::new(9, 9);
        let moves = ["0", "40", "41", "1", "2", "42", "43", "3", "4", "44", "45", "5"];
        for m in moves {
            assert_eq!(g.result(), "", "still in progress before {m}");
            assert!(g.apply_move(m), "move {m}");
        }
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1"); // black (seat 0) completed the line
    }

    #[test]
    fn connect6_two_placements_per_turn() {
        let mut g = Connect6Wasm::new(9, 9);
        assert!(g.apply_move("0")); // black opening: 1 stone, then switches
        assert_eq!(g.current_player(), 1);
        assert!(g.apply_move("80")); // white stone 1 of 2
        assert_eq!(g.current_player(), 1, "white still has a second stone");
        assert!(g.apply_move("79")); // white stone 2 of 2
        assert_eq!(g.current_player(), 0, "back to black");
    }
}
