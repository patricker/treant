use mcts::tree_policy::*;
use mcts::*;
use wasm_bindgen::prelude::*;

use crate::types;

// --- Game ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    Empty,
    X,
    O,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Player {
    X,
    O,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TttMove(u8);

impl std::fmt::Display for TttMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug)]
struct TicTacToe {
    board: [Cell; 9],
    current: Player,
}

const WIN_LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

impl TicTacToe {
    fn new() -> Self {
        Self {
            board: [Cell::Empty; 9],
            current: Player::X,
        }
    }

    fn winner(&self) -> Option<Player> {
        for line in &WIN_LINES {
            let a = self.board[line[0]];
            let b = self.board[line[1]];
            let c = self.board[line[2]];
            if a != Cell::Empty && a == b && b == c {
                return match a {
                    Cell::X => Some(Player::X),
                    Cell::O => Some(Player::O),
                    Cell::Empty => unreachable!(),
                };
            }
        }
        None
    }

    fn board_full(&self) -> bool {
        self.board.iter().all(|c| *c != Cell::Empty)
    }

    /// Return the result string: "X", "O", "Draw", or "" (not over).
    fn result_str(&self) -> &'static str {
        if let Some(w) = self.winner() {
            match w {
                Player::X => "X",
                Player::O => "O",
            }
        } else if self.board_full() {
            "Draw"
        } else {
            ""
        }
    }
}

impl GameState for TicTacToe {
    type Move = TttMove;
    type Player = Player;
    type MoveList = Vec<TttMove>;

    fn current_player(&self) -> Player {
        self.current
    }

    fn available_moves(&self) -> Vec<TttMove> {
        // No moves if the game is already won
        if self.winner().is_some() {
            return vec![];
        }
        self.board
            .iter()
            .enumerate()
            .filter(|(_, c)| **c == Cell::Empty)
            .map(|(i, _)| TttMove(i as u8))
            .collect()
    }

    fn make_move(&mut self, mov: &TttMove) {
        let cell = match self.current {
            Player::X => Cell::X,
            Player::O => Cell::O,
        };
        self.board[mov.0 as usize] = cell;
        self.current = match self.current {
            Player::X => Player::O,
            Player::O => Player::X,
        };
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.winner().is_some() {
            // The winner just moved, so current player lost
            Some(ProvenValue::Loss)
        } else if self.board_full() {
            Some(ProvenValue::Draw)
        } else {
            None
        }
    }
}

// --- Evaluator ---

struct TttEval;

impl Evaluator<TttConfig> for TttEval {
    type StateEvaluation = ();

    fn evaluate_new_state(
        &self,
        _state: &TicTacToe,
        moves: &Vec<TttMove>,
        _: Option<SearchHandle<TttConfig>>,
    ) -> (Vec<()>, ()) {
        (vec![(); moves.len()], ())
    }

    fn interpret_evaluation_for_player(&self, _evaln: &(), _player: &Player) -> i64 {
        0
    }

    fn evaluate_existing_state(
        &self,
        _: &TicTacToe,
        _evaln: &(),
        _: SearchHandle<TttConfig>,
    ) {
    }
}

// --- MCTS Config ---

#[derive(Default)]
struct TttConfig;

impl MCTS for TttConfig {
    type State = TicTacToe;
    type Eval = TttEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();

    fn solver_enabled(&self) -> bool {
        true
    }
}

// --- WASM API ---

#[wasm_bindgen]
pub struct TicTacToeWasm {
    manager: MCTSManager<TttConfig>,
}

impl Default for TicTacToeWasm {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl TicTacToeWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            manager: MCTSManager::new(
                TicTacToe::new(),
                TttConfig,
                TttEval,
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
            types::export_tree::<TttConfig>(self.manager.tree().root_node(), max_depth, &|_| None);
        serde_wasm_bindgen::to_value(&tree).unwrap()
    }

    /// 9-character string representing the board: ' '=empty, 'X', 'O'.
    pub fn get_board(&self) -> String {
        let state = self.manager.tree().root_state();
        state
            .board
            .iter()
            .map(|c| match c {
                Cell::Empty => ' ',
                Cell::X => 'X',
                Cell::O => 'O',
            })
            .collect()
    }

    pub fn current_player(&self) -> String {
        match self.manager.tree().root_state().current {
            Player::X => "X".into(),
            Player::O => "O".into(),
        }
    }

    pub fn is_terminal(&self) -> bool {
        let state = self.manager.tree().root_state();
        state.winner().is_some() || state.board_full()
    }

    /// Return "X", "O", "Draw", or "" (not over).
    pub fn result(&self) -> String {
        self.manager.tree().root_state().result_str().into()
    }

    pub fn root_proven_value(&self) -> String {
        format!("{:?}", self.manager.root_proven_value())
    }

    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    /// Apply a move by cell index (0-8) and advance the tree.
    /// Apply a move and advance the tree.
    /// Runs a few playouts first if needed to ensure the child is expanded.
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let idx: u8 = match mov.parse() {
            Ok(v) if v < 9 => v,
            _ => return false,
        };
        let m = TttMove(idx);
        if self.manager.advance(&m).is_ok() {
            return true;
        }
        self.manager.playout_n(100);
        self.manager.advance(&m).is_ok()
    }

    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(
            TicTacToe::new(),
            TttConfig,
            TttEval,
            UCTPolicy::new(1.4),
            (),
        );
    }
}
