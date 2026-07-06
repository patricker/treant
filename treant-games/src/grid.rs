//! Shared m,n,k grid engine backing Connect Four (gravity) and Tic-Tac-Toe.
use treant::tree_policy::UCTPolicy;
use treant::*;

/// Pop-move encoding offset. A [`GridMove`] whose value is `>= POP_OFFSET`
/// encodes a *pop-out* move on column `value - POP_OFFSET` (remove that column's
/// bottom disc; the stack above it falls one row). Regular gravity/placement
/// moves are `< POP_OFFSET`. Columns are clamped to ≤ 12 (`MAX_DIM` in the wasm
/// layer), so column indices `0..=11` never collide with the pop range
/// `128..=139`. Pops are only generated when [`GridConfig::allow_pop`] is set.
pub const POP_OFFSET: u8 = 128;

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
    /// `true` = Cylinder: the board is a tube — the `winner()`/`winning_line()`
    /// scan wraps columns modulo `cols` (rows never wrap). `false` = flat board.
    pub wrap_cols: bool,
    /// `true` = Pop Out: a player may remove the bottom disc of a column that is
    /// their own (the stack above falls one row). Adds pop moves (see
    /// [`POP_OFFSET`]) and makes `terminal_value` seat-aware, because a pop can
    /// complete the *opponent's* line.
    pub allow_pop: bool,
}

