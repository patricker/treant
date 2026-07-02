use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

// --- Quadline: 2 players, N×N board, drop four then slide ---
// Phase 1 (drop): players alternately drop their 4 pieces onto empty cells.
// Phase 2 (move): pick one of your pieces and slide it to an ADJACENT empty
// cell (8 directions). A win in either phase: your 4 pieces form a straight
// line of four (horizontal/vertical, plus diagonal when enabled) OR a 2×2
// square. Sliding can cycle forever, so a ply cap declares a draw.

const MAX_DIM: usize = 7;
const MAX_CELLS: usize = MAX_DIM * MAX_DIM;
const PIECES_PER_PLAYER: u8 = 4;
const WIN_LEN: usize = 4;
const PLY_CAP: u16 = 120;

const PLAYER_SYMBOLS: [char; 2] = ['X', 'O'];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    Empty,
    Player(u8),
}

/// Move: either Drop(cell) or Slide(from, to).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct QuadMove {
    from: Option<u8>,
    to: u8,
}

impl std::fmt::Display for QuadMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.from {
            None => write!(f, "d{}", self.to),
            Some(fr) => write!(f, "{},{}", fr, self.to),
        }
    }
}

impl QuadMove {
    fn encode(&self) -> String {
        // Drops read "d<cell>"; slides read "<from>-<to>".
        match self.from {
            None => format!("d{}", self.to),
            Some(fr) => format!("{}-{}", fr, self.to),
        }
    }

