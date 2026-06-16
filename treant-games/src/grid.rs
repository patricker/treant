//! Shared m,n,k grid engine backing Connect Four (gravity) and Tic-Tac-Toe.
use treant::tree_policy::UCTPolicy;
use treant::*;

/// Maximum board dimension (columns or rows) supported.
pub const MAX_DIM: usize = 10;
/// Maximum number of players supported.
pub const MAX_PLAYERS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Player(u8), // 0-indexed
}

/// Static parameters for a grid game.
#[derive(Clone, Copy, Debug)]
pub struct GridConfig {
    pub cols: usize,
    pub rows: usize,
    pub k: usize,
    pub num_players: usize,
    /// `true` = Connect Four (drop into a column); `false` = free placement.
    pub gravity: bool,
    /// `true` adds Connect Four's center-column heuristic bonus.
    pub center_column_bonus: bool,
}

/// A move. For gravity games it is a **column index**; for placement games it
/// is a **cell index** (`row * cols + col`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridMove(pub u8);

impl std::fmt::Display for GridMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An m,n,k grid game state. Board is stored row-major with **row 0 = top**;
/// gravity drops pieces to the largest empty row index (the bottom).
#[derive(Clone, Debug)]
pub struct GridGame {
    board: Vec<Cell>,
    cfg: GridConfig,
    current: u8,
}

impl GridGame {
    pub fn new(cfg: GridConfig) -> Self {
        Self {
            board: vec![Cell::Empty; cfg.cols * cfg.rows],
            cfg,
            current: 0,
        }
    }

    #[inline]
    fn idx(&self, row: usize, col: usize) -> usize {
        row * self.cfg.cols + col
    }

    /// 0-indexed player at `(row, col)` (row 0 = top), or `None` if empty.
    pub fn cell(&self, row: usize, col: usize) -> Option<u8> {
        match self.board[self.idx(row, col)] {
            Cell::Empty => None,
            Cell::Player(p) => Some(p),
        }
    }

    pub fn config(&self) -> &GridConfig {
        &self.cfg
    }

    pub fn current(&self) -> u8 {
        self.current
    }

    /// Lowest empty row (largest index) in a column, for gravity games.
    fn drop_row(&self, col: usize) -> Option<usize> {
        (0..self.cfg.rows)
            .rev()
            .find(|&row| self.board[self.idx(row, col)] == Cell::Empty)
    }

    /// The player who has a k-in-a-row, if any.
    pub fn winner(&self) -> Option<u8> {
        let (rows, cols, k) = (self.cfg.rows, self.cfg.cols, self.cfg.k);
        let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
        for r in 0..rows {
            for c in 0..cols {
                if let Cell::Player(p) = self.board[self.idx(r, c)] {
                    for &(dr, dc) in &dirs {
                        let mut count = 1usize;
                        for step in 1..k {
                            let nr = r as i32 + dr * step as i32;
                            let nc = c as i32 + dc * step as i32;
                            if nr < 0 || nr >= rows as i32 || nc < 0 || nc >= cols as i32 {
                                break;
                            }
                            if self.board[self.idx(nr as usize, nc as usize)] == Cell::Player(p) {
                                count += 1;
                            } else {
                                break;
                            }
                        }
                        if count >= k {
                            return Some(p);
                        }
                    }
                }
            }
        }
        None
    }

    /// Every cell is occupied.
    pub fn is_full(&self) -> bool {
        self.board.iter().all(|&c| c != Cell::Empty)
    }

    /// Heuristic value of the position from `player`'s perspective.
    fn evaluate_for(&self, player: u8) -> i64 {
        let (rows, cols, k) = (self.cfg.rows, self.cfg.cols, self.cfg.k);
        let my = Cell::Player(player);
        let mut score: i64 = 0;

        if self.cfg.center_column_bonus {
            let center = cols / 2;
            for r in 0..rows {
                if self.board[self.idx(r, center)] == my {
                    score += 3;
                }
            }
        }

        let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
        for r in 0..rows {
            for c in 0..cols {
                for &(dr, dc) in &dirs {
                    let end_r = r as i32 + dr * (k as i32 - 1);
                    let end_c = c as i32 + dc * (k as i32 - 1);
                    if end_r < 0 || end_r >= rows as i32 || end_c < 0 || end_c >= cols as i32 {
                        continue;
                    }
                    let mut mine = 0usize;
                    let mut empty = 0usize;
                    let mut theirs = 0usize;
                    for step in 0..k {
                        let rr = (r as i32 + dr * step as i32) as usize;
                        let cc = (c as i32 + dc * step as i32) as usize;
                        match self.board[self.idx(rr, cc)] {
                            cell if cell == my => mine += 1,
                            Cell::Empty => empty += 1,
                            _ => theirs += 1,
                        }
                    }
                    if mine == k {
                        score += 1000;
                    } else if theirs == k {
                        score -= 1000;
                    } else if mine == k - 1 && empty == 1 {
                        score += 50;
                    } else if theirs == k - 1 && empty == 1 {
                        score -= 80;
                    } else if mine >= 2 && theirs == 0 {
                        score += 5;
                    }
                }
            }
        }
        score
    }
}

