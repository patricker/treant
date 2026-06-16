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
        (vec![(); m.len()], center_eval(&s.board, s.cols, s.rows, 0) - center_eval(&s.board, s.cols, s.rows, 1))
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 { *e } else { -*e }
    }
    fn evaluate_existing_state(&self, s: &Squava, _: &i64, _: SearchHandle<SqCfg>) -> i64 {
        center_eval(&s.board, s.cols, s.rows, 0) - center_eval(&s.board, s.cols, s.rows, 1)
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
macro_rules! cell_game_wasm {
    ($wasm:ident, $game:ident, $cfg:ident, $eval:ident, $c:expr) => {
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
                serde_wasm_bindgen::to_value(&stats).unwrap()
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
            pub fn apply_move(&mut self, mov: &str) -> bool {
                let m: u16 = match mov.parse() {
                    Ok(v) if (v as usize) < self.cols * self.rows => v,
                    _ => return false,
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

cell_game_wasm!(Connect6Wasm, Connect6, C6Cfg, C6Eval, 1.6);
cell_game_wasm!(SquavaWasm, Squava, SqCfg, SqEval, 1.4);
cell_game_wasm!(NotaktoWasm, Notakto, NkCfg, NkEval, 1.4);
cell_game_wasm!(SquareUpWasm, SquareUp, SuCfg, SuEval, 1.4);

// Order & Chaos has a (cell, symbol) move, so its own thin wrapper.
#[wasm_bindgen]
pub struct OrderChaosWasm {
    manager: MCTSManager<OcCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl OrderChaosWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32) -> Self {
        let cols = (cols as usize).clamp(3, MAX_DIM);
        let rows = (rows as usize).clamp(3, MAX_DIM);
        Self { manager: MCTSManager::new(OrderChaos::new(cols, rows), OcCfg, OcEval, UCTPolicy::new(1.5), ()), cols, rows }
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
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap()
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
    /// Move format: "cell,sym" where sym is 0 (X) or 1 (O).
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let parts: Vec<&str> = mov.split(',').collect();
        if parts.len() != 2 {
            return false;
        }
        let (cell, sym): (u16, u8) = match (parts[0].parse(), parts[1].parse()) {
            (Ok(c), Ok(s)) if (c as usize) < self.cols * self.rows && s < 2 => (c, s),
            _ => return false,
        };
        let m = OcMove { cell, sym };
        let mut st = self.manager.tree().root_state().clone();
        if !st.available_moves().contains(&m) {
            return false;
        }
        st.make_move(&m);
        self.manager = MCTSManager::new(st, OcCfg, OcEval, UCTPolicy::new(1.5), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(OrderChaos::new(self.cols, self.rows), OcCfg, OcEval, UCTPolicy::new(1.5), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