    fn decode(s: &str) -> Option<Self> {
        if let Some(rest) = s.strip_prefix('d') {
            let to: u8 = rest.parse().ok()?;
            Some(QuadMove { from: None, to })
        } else if let Some((a, b)) = s.split_once('-') {
            let from: u8 = a.parse().ok()?;
            let to: u8 = b.parse().ok()?;
            Some(QuadMove {
                from: Some(from),
                to,
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
struct QuadlineGame {
    board: [Cell; MAX_CELLS],
    current: u8,
    size: usize,
    diagonals: bool,
    pieces_placed: [u8; 2],
    ply: u16,
}

impl QuadlineGame {
    fn new(size: usize, diagonals: bool) -> Self {
        Self {
            board: [Cell::Empty; MAX_CELLS],
            current: 0,
            size,
            diagonals,
            pieces_placed: [0; 2],
            ply: 0,
        }
    }

    fn cell_count(&self) -> usize {
        self.size * self.size
    }

    fn in_placement_phase(&self) -> bool {
        self.pieces_placed[self.current as usize] < PIECES_PER_PLAYER
    }

    /// The eight neighbour cells of `idx` (as flat indices) within the board.
    fn neighbours(&self, idx: usize) -> Vec<u8> {
        let r = (idx / self.size) as i32;
        let c = (idx % self.size) as i32;
        let mut out = Vec::with_capacity(8);
        for dr in -1..=1 {
            for dc in -1..=1 {
                if dr == 0 && dc == 0 {
                    continue;
                }
                let nr = r + dr;
                let nc = c + dc;
                if nr >= 0 && nr < self.size as i32 && nc >= 0 && nc < self.size as i32 {
                    out.push((nr as usize * self.size + nc as usize) as u8);
                }
            }
        }
        out
    }

    /// Every winning shape as a set of four flat cell indices: horizontal and
    /// vertical 4-lines, diagonal 4-lines (only when enabled), and 2×2 squares.
    fn shapes(&self) -> Vec<[usize; 4]> {
        let s = self.size;
        let mut out = Vec::new();
        for r in 0..s {
            for c in 0..s {
                // Horizontal line of four.
                if c + WIN_LEN <= s {
                    out.push([
                        r * s + c,
                        r * s + c + 1,
                        r * s + c + 2,
                        r * s + c + 3,
                    ]);
                }
                // Vertical line of four.
                if r + WIN_LEN <= s {
                    out.push([
                        r * s + c,
                        (r + 1) * s + c,
                        (r + 2) * s + c,
                        (r + 3) * s + c,
                    ]);
                }
                if self.diagonals {
                    // Diagonal ↘.
                    if r + WIN_LEN <= s && c + WIN_LEN <= s {
                        out.push([
                            r * s + c,
                            (r + 1) * s + c + 1,
                            (r + 2) * s + c + 2,
                            (r + 3) * s + c + 3,
                        ]);
                    }
                    // Diagonal ↙.
                    if r + WIN_LEN <= s && c >= WIN_LEN - 1 {
                        out.push([
                            r * s + c,
                            (r + 1) * s + c - 1,
                            (r + 2) * s + c - 2,
                            (r + 3) * s + c - 3,
                        ]);
                    }
                }
                // 2×2 square.
                if r + 1 < s && c + 1 < s {
                    out.push([r * s + c, r * s + c + 1, (r + 1) * s + c, (r + 1) * s + c + 1]);
                }
            }
        }
        out
    }

    fn winner(&self) -> Option<u8> {
        for shape in self.shapes() {
            if let Cell::Player(p) = self.board[shape[0]] {
                if shape.iter().all(|&i| self.board[i] == Cell::Player(p)) {
                    return Some(p);
                }
            }
        }
        None
    }

    /// Legal moves ignoring terminality (the terminal guard lives in
    /// `available_moves`/`terminal_value`). Empty only if a mover is stuck.
    fn gen_moves(&self) -> Vec<QuadMove> {
        let n = self.cell_count();
        let mut moves = Vec::new();
        if self.in_placement_phase() {
            for i in 0..n {
                if self.board[i] == Cell::Empty {
                    moves.push(QuadMove {
                        from: None,
                        to: i as u8,
                    });
                }
            }
        } else {
            let me = Cell::Player(self.current);
            for from in 0..n {
                if self.board[from] != me {
                    continue;
                }
                for to in self.neighbours(from) {
                    if self.board[to as usize] == Cell::Empty {
                        moves.push(QuadMove {
                            from: Some(from as u8),
                            to,
                        });
                    }
                }
            }
        }
        moves
    }

    fn board_string(&self) -> String {
        (0..self.cell_count())
            .map(|i| match self.board[i] {
                Cell::Empty => ' ',
                Cell::Player(p) => PLAYER_SYMBOLS.get(p as usize).copied().unwrap_or('?'),
            })
            .collect()
    }

    /// Heuristic from `player`'s perspective: reward shapes that hold only your
    /// pieces (weighted by how many), penalise shapes the opponent owns alone.
    fn evaluate_for(&self, player: u8) -> i64 {
        let mut score: i64 = 0;
        for shape in self.shapes() {
            let mut mine = 0i64;
            let mut theirs = 0i64;
            for &i in &shape {
                match self.board[i] {
                    Cell::Player(p) if p == player => mine += 1,
                    Cell::Player(_) => theirs += 1,
                    Cell::Empty => {}
                }
            }
            if theirs == 0 {
                score += mine * mine;
            } else if mine == 0 {
                score -= theirs * theirs;
            }
        }
        score
    }
}

impl GameState for QuadlineGame {
    type Move = QuadMove;
    type Player = u8;
    type MoveList = Vec<QuadMove>;

    fn current_player(&self) -> u8 {
        self.current
    }

    fn available_moves(&self) -> Vec<QuadMove> {
        if self.winner().is_some() || self.ply >= PLY_CAP {
            return vec![];
        }
        self.gen_moves()
    }

    fn make_move(&mut self, mov: &QuadMove) {
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
        self.ply += 1;
        self.current = 1 - self.current;
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.winner().is_some() {
            // The mover just completed the shape; the player to move has lost.
            Some(ProvenValue::Loss)
        } else if self.ply >= PLY_CAP {
            Some(ProvenValue::Draw)
        } else if self.gen_moves().is_empty() {
            // No line yet and nobody can move (rare): a stalemate draw.
            Some(ProvenValue::Draw)
        } else {
            None
        }
    }
}

// --- Evaluator ---

struct QuadlineEval;

#[derive(Clone, Debug)]
struct QuadlineStateEval {
    score: i64,
    player: u8,
}

impl Evaluator<QuadlineConfig> for QuadlineEval {
    type StateEvaluation = QuadlineStateEval;

    fn evaluate_new_state(
        &self,
        state: &QuadlineGame,
        moves: &Vec<QuadMove>,
        _: Option<SearchHandle<QuadlineConfig>>,
    ) -> (Vec<()>, QuadlineStateEval) {
        let player = state.current;
        (
            vec![(); moves.len()],
            QuadlineStateEval {
                score: state.evaluate_for(player),
                player,
            },
        )
    }

    fn interpret_evaluation_for_player(&self, evaln: &QuadlineStateEval, player: &u8) -> i64 {
        if *player == evaln.player {
            evaln.score
        } else {
            -evaln.score
        }
    }

    fn evaluate_existing_state(
        &self,
        state: &QuadlineGame,
        _evaln: &QuadlineStateEval,
        _: SearchHandle<QuadlineConfig>,
    ) -> QuadlineStateEval {
        let player = state.current;
        QuadlineStateEval {
            score: state.evaluate_for(player),
            player,
        }
    }
}

#[derive(Default)]
struct QuadlineConfig;

impl MCTS for QuadlineConfig {
    type State = QuadlineGame;
    type Eval = QuadlineEval;
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
pub struct QuadlineWasm {
    manager: MCTSManager<QuadlineConfig>,
    size: usize,
    diagonals: bool,
}

impl Default for QuadlineWasm {
    fn default() -> Self {
        Self::create(5, true)
    }
}

#[wasm_bindgen]
impl QuadlineWasm {
    fn create(size: usize, diagonals: bool) -> Self {
        let size = size.clamp(WIN_LEN, MAX_DIM);
        Self {
            manager: MCTSManager::new(
                QuadlineGame::new(size, diagonals),
                QuadlineConfig,
                QuadlineEval,
                UCTPolicy::new(1.4),
                (),
            ),
            size,
            diagonals,
        }
    }

    #[wasm_bindgen(constructor)]
    pub fn new(size: u32, diagonals: u32) -> Self {
        Self::create(size as usize, diagonals != 0)
    }

    pub fn size(&self) -> u32 {
        self.size as u32
    }

    pub fn pieces_per_player(&self) -> u32 {
        PIECES_PER_PLAYER as u32
    }

    pub fn diagonals(&self) -> bool {
        self.diagonals
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        let stats = types::build_stats(&self.manager, |_| None);
        serde_wasm_bindgen::to_value(&stats).unwrap_or(JsValue::NULL)
    }

    pub fn get_tree(&self, max_depth: u32) -> JsValue {
        let tree = types::export_tree::<QuadlineConfig>(
            self.manager.tree().root_node(),
            max_depth,
            &|_| None,
        );
        serde_wasm_bindgen::to_value(&tree).unwrap_or(JsValue::NULL)
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
        self.manager.tree().root_state().terminal_value().is_some()
    }

    pub fn result(&self) -> String {
        let s = self.manager.tree().root_state();
        if let Some(w) = s.winner() {
            format!("{}", w + 1)
        } else if s.terminal_value().is_some() {
            "Draw".into()
        } else {
            String::new()
        }
    }

    pub fn legal_moves(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .available_moves()
            .iter()
            .map(|m| m.encode())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn root_proven_value(&self) -> String {
        format!("{:?}", self.manager.root_proven_value())
    }

    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| m.encode())
    }

    pub fn weak_move(
        &mut self,
        playouts: u32,
        top_k: usize,
        temp: f64,
        seed: u32,
    ) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }

    pub fn apply_move(&mut self, mov: &str) -> bool {
        if let Some(m) = QuadMove::decode(mov) {
            let mut state = self.manager.tree().root_state().clone();
            let legal = state.available_moves();
            if !legal.contains(&m) {
                return false;
            }
            state.make_move(&m);
            self.manager =
                MCTSManager::new(state, QuadlineConfig, QuadlineEval, UCTPolicy::new(1.4), ());
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(
            QuadlineGame::new(self.size, self.diagonals),
            QuadlineConfig,
            QuadlineEval,
            UCTPolicy::new(1.4),
            (),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive the game past the drop phase into a chosen position by applying a
    /// list of encoded moves, asserting each is legal.
    fn play(g: &mut QuadlineWasm, moves: &[&str]) {
        for m in moves {
            assert!(g.apply_move(m), "move {m} should be legal");
        }
    }

    #[test]
    fn drop_phase_movegen_count() {
        // 5×5, first drop: every one of 25 empty cells is a legal drop.
        let g = QuadlineWasm::new(5, 1);
        assert_eq!(g.legal_moves().split(',').count(), 25);
        assert!(g.in_placement_phase());
    }

    #[test]
    fn drop_phase_lasts_eight_plies() {
        let mut g = QuadlineWasm::new(5, 1);
        // 8 drops (4 each) then we should be in the move phase.
        play(&mut g, &["d0", "d5", "d1", "d6", "d2", "d7", "d20", "d24"]);
        assert!(!g.in_placement_phase(), "after 8 drops the move phase begins");
        // Moves now look like "<from>-<to>" and only slide to adjacent empties.
        let moves = g.legal_moves();
        assert!(!moves.is_empty());
        assert!(
            moves.split(',').all(|m| m.contains('-')),
            "move-phase moves slide"
        );
    }

    #[test]
    fn horizontal_line_of_four_wins() {
        // Red (X) drops a row of four; Yellow (O) drops elsewhere. Red's 4th
        // drop completes the line and wins during the DROP phase.
        let mut g = QuadlineWasm::new(5, 1);
        // Red: 0,1,2,3 (top row). Yellow: 20,21,22 (bottom, harmless).
        play(&mut g, &["d0", "d20", "d1", "d21", "d2", "d22"]);
        assert!(!g.is_terminal());
        assert!(g.apply_move("d3"), "Red completes the four-line");
        assert!(g.is_terminal(), "a completed line is terminal");
        assert_eq!(g.result(), "1", "Red (seat 1) wins");
    }

    #[test]
    fn two_by_two_square_wins() {
        // Red forms the 2×2 square {0,1,5,6} on a 5×5 board.
        let mut g = QuadlineWasm::new(5, 1);
        play(&mut g, &["d0", "d20", "d1", "d21", "d5", "d22"]);
        assert!(!g.is_terminal());
        assert!(g.apply_move("d6"), "Red completes the 2x2 square");
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1");
    }

    #[test]
    fn diagonal_counts_only_when_enabled() {
        // The cells 0,6,12,18 are a ↘ diagonal on a 5×5 board.
        let diag = ["d0", "d20", "d6", "d21", "d12", "d22", "d18"];

        let mut on = QuadlineWasm::new(5, 1);
        play(&mut on, &diag[..6]);
        assert!(on.apply_move("d18"));
        assert!(on.is_terminal(), "diagonal wins when diagonals are on");
        assert_eq!(on.result(), "1");

        let mut off = QuadlineWasm::new(5, 0);
        play(&mut off, &diag[..6]);
        assert!(off.apply_move("d18"));
        assert!(
            !off.is_terminal(),
            "the same diagonal is NOT a win when diagonals are off"
        );
    }

    #[test]
    fn terminal_verdict_is_loss_for_player_to_move() {
        // After Red completes a line it is Yellow (seat 2) to move, and the
        // player to move sees a Loss (negamax convention).
        let mut g = QuadlineWasm::new(5, 1);
        play(&mut g, &["d0", "d20", "d1", "d21", "d2", "d22", "d3"]);
        let s = g.manager.tree().root_state();
        assert_eq!(s.current, 1, "Yellow is to move");
        assert_eq!(s.terminal_value(), Some(ProvenValue::Loss));
    }

    #[test]
    fn ply_cap_forces_a_draw() {
        // Build a benign non-winning position, then shuffle a lone Red piece
        // back and forth (with Yellow doing likewise) until the ply cap trips.
        let mut g = QuadlineGame::new(5, false);
        // Red pieces well apart; Yellow pieces well apart — no shape possible.
        // Red: 0, 2, 4, 10 ; Yellow: 14, 20, 22, 24 (drops, alternating).
        for m in [
            QuadMove { from: None, to: 0 },
            QuadMove { from: None, to: 24 },
            QuadMove { from: None, to: 2 },
            QuadMove { from: None, to: 22 },
            QuadMove { from: None, to: 4 },
            QuadMove { from: None, to: 20 },
            QuadMove { from: None, to: 10 },
            QuadMove { from: None, to: 14 },
        ] {
            g.make_move(&m);
        }
        assert!(g.terminal_value().is_none());
        // Slide Red's piece at 10 between 10 and 11 while Yellow slides 14<->13.
        let mut red_at = 10u8;
        let mut yel_at = 14u8;
        while g.terminal_value().is_none() {
            let (from, to) = if g.current == 0 {
                let t = if red_at == 10 { 11 } else { 10 };
                let mv = (red_at, t);
                red_at = t;
                mv
            } else {
                let t = if yel_at == 14 { 13 } else { 14 };
                let mv = (yel_at, t);
                yel_at = t;
                mv
            };
            g.make_move(&QuadMove {
                from: Some(from),
                to,
            });
        }
        assert_eq!(g.terminal_value(), Some(ProvenValue::Draw));
        assert!(g.ply >= PLY_CAP);
    }

    #[test]
    fn ai_plays_a_legal_move() {
        let mut g = QuadlineWasm::new(5, 1);
        g.playout_n(200);
        let mv = g.best_move().expect("AI should find a move");
        assert!(g.apply_move(&mv), "the AI's move must be legal");
    }
}
