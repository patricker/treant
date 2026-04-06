use mcts::tree_policy::*;
use mcts::*;
use wasm_bindgen::prelude::*;

use crate::types;

// --- Game ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    Empty,
    Red,
    Yellow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Player {
    Red,
    Yellow,
}

#[derive(Clone, Debug, PartialEq)]
struct CfMove(u8);

impl std::fmt::Display for CfMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

const COLS: usize = 7;
const ROWS: usize = 6;

#[derive(Clone, Debug)]
struct ConnectFour {
    board: [[Cell; COLS]; ROWS],
    current: Player,
}

impl ConnectFour {
    fn new() -> Self {
        Self {
            board: [[Cell::Empty; COLS]; ROWS],
            current: Player::Red,
        }
    }

    fn cell_for_player(player: Player) -> Cell {
        match player {
            Player::Red => Cell::Red,
            Player::Yellow => Cell::Yellow,
        }
    }

    /// Find the lowest empty row in a column, or None if full.
    fn drop_row(&self, col: usize) -> Option<usize> {
        (0..ROWS).find(|&row| self.board[row][col] == Cell::Empty)
    }

    /// Check if the given cell color has 4 in a row through (row, col).
    fn check_win_at(&self, row: usize, col: usize, cell: Cell) -> bool {
        if self.board[row][col] != cell {
            return false;
        }
        // Directions: horizontal, vertical, diagonal /, diagonal \.
        let directions: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];
        for (dr, dc) in directions {
            let mut count = 1;
            // Positive direction
            for i in 1..4 {
                let r = row as i32 + dr * i;
                let c = col as i32 + dc * i;
                if r < 0 || r >= ROWS as i32 || c < 0 || c >= COLS as i32 {
                    break;
                }
                if self.board[r as usize][c as usize] != cell {
                    break;
                }
                count += 1;
            }
            // Negative direction
            for i in 1..4 {
                let r = row as i32 - dr * i;
                let c = col as i32 - dc * i;
                if r < 0 || r >= ROWS as i32 || c < 0 || c >= COLS as i32 {
                    break;
                }
                if self.board[r as usize][c as usize] != cell {
                    break;
                }
                count += 1;
            }
            if count >= 4 {
                return true;
            }
        }
        false
    }

    /// Check if the player who just moved won.
    fn has_winner(&self) -> Option<Player> {
        for row in 0..ROWS {
            for col in 0..COLS {
                match self.board[row][col] {
                    Cell::Red if self.check_win_at(row, col, Cell::Red) => return Some(Player::Red),
                    Cell::Yellow if self.check_win_at(row, col, Cell::Yellow) => {
                        return Some(Player::Yellow)
                    }
                    _ => {}
                }
            }
        }
        None
    }

    fn is_full(&self) -> bool {
        (0..COLS).all(|col| self.board[ROWS - 1][col] != Cell::Empty)
    }

    fn board_string(&self) -> String {
        let mut s = String::with_capacity(ROWS * COLS);
        // Top row first (row 5 down to row 0)
        for row in (0..ROWS).rev() {
            for col in 0..COLS {
                s.push(match self.board[row][col] {
                    Cell::Empty => ' ',
                    Cell::Red => 'R',
                    Cell::Yellow => 'Y',
                });
            }
        }
        s
    }
}

impl GameState for ConnectFour {
    type Move = CfMove;
    type Player = Player;
    type MoveList = Vec<CfMove>;

    fn current_player(&self) -> Player {
        self.current
    }

    fn available_moves(&self) -> Vec<CfMove> {
        // No moves if someone won
        if self.has_winner().is_some() {
            return vec![];
        }
        (0..COLS as u8)
            .filter(|&col| self.drop_row(col as usize).is_some())
            .map(CfMove)
            .collect()
    }

    fn make_move(&mut self, mov: &CfMove) {
        let col = mov.0 as usize;
        if let Some(row) = self.drop_row(col) {
            self.board[row][col] = Self::cell_for_player(self.current);
        }
        self.current = match self.current {
            Player::Red => Player::Yellow,
            Player::Yellow => Player::Red,
        };
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        // If someone won, the winner just moved, so current player lost.
        if self.has_winner().is_some() {
            return Some(ProvenValue::Loss);
        }
        if self.is_full() {
            return Some(ProvenValue::Draw);
        }
        None
    }
}

// --- Evaluator ---

struct CfEval;

impl Evaluator<CfConfig> for CfEval {
    type StateEvaluation = i64;

    fn evaluate_new_state(
        &self,
        state: &ConnectFour,
        moves: &Vec<CfMove>,
        _: Option<SearchHandle<CfConfig>>,
    ) -> (Vec<()>, i64) {
        // Center-control heuristic: count pieces in columns 2-4.
        let mut score: i64 = 0;
        for row in 0..ROWS {
            for col in 2..=4 {
                match state.board[row][col] {
                    Cell::Red => score += 1,
                    Cell::Yellow => score -= 1,
                    Cell::Empty => {}
                }
            }
        }
        (vec![(); moves.len()], score)
    }

    fn interpret_evaluation_for_player(&self, evaln: &i64, player: &Player) -> i64 {
        match player {
            Player::Red => *evaln,
            Player::Yellow => -*evaln,
        }
    }

    fn evaluate_existing_state(
        &self,
        _: &ConnectFour,
        evaln: &i64,
        _: SearchHandle<CfConfig>,
    ) -> i64 {
        *evaln
    }
}

// --- MCTS Config ---

#[derive(Default)]
struct CfConfig;

impl MCTS for CfConfig {
    type State = ConnectFour;
    type Eval = CfEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();

    fn solver_enabled(&self) -> bool {
        false
    }
}

// --- WASM API ---

#[wasm_bindgen]
pub struct ConnectFourWasm {
    manager: MCTSManager<CfConfig>,
}

impl Default for ConnectFourWasm {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl ConnectFourWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            manager: MCTSManager::new(
                ConnectFour::new(),
                CfConfig,
                CfEval,
                UCTPolicy::new(1.4),
                (),
            ),
        }
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        let stats = types::build_stats(&self.manager, |_| None);
        serde_wasm_bindgen::to_value(&stats).unwrap()
    }

    pub fn get_tree(&self, max_depth: u32) -> JsValue {
        let tree =
            types::export_tree::<CfConfig>(self.manager.tree().root_node(), max_depth, &|_| None);
        serde_wasm_bindgen::to_value(&tree).unwrap()
    }

    pub fn get_board(&self) -> String {
        self.manager.tree().root_state().board_string()
    }

    pub fn current_player(&self) -> String {
        match self.manager.tree().root_state().current {
            Player::Red => "Red".into(),
            Player::Yellow => "Yellow".into(),
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.manager
            .tree()
            .root_state()
            .terminal_value()
            .is_some()
    }

    pub fn result(&self) -> String {
        let state = self.manager.tree().root_state();
        if let Some(winner) = state.has_winner() {
            match winner {
                Player::Red => "Red".into(),
                Player::Yellow => "Yellow".into(),
            }
        } else if state.is_full() {
            "Draw".into()
        } else {
            String::new()
        }
    }

    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    /// Apply a move and advance the tree (preserving search).
    pub fn apply_move(&mut self, col: &str) -> bool {
        let col_num: u8 = match col.parse() {
            Ok(n) if n < COLS as u8 => n,
            _ => return false,
        };
        let m = CfMove(col_num);
        self.manager.advance(&m).is_ok()
    }

    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(
            ConnectFour::new(),
            CfConfig,
            CfEval,
            UCTPolicy::new(1.4),
            (),
        );
    }
}