impl GameState for GridGame {
    type Move = GridMove;
    type Player = u8;
    type MoveList = Vec<GridMove>;

    fn current_player(&self) -> u8 {
        self.current
    }

    fn available_moves(&self) -> Vec<GridMove> {
        if self.winner().is_some() {
            return vec![];
        }
        if self.cfg.gravity {
            (0..self.cfg.cols as u8)
                .filter(|&col| self.drop_row(col as usize).is_some())
                .map(GridMove)
                .collect()
        } else {
            (0..(self.cfg.cols * self.cfg.rows))
                .filter(|&i| self.board[i] == Cell::Empty)
                .map(|i| GridMove(i as u8))
                .collect()
        }
    }

    fn make_move(&mut self, mov: &GridMove) {
        if self.cfg.gravity {
            let col = mov.0 as usize;
            if let Some(row) = self.drop_row(col) {
                let i = self.idx(row, col);
                self.board[i] = Cell::Player(self.current);
            }
        } else {
            self.board[mov.0 as usize] = Cell::Player(self.current);
        }
        self.current = (self.current + 1) % self.cfg.num_players as u8;
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.winner().is_some() {
            Some(ProvenValue::Loss) // winner just moved; current player lost
        } else if self.is_full() {
            Some(ProvenValue::Draw)
        } else {
            None
        }
    }
}

/// Evaluator: window-based heuristic, negamax-interpreted per player.
pub struct GridEval;

#[derive(Clone, Debug)]
pub struct GridStateEval {
    score: i64,
    player: u8,
}

impl Evaluator<GridMcts> for GridEval {
    type StateEvaluation = GridStateEval;

    fn evaluate_new_state(
        &self,
        state: &GridGame,
        moves: &Vec<GridMove>,
        _: Option<SearchHandle<GridMcts>>,
    ) -> (Vec<()>, GridStateEval) {
        let player = state.current;
        (
            vec![(); moves.len()],
            GridStateEval {
                score: state.evaluate_for(player),
                player,
            },
        )
    }

    fn interpret_evaluation_for_player(&self, evaln: &GridStateEval, player: &u8) -> i64 {
        if *player == evaln.player {
            evaln.score
        } else {
            -evaln.score
        }
    }

    fn evaluate_existing_state(
        &self,
        state: &GridGame,
        _evaln: &GridStateEval,
        _: SearchHandle<GridMcts>,
    ) -> GridStateEval {
        let player = state.current;
        GridStateEval {
            score: state.evaluate_for(player),
            player,
        }
    }
}

/// MCTS spec for grid games. `solver` toggles MCTS-Solver (on for Tic-Tac-Toe,
/// off for Connect Four).
pub struct GridMcts {
    pub solver: bool,
}

impl MCTS for GridMcts {
    type State = GridGame;
    type Eval = GridEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();