/// A move. For gravity games it is a **column index**; for placement games it
/// is a **cell index** (`row * cols + col`). When [`GridConfig::allow_pop`] is
/// set, a value `>= POP_OFFSET` instead encodes a *pop-out* on column
/// `value - POP_OFFSET` (see [`POP_OFFSET`]).
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

    /// Resolve the neighbour column `step` cells along `dc` from column `c`.
    /// On a cylinder (`wrap_cols`) columns wrap modulo `cols`; otherwise an
    /// off-board column returns `None`. Rows never wrap, so callers bound rows
    /// separately. Returns `None` for a purely-horizontal run (`dr == 0`) that
    /// has already traversed a full lap — a wrapped row must not revisit a cell.
    #[inline]
    fn wrap_col(&self, c: usize, dr: i32, dc: i32, step: i32) -> Option<usize> {
        let cols = self.cfg.cols as i32;
        let raw = c as i32 + dc * step;
        if self.cfg.wrap_cols {
            if dr == 0 && step >= cols {
                return None; // one lap of a row is all the distinct cells there are
            }
            Some(raw.rem_euclid(cols) as usize)
        } else if raw < 0 || raw >= cols {
            None
        } else {
            Some(raw as usize)
        }
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
                            if nr < 0 || nr >= rows as i32 {
                                break;
                            }
                            let nc = match self.wrap_col(c, dr, dc, step as i32) {
                                Some(nc) => nc,
                                None => break,
                            };
                            if self.board[self.idx(nr as usize, nc)] == Cell::Player(p) {
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

    /// The cells forming the first k-in-a-row (row-major, in the 4 canonical
    /// directions), extended to the full maximal run. Cell index = `row*cols+col`.
    /// `None` when there is no line. Used by the UI to highlight the winning row.
    pub fn winning_line(&self) -> Option<Vec<usize>> {
        let (rows, cols, k) = (self.cfg.rows, self.cfg.cols, self.cfg.k);
        let dirs: [(i32, i32); 4] = [(0, 1), (1, 0), (1, 1), (1, -1)];
        for r in 0..rows {
            for c in 0..cols {
                if let Cell::Player(p) = self.board[self.idx(r, c)] {
                    for &(dr, dc) in &dirs {
                        let mut cells = vec![self.idx(r, c)];
                        let mut step = 1i32;
                        loop {
                            let nr = r as i32 + dr * step;
                            if nr < 0 || nr >= rows as i32 {
                                break;
                            }
                            let nc = match self.wrap_col(c, dr, dc, step) {
                                Some(nc) => nc,
                                None => break,
                            };
                            if self.board[self.idx(nr as usize, nc)] == Cell::Player(p) {
                                cells.push(self.idx(nr as usize, nc));
                                step += 1;
                            } else {
                                break;
                            }
                        }
                        if cells.len() >= k {
                            return Some(cells);
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
        let mut moves: Vec<GridMove> = if self.cfg.gravity {
            (0..self.cfg.cols as u8)
                .filter(|&col| self.drop_row(col as usize).is_some())
                .map(GridMove)
                .collect()
        } else {
            (0..(self.cfg.cols * self.cfg.rows))
                .filter(|&i| self.board[i] == Cell::Empty)
                .map(|i| GridMove(i as u8))
                .collect()
        };
        if self.cfg.allow_pop {
            // Pop Out: a player may remove the bottom disc of any column whose
            // bottom disc is their own (POP_OFFSET + column).
            let bottom = self.cfg.rows - 1;
            for col in 0..self.cfg.cols {
                if self.cell(bottom, col) == Some(self.current) {
                    moves.push(GridMove(col as u8 + POP_OFFSET));
                }
            }
        }
        moves
    }

    fn make_move(&mut self, mov: &GridMove) {
        if self.cfg.allow_pop && mov.0 >= POP_OFFSET {
            // Pop the bottom disc of this column: every disc falls one row and
            // the top cell empties. Bottom = rows-1, top = 0.
            let col = (mov.0 - POP_OFFSET) as usize;
            for row in (1..self.cfg.rows).rev() {
                let from = self.idx(row - 1, col);
                let to = self.idx(row, col);
                self.board[to] = self.board[from];
            }
            let top = self.idx(0, col);
            self.board[top] = Cell::Empty;
        } else if self.cfg.gravity {
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
        if let Some(w) = self.winner() {
            if self.cfg.allow_pop {
                // With pops a move can complete EITHER player's line, so the
                // winner is not necessarily the previous mover. Compare seats:
                // the player to move (`self.current`) has won iff the line is
                // theirs, otherwise the previous mover completed it and the
                // player to move has lost.
                //
                // A single pop can complete BOTH players' lines at once (the
                // shifted column lands a disc that closes one line for each
                // seat). We do not adjudicate the tie specially: `winner()`
                // returns the first line in its row-major, 4-direction scan
                // order, and that seat is reported here. This is accepted —
                // such double-completions are vanishingly rare, the outcome is
                // deterministic, and the game still ends sanely (exactly one
                // winner, no panic). A tie-break rule would add complexity for
                // a case players effectively never reach.
                if w == self.current {
                    Some(ProvenValue::Win)
                } else {
                    Some(ProvenValue::Loss)
                }
            } else {
                // Classic drop/placement: a move can only extend the mover's own
                // line, so the winner is always the previous mover and the player
                // to move has lost. Kept as a distinct branch to preserve the
                // exact semantics TicTacToe's solver proofs depend on. (The
                // seat-aware branch above is equivalent here — the winner is
                // never `self.current` — see `no_pop_winner_is_previous_mover`.)
                Some(ProvenValue::Loss)
            }
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
            wrap_cols: false,
            allow_pop: false,
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
            wrap_cols: false,
            allow_pop: false,
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
    fn winning_line_reports_the_run_cells() {
        // Horizontal 3-in-a-row on row 2 of a 5×5: indices 10,11,12.
        let mut g = GridGame::new(ttt(5, 5, 3, 2));
        put(&mut g, 2, 0, 0);
        put(&mut g, 2, 1, 0);
        put(&mut g, 2, 2, 0);
        assert_eq!(g.winning_line(), Some(vec![10, 11, 12]));
        // No line yet → None.
        assert_eq!(GridGame::new(ttt(3, 3, 3, 2)).winning_line(), None);
    }

    #[test]
    fn winning_line_captures_overshoot() {
        // Four in a row when only k=3 is needed → the full run is highlighted.
        let mut g = GridGame::new(ttt(5, 5, 3, 2));
        for c in 0..4 {
            put(&mut g, 0, c, 1);
        }
        assert_eq!(g.winning_line(), Some(vec![0, 1, 2, 3]));
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

    // ---- Cylinder (wrap_cols) ----

    #[test]
    fn cylinder_wraps_a_horizontal_line_across_the_seam() {
        // 12-wide tube, k=4. P0 at bottom-row cols 10,11,0,1 wraps the seam.
        let mut g = GridGame::new(GridConfig { wrap_cols: true, ..cf(12, 6, 4, 2) });
        for c in [10, 11, 0, 1] {
            put(&mut g, 5, c, 0);
        }
        assert_eq!(g.winner(), Some(0));
        // The highlighted line includes the wrapped cells (row 5 = indices 60..71).
        let line = g.winning_line().unwrap();
        assert_eq!(line.len(), 4);
        for c in [10, 11, 0, 1] {
            assert!(line.contains(&(5 * 12 + c)), "line should include col {c}");
        }
    }

    #[test]
    fn flat_board_does_not_wrap_the_seam() {
        // Same discs, but on a FLAT board cols 10,11,0,1 are NOT contiguous.
        let mut g = GridGame::new(cf(12, 6, 4, 2));
        for c in [10, 11, 0, 1] {
            put(&mut g, 5, c, 0);
        }
        assert_eq!(g.winner(), None);
    }

    #[test]
    fn cylinder_short_row_does_not_double_count() {
        // A 3-wide tube with k=4: a full lap is only 3 distinct cells, so even a
        // completely filled row must NOT read as a 4-in-a-row (no revisiting).
        let mut g = GridGame::new(GridConfig { wrap_cols: true, ..cf(3, 3, 4, 2) });
        for c in 0..3 {
            put(&mut g, 2, c, 0);
        }
        assert_eq!(g.winner(), None);
    }

    // ---- Pop Out (allow_pop) ----

    fn pop(cols: usize, rows: usize, k: usize, players: usize) -> GridConfig {
        GridConfig { allow_pop: true, ..cf(cols, rows, k, players) }
    }

    #[test]
    fn pop_shifts_the_whole_column_down_one() {
        // col 0 from bottom: P0(r5), P1(r4), P1(r3). Pop the bottom P0 disc.
        let mut g = GridGame::new(pop(7, 6, 4, 2));
        put(&mut g, 5, 0, 0);
        put(&mut g, 4, 0, 1);
        put(&mut g, 3, 0, 1);
        g.make_move(&GridMove(POP_OFFSET)); // pop column 0
        // Everything fell one row: bottom is now the former r4=P1, then r3=P1.
        assert_eq!(g.cell(5, 0), Some(1));
        assert_eq!(g.cell(4, 0), Some(1));
        assert_eq!(g.cell(3, 0), None);
    }

    #[test]
    fn pop_is_only_offered_for_the_movers_own_bottom_disc() {
        let mut g = GridGame::new(pop(7, 6, 4, 2));
        put(&mut g, 5, 0, 0); // bottom of col0 is P0
        put(&mut g, 5, 1, 1); // bottom of col1 is P1
        // current is P0: only col0 is poppable.
        let pops: Vec<u8> = g
            .available_moves()
            .iter()
            .map(|m| m.0)
            .filter(|&v| v >= POP_OFFSET)
            .collect();
        assert_eq!(pops, vec![POP_OFFSET]); // only column 0
    }

    #[test]
    fn no_pop_winner_is_previous_mover() {
        // Equivalence witness for terminal_value's classic branch: in a no-pop
        // game a completed line always belongs to the PREVIOUS mover, never the
        // player to move, so the seat-aware comparison would return Loss too.
        let mut g = GridGame::new(cf(7, 6, 4, 2));
        // P0 stacks col0 four high (P1 answers col1) → P0 completes a vertical.
        for _ in 0..3 {
            g.make_move(&GridMove(0)); // P0
            g.make_move(&GridMove(1)); // P1
        }
        g.make_move(&GridMove(0)); // P0's 4th disc completes the line
        let w = g.winner().expect("a winner");
        assert_ne!(w, g.current(), "winner is the previous mover, not the player to move");
        assert_eq!(g.terminal_value(), Some(ProvenValue::Loss));
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
