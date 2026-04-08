use mcts::tree_policy::*;
use mcts::*;
use wasm_bindgen::prelude::*;

use crate::types;

// --- Shift: configurable board, k-in-a-row, pieces per player, N players ---
// Place phase: place pieces on empty cells until you have your quota.
// Move phase: pick one of your pieces and move it to any empty cell.
// Win: k in a row.

const MAX_COLS: usize = 10;
const MAX_ROWS: usize = 10;
const MAX_CELLS: usize = MAX_COLS * MAX_ROWS;
const MAX_PLAYERS: usize = 4;

const PLAYER_SYMBOLS: [char; MAX_PLAYERS] = ['X', 'O', 'A', 'B'];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    Empty,
    Player(u8),
}

/// Move: either Place(cell) or Shift(from, to).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ShiftMove {
    from: Option<u8>,
    to: u8,
}

impl std::fmt::Display for ShiftMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.from {
            None => write!(f, "P{}", self.to),
            Some(fr) => write!(f, "M{},{}", fr, self.to),
        }
    }
}

impl ShiftMove {
    fn encode(&self) -> String {
        format!("{}", self)
    }

    fn decode(s: &str) -> Option<Self> {
        if let Some(rest) = s.strip_prefix('P') {
            let to: u8 = rest.parse().ok()?;
            Some(ShiftMove { from: None, to })
        } else if let Some(rest) = s.strip_prefix('M') {
            let parts: Vec<&str> = rest.split(',').collect();
            if parts.len() == 2 {
                let from: u8 = parts[0].parse().ok()?;
                let to: u8 = parts[1].parse().ok()?;
                Some(ShiftMove { from: Some(from), to })
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
struct ShiftGame {
    board: [Cell; MAX_CELLS],
    current: u8,
    cols: usize,
    rows: usize,
    k: usize,
    num_players: usize,
    pieces_per_player: u8,
    pieces_placed: [u8; MAX_PLAYERS],
}

impl ShiftGame {
    fn new(cols: usize, rows: usize, k: usize, num_players: usize, pieces_per_player: u8) -> Self {
        Self {
            board: [Cell::Empty; MAX_CELLS],
            current: 0,
            cols,
            rows,
            k,
            num_players,
            pieces_per_player,
            pieces_placed: [0; MAX_PLAYERS],
        }
    }

    fn cell_count(&self) -> usize {
        self.cols * self.rows
    }

    fn in_placement_phase(&self) -> bool {
        self.pieces_placed[self.current as usize] < self.pieces_per_player
    }

    fn winner(&self) -> Option<u8> {
        let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
        for r in 0..self.rows {
            for c in 0..self.cols {
                if let Cell::Player(p) = self.board[r * self.cols + c] {
                    for &(dr, dc) in &dirs {
                        let mut count = 1usize;
                        for step in 1..self.k {
                            let nr = r as i32 + dr * step as i32;
                            let nc = c as i32 + dc * step as i32;
                            if nr < 0 || nr >= self.rows as i32 || nc < 0 || nc >= self.cols as i32
                            {
                                break;
                            }
                            if self.board[nr as usize * self.cols + nc as usize] == Cell::Player(p)
                            {
                                count += 1;
                            } else {
                                break;
                            }
                        }
                        if count >= self.k {
                            return Some(p);
                        }
                    }
                }
            }
        }
        None
    }

    fn board_string(&self) -> String {
        (0..self.cell_count())
            .map(|i| match self.board[i] {
                Cell::Empty => ' ',
                Cell::Player(p) => PLAYER_SYMBOLS.get(p as usize).copied().unwrap_or('?'),
            })
            .collect()
    }

    fn evaluate_for(&self, player: u8) -> i64 {
        let my_cell = Cell::Player(player);
        let mut score: i64 = 0;

        let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
        for r in 0..self.rows {
            for c in 0..self.cols {
                for &(dr, dc) in &dirs {
                    let end_r = r as i32 + dr * (self.k as i32 - 1);
                    let end_c = c as i32 + dc * (self.k as i32 - 1);
                    if end_r < 0 || end_r >= self.rows as i32 || end_c < 0 || end_c >= self.cols as i32 {
                        continue;
                    }

                    let mut mine = 0usize;
                    let mut empty = 0usize;
                    let mut theirs = 0usize;
                    for step in 0..self.k {
                        let idx = (r as i32 + dr * step as i32) as usize * self.cols
                            + (c as i32 + dc * step as i32) as usize;
                        match self.board[idx] {
                            c if c == my_cell => mine += 1,
                            Cell::Empty => empty += 1,
                            _ => theirs += 1,
                        }
                    }

                    if mine == self.k {
                        score += 1000;
                    } else if theirs == self.k {
                        score -= 1000;
                    } else if mine == self.k - 1 && empty == 1 {
                        score += 50;
                    } else if theirs == self.k - 1 && empty == 1 {
                        score -= 80;
                    } else if mine >= 2 && theirs == 0 {
                        score += 5;
                    }
                }
            }
        }

        // Center bonus
        let center_r = self.rows / 2;
        let center_c = self.cols / 2;
        if self.board[center_r * self.cols + center_c] == my_cell {
            score += 10;
        }

        score
    }
}

impl GameState for ShiftGame {
    type Move = ShiftMove;
    type Player = u8;
    type MoveList = Vec<ShiftMove>;

    fn current_player(&self) -> u8 {
        self.current
    }

    fn available_moves(&self) -> Vec<ShiftMove> {
        if self.winner().is_some() {
            return vec![];
        }

        let mut moves = Vec::new();
        let n = self.cell_count();

        if self.in_placement_phase() {
            for i in 0..n {
                if self.board[i] == Cell::Empty {
                    moves.push(ShiftMove { from: None, to: i as u8 });
                }
            }
        } else {
            let my_cell = Cell::Player(self.current);
            let my_pieces: Vec<u8> = (0..n as u8)
                .filter(|&i| self.board[i as usize] == my_cell)
                .collect();
            let empty_cells: Vec<u8> = (0..n as u8)
                .filter(|&i| self.board[i as usize] == Cell::Empty)
                .collect();
            for &from in &my_pieces {
                for &to in &empty_cells {
                    moves.push(ShiftMove { from: Some(from), to });
                }
            }
        }

        moves
    }

    fn make_move(&mut self, mov: &ShiftMove) {
        match mov.from {
            None => {
                self.board[mov.to as usize] = Cell::Player(self.current);
                self.pieces_placed[self.current as usize] += 1;
            }
            Some(from) => {
                self.board[from as usize] = Cell::Empty;
                self.board[mov.to as usize] = Cell::Player(self.current);
            }
        }
        self.current = (self.current + 1) % self.num_players as u8;
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.winner().is_some() {
            Some(ProvenValue::Loss)
        } else {
            None
        }
    }
}

// --- Evaluator ---

struct ShiftEval;

#[derive(Clone, Debug)]
struct ShiftStateEval {
    score: i64,
    player: u8,
}

impl Evaluator<ShiftConfig> for ShiftEval {
    type StateEvaluation = ShiftStateEval;

    fn evaluate_new_state(
        &self,
        state: &ShiftGame,
        moves: &Vec<ShiftMove>,
        _: Option<SearchHandle<ShiftConfig>>,
    ) -> (Vec<()>, ShiftStateEval) {
        let player = state.current;
        (
            vec![(); moves.len()],
            ShiftStateEval { score: state.evaluate_for(player), player },
        )
    }

    fn interpret_evaluation_for_player(&self, evaln: &ShiftStateEval, player: &u8) -> i64 {
        if *player == evaln.player { evaln.score } else { -evaln.score }
    }

    fn evaluate_existing_state(
        &self,
        state: &ShiftGame,
        _evaln: &ShiftStateEval,
        _: SearchHandle<ShiftConfig>,
    ) -> ShiftStateEval {
        let player = state.current;
        ShiftStateEval { score: state.evaluate_for(player), player }
    }
}

#[derive(Default)]
struct ShiftConfig;

impl MCTS for ShiftConfig {
    type State = ShiftGame;
    type Eval = ShiftEval;
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
pub struct ShiftWasm {
    manager: MCTSManager<ShiftConfig>,
    cols: usize,
    rows: usize,
    k: usize,
    num_players: usize,
    pieces_per_player: u8,
}

impl Default for ShiftWasm {
    fn default() -> Self {
        Self::create(3, 3, 3, 2, 3)
    }
}

#[wasm_bindgen]
impl ShiftWasm {
    fn create(cols: usize, rows: usize, k: usize, num_players: usize, pieces: u8) -> Self {
        let cols = cols.clamp(2, MAX_COLS);
        let rows = rows.clamp(2, MAX_ROWS);
        let k = k.clamp(2, cols.max(rows));
        let num_players = num_players.clamp(2, MAX_PLAYERS);
        let max_pieces = (cols * rows / num_players) as u8;
        let pieces = pieces.clamp(1, max_pieces.max(1));
        Self {
            manager: MCTSManager::new(
                ShiftGame::new(cols, rows, k, num_players, pieces),
                ShiftConfig,
                ShiftEval,
                UCTPolicy::new(1.4),
                (),
            ),
            cols,
            rows,
            k,
            num_players,
            pieces_per_player: pieces,
        }
    }

    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32, k: u32, num_players: u32, pieces: u32) -> Self {
        Self::create(cols as usize, rows as usize, k as usize, num_players as usize, pieces as u8)
    }

    pub fn cols(&self) -> u32 { self.cols as u32 }
    pub fn rows(&self) -> u32 { self.rows as u32 }
    pub fn win_length(&self) -> u32 { self.k as u32 }
    pub fn num_players(&self) -> u32 { self.num_players as u32 }
    pub fn pieces_per_player(&self) -> u32 { self.pieces_per_player as u32 }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        let stats = types::build_stats(&self.manager, |_| None);
        serde_wasm_bindgen::to_value(&stats).unwrap()
    }

    pub fn get_tree(&self, max_depth: u32) -> JsValue {
        let tree = types::export_tree::<ShiftConfig>(
            self.manager.tree().root_node(), max_depth, &|_| None,
        );
        serde_wasm_bindgen::to_value(&tree).unwrap()
    }

    pub fn get_board(&self) -> String {
        self.manager.tree().root_state().board_string()
    }

    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current as u32
    }

    pub fn in_placement_phase(&self) -> bool {
        self.manager.tree().root_state().in_placement_phase()
    }

    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().winner().is_some()
    }

    pub fn result(&self) -> String {
        if let Some(w) = self.manager.tree().root_state().winner() {
            format!("{}", w + 1)
        } else {
            String::new()
        }
    }

    pub fn root_proven_value(&self) -> String {
        format!("{:?}", self.manager.root_proven_value())
    }

    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| m.encode())
    }

    pub fn apply_move(&mut self, mov: &str) -> bool {
        if let Some(m) = ShiftMove::decode(mov) {
            let mut state = self.manager.tree().root_state().clone();
            let legal = state.available_moves();
            if !legal.contains(&m) {
                return false;
            }
            state.make_move(&m);
            self.manager = MCTSManager::new(
                state, ShiftConfig, ShiftEval, UCTPolicy::new(1.4), (),
            );
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(
            ShiftGame::new(self.cols, self.rows, self.k, self.num_players, self.pieces_per_player),
            ShiftConfig, ShiftEval, UCTPolicy::new(1.4), (),
        );
    }
}