    fn solver_enabled(&self) -> bool {
        self.solver
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cf(cols: usize, rows: usize, k: usize, players: usize) -> GridConfig {
        GridConfig {
            cols,
            rows,
            k,
            num_players: players,
            gravity: true,
            center_column_bonus: true,
        }
    }
    fn ttt(cols: usize, rows: usize, k: usize, players: usize) -> GridConfig {
        GridConfig {
            cols,
            rows,
            k,
            num_players: players,
            gravity: false,
            center_column_bonus: false,
        }
    }

    // place at (row,col) for player p directly (test helper, bypasses turn order)
    fn put(g: &mut GridGame, row: usize, col: usize, p: u8) {
        let i = g.idx(row, col);
        g.board[i] = Cell::Player(p);
    }

    #[test]
    fn detects_horizontal_win() {
        let mut g = GridGame::new(ttt(5, 5, 3, 2));
        put(&mut g, 2, 0, 0);
        put(&mut g, 2, 1, 0);
        put(&mut g, 2, 2, 0);
        assert_eq!(g.winner(), Some(0));
    }

    #[test]
    fn detects_vertical_win() {
        let mut g = GridGame::new(ttt(5, 5, 3, 2));
        put(&mut g, 0, 1, 1);
        put(&mut g, 1, 1, 1);
        put(&mut g, 2, 1, 1);
        assert_eq!(g.winner(), Some(1));
    }

    #[test]
    fn detects_both_diagonals() {
        let mut g = GridGame::new(ttt(4, 4, 3, 2));
        put(&mut g, 0, 0, 0);
        put(&mut g, 1, 1, 0);
        put(&mut g, 2, 2, 0);
        assert_eq!(g.winner(), Some(0));
        let mut h = GridGame::new(ttt(4, 4, 3, 2));
        put(&mut h, 0, 2, 1);
        put(&mut h, 1, 1, 1);
        put(&mut h, 2, 0, 1);
        assert_eq!(h.winner(), Some(1));
    }

    #[test]
    fn no_false_win_below_k() {
        let mut g = GridGame::new(ttt(5, 5, 4, 2));
        put(&mut g, 0, 0, 0);
        put(&mut g, 0, 1, 0);
        put(&mut g, 0, 2, 0);
        assert_eq!(g.winner(), None);
    }

    #[test]
    fn full_board_detected() {
        let mut g = GridGame::new(ttt(2, 2, 3, 2));
        for r in 0..2 {
            for c in 0..2 {
                put(&mut g, r, c, 0);
            }
        }
        assert!(g.is_full());
        assert!(!GridGame::new(ttt(2, 2, 3, 2)).is_full());
    }

    #[test]
    fn cell_accessor_origin_is_top_left() {
        let mut g = GridGame::new(ttt(3, 3, 3, 2));
        put(&mut g, 0, 0, 2);
        assert_eq!(g.cell(0, 0), Some(2));
        assert_eq!(g.cell(1, 1), None);
    }

    #[test]
    fn gravity_moves_are_nonfull_columns() {
        let g = GridGame::new(cf(7, 6, 4, 2));
        let moves: Vec<u8> = g.available_moves().iter().map(|m| m.0).collect();
        assert_eq!(moves, vec![0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn gravity_drops_to_bottom() {
        let mut g = GridGame::new(cf(7, 6, 4, 2));
        g.make_move(&GridMove(3)); // player 0 into column 3
        // bottom row is rows-1 = 5
        assert_eq!(g.cell(5, 3), Some(0));
        assert_eq!(g.cell(4, 3), None);
        assert_eq!(g.current(), 1);
    }

    #[test]
    fn placement_moves_are_empty_cells_and_place_exactly() {
        let mut g = GridGame::new(ttt(3, 3, 3, 2));
        assert_eq!(g.available_moves().len(), 9);
        g.make_move(&GridMove(4)); // center cell of 3x3
        assert_eq!(g.cell(1, 1), Some(0));
        assert_eq!(g.available_moves().len(), 8);
    }

    #[test]
    fn no_moves_after_win() {
        let mut g = GridGame::new(ttt(5, 5, 3, 2));
        put(&mut g, 2, 0, 0);
        put(&mut g, 2, 1, 0);
        put(&mut g, 2, 2, 0);
        assert!(g.available_moves().is_empty());
    }

    #[test]
    fn terminal_value_loss_for_mover_then_draw() {
        let mut win = GridGame::new(ttt(5, 5, 3, 2));
        put(&mut win, 2, 0, 0);
        put(&mut win, 2, 1, 0);
        put(&mut win, 2, 2, 0);
        assert_eq!(win.terminal_value(), Some(ProvenValue::Loss));

        let mut full = GridGame::new(ttt(2, 2, 3, 2));
        for r in 0..2 {
            for c in 0..2 {
                put(&mut full, r, c, 0);
            }
        }
        assert_eq!(full.terminal_value(), Some(ProvenValue::Draw));

        assert_eq!(GridGame::new(ttt(3, 3, 3, 2)).terminal_value(), None);
    }

    #[test]
    fn turn_order_wraps_for_three_players() {
        let mut g = GridGame::new(ttt(5, 5, 4, 3));
        g.make_move(&GridMove(0));
        g.make_move(&GridMove(1));
        g.make_move(&GridMove(2));
        assert_eq!(g.current(), 0); // wrapped 0->1->2->0
    }

    #[test]
    fn center_bonus_only_applies_with_gravity_config() {
        // single piece in the center column, k=4 so no window branch fires
        let mut g = GridGame::new(cf(7, 6, 4, 2));
        put(&mut g, 5, 3, 0); // bottom-center
        assert_eq!(g.evaluate_for(0), 3); // center bonus only
        let mut t = GridGame::new(ttt(7, 6, 4, 2));
        put(&mut t, 5, 3, 0);
        assert_eq!(t.evaluate_for(0), 0); // no center bonus
    }

    #[test]
    fn heuristic_rewards_won_window() {
        let mut g = GridGame::new(ttt(5, 1, 3, 2));
        put(&mut g, 0, 0, 0);
        put(&mut g, 0, 1, 0);
        put(&mut g, 0, 2, 0);
        // three-in-a-row window = +1000; player 1 sees -1000
        assert!(g.evaluate_for(0) >= 1000);
        assert!(g.evaluate_for(1) <= -1000);
    }

    #[test]
    fn search_runs_and_picks_a_legal_move() {
        let mut mgr = MCTSManager::new(
            GridGame::new(cf(7, 6, 4, 2)),
            GridMcts { solver: false },
            GridEval,
            UCTPolicy::new(1.4),
            (),
        );
        mgr.playout_n(200);
        let m = mgr.best_move().expect("a move");
        assert!((m.0 as usize) < 7);
    }

    #[test]
    fn solver_proves_trivial_ttt_win() {
        // 3x1 board, player 0 to move can complete 3-in-a-row immediately
        let mut g = GridGame::new(ttt(3, 1, 3, 2));
        let idx0 = g.idx(0, 0);
        g.board[idx0] = Cell::Player(0);
        let idx1 = g.idx(0, 1);
        g.board[idx1] = Cell::Player(0);
        let mut mgr = MCTSManager::new(g, GridMcts { solver: true }, GridEval, UCTPolicy::new(1.4), ());
        mgr.playout_n(50);
        assert_eq!(mgr.best_move(), Some(GridMove(2))); // the winning cell
    }
}
