//! Slimetrail — one shared token (the "snail") is walked across a square grid;
//! the cell it leaves turns to permanent slime and can never be entered again.
//! Each player owns one goal corner; the game ends the instant the token lands
//! on a goal (its OWNER wins — no matter who moved it there), or when the player
//! to move is boxed in.
//!
//! SOURCES (rules verified against, encoded below):
//! * Combinatorial Game Theory blog, "Game Description: Slimetrail" (2017),
//!   http://combinatorialgametheory.blogspot.com/2017/07/game-description-slimetrail.html
//!   — "one vertex colored Blue, another colored Red, and a third vertex with a
//!   moveable piece or token which will create the trail of slime"; "players
//!   alternate turns moving the token one space, then marking the previous space
//!   … a third color (usually green). The token can never be moved back to one of
//!   these 'slimed' spaces"; "A player wins when the token is moved onto the space
//!   of their color"; "played on a grid, with adjacencies in all 8 directions".
//! * bodogemu, "Rules: Slimetrail" (playable implementation, snail/vegetable
//!   theme) — "played on a 7x7 board"; "You must move the snail to any adjacent
//!   place which doesn't have a slime (horizontaly, vertically and diagonal)";
//!   "The vegetable on the bottom left is for the first player and on the top
//!   right is for the second one"; "The player wins when the snail is moved over
//!   the correct vegetable OR IF THE OPPONENT IS BLOCKED".
//! * Mancala World wiki, "Slimetrail" — "The player whose goal square is reached
//!   first (it doesn't matter by whom) wins"; goals sit in diagonally-opposite
//!   corners.
//!
//! SOURCED RULE DECISIONS (each with a test below):
//! * ADJACENCY: 8-directional (king moves — orthogonal + diagonal). Both primary
//!   sources agree (CGT "all 8 directions"; bodogemu "horizontaly, vertically and
//!   diagonal"). (A Mancala-World summary mentions an orthogonal-only framing; we
//!   follow the two agreeing primary sources.) → `eight_neighbors` / test
//!   `adjacency_is_eight_directional`.
//! * START: token starts at the board CENTRE (idx(size/2, size/2)). The sources
//!   describe "a third vertex" but do not pin it; centre is the common, symmetric
//!   form (and the form the phase plan specifies). → test `start_is_centre`.
//! * GOALS: diagonally-opposite corners — P0/Red = bottom-left, P1/Gold =
//!   top-right (bodogemu's first/second-player corners). → test `goals_in_opposite_corners`.
//! * WIN BY GOAL: reaching a goal wins for that goal's OWNER regardless of who
//!   moved the token there (Mancala "it doesn't matter by whom"; CGT "onto the
//!   space of their color"). So being forced to push the token onto the ENEMY
//!   goal loses. → tests `reaching_own_goal_wins` + `pushing_token_onto_enemy_goal_loses`.
//! * NO-MOVE RULE: the player to move who has NO legal move LOSES (the opponent
//!   wins) — bodogemu's "or if the opponent is blocked", the normal-play
//!   convention this arcade's other walk-and-burn games (Trails, Domineering)
//!   also use. → test `boxed_in_player_loses`.
//!
//! WHY NO REACHABILITY RESTRICTION: the CGT graph version adds "you may not move
//! the token to a space from which it can't reach at least one goal" to keep the
//! game winnable / avoid draws. Under the normal-play "blocked player loses"
//! adjudication above the game is ALREADY fully decisive and always terminates
//! (the slimed set strictly grows on every move, the board is finite), so that
//! helper rule is unnecessary here — and dropping it keeps the rule kid-simple
//! ("walk to any next-door square that isn't slime"). Legal moves are therefore
//! exactly the empty 8-neighbours of the token.
use crate::gridlib::idx;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

const SLIME: i8 = -2;
const EMPTY: i8 = -1;

#[derive(Clone)]
struct Slime {
    /// Per-cell state: -1 empty, -2 slime. The token's own cell stays EMPTY
    /// (it is tracked by `pos`); it only turns to slime once the token leaves.
    board: Vec<i8>,
    /// Cell the shared token currently occupies.
    pos: usize,
    size: usize,
    current: u8,
    /// goal[0] = P0 (Red, bottom-left), goal[1] = P1 (Gold, top-right).
    goal: [usize; 2],
}

impl Slime {
    fn new(size: usize) -> Self {
        let board = vec![EMPTY; size * size];
        // Sourced start: the board centre.
        let pos = idx(size / 2, size / 2, size);
        // Sourced goals: diagonally-opposite corners.
        let goal = [idx(size - 1, 0, size), idx(0, size - 1, size)];
        Self { board, pos, size, current: 0, goal }
    }

    /// The 8 king-move neighbours of `cell`, bounds-checked (sourced adjacency).
    fn eight_neighbors(&self, cell: usize) -> Vec<usize> {
        let (r, c) = ((cell / self.size) as i32, (cell % self.size) as i32);
        const OFF: [(i32, i32); 8] =
            [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];
        let mut v = Vec::new();
        for (dr, dc) in OFF {
            let (nr, nc) = (r + dr, c + dc);
            if nr >= 0 && nr < self.size as i32 && nc >= 0 && nc < self.size as i32 {
                v.push(idx(nr as usize, nc as usize, self.size));
            }
        }
        v
    }

    /// Legal destinations: empty 8-neighbours of the token.
    fn gen(&self) -> Vec<u16> {
        self.eight_neighbors(self.pos)
            .into_iter()
            .filter(|&to| self.board[to] == EMPTY)
            .map(|to| to as u16)
            .collect()
    }

    /// The owner of a goal cell, if `cell` is a goal.
    fn goal_owner(&self, cell: usize) -> Option<u8> {
        if cell == self.goal[0] {
            Some(0)
        } else if cell == self.goal[1] {
            Some(1)
        } else {
            None
        }
    }

    fn term(&self) -> Option<ProvenValue> {
        // Reaching a goal ends it immediately — the goal's OWNER wins, whoever
        // moved the token there. Negamax perspective is the player to move.
        if let Some(owner) = self.goal_owner(self.pos) {
            return Some(if owner == self.current { ProvenValue::Win } else { ProvenValue::Loss });
        }
        // Otherwise, a player with no move loses (normal play).
        if self.gen().is_empty() {
            return Some(ProvenValue::Loss);
        }
        None
    }
}

impl GameState for Slime {
    type Move = u16;
    type Player = u8;
    type MoveList = Vec<u16>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<u16> {
        // No moves once the token has reached a goal (so playouts stop at a win).
        if self.term().is_some() {
            return vec![];
        }
        self.gen()
    }
    fn make_move(&mut self, m: &u16) {
        self.board[self.pos] = SLIME; // the vacated cell turns to permanent slime
        self.pos = *m as usize;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}

struct SlimeEval;
impl Evaluator<SlimeCfg> for SlimeEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Slime, m: &Vec<u16>, _: Option<SearchHandle<SlimeCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Slime, _: &i64, _: SearchHandle<SlimeCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct SlimeCfg;
impl MCTS for SlimeCfg {
    type State = Slime;
    type Eval = SlimeEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct SlimetrailWasm {
    manager: MCTSManager<SlimeCfg>,
    size: usize,
}
#[wasm_bindgen]
impl SlimetrailWasm {
    /// `size` = board edge, clamped 5..=9 (default preset 7). Goals sit in the
    /// bottom-left (P0/Red) and top-right (P1/Gold) corners; the token starts
    /// centred.
    #[wasm_bindgen(constructor)]
    pub fn new(size: u32) -> Self {
        let size = (size as usize).clamp(5, 9);
        Self { manager: MCTSManager::new(Slime::new(size), SlimeCfg, SlimeEval, UCTPolicy::new(1.4), ()), size }
    }
    pub fn size(&self) -> u32 {
        self.size as u32
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// One char per cell: ' '=empty, '#'=slime, 'T'=the shared token. Goal corners
    /// are not encoded here — the UI derives them from the board size.
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        (0..s.board.len())
            .map(|i| {
                if i == s.pos {
                    'T'
                } else if s.board[i] == SLIME {
                    '#'
                } else {
                    ' '
                }
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
            _ => String::new(),
        }
    }
    /// Comma-joined destination cell indices (the token's legal steps).
    pub fn legal_moves(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .available_moves()
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }
    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| m.to_string())
    }
    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m: u16 = match mov.parse() {
            Ok(v) => v,
            _ => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, SlimeCfg, SlimeEval, UCTPolicy::new(1.4), ());
        true
    }
    /// The winning goal cell (glowed by the UI) when the token has reached a
    /// goal; empty otherwise (including a boxed-in loss, where no goal is reached).
    pub fn winning_cells(&self) -> String {
        let s = self.manager.tree().root_state();
        if s.goal_owner(s.pos).is_some() {
            s.pos.to_string()
        } else {
            String::new()
        }
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Slime::new(self.size), SlimeCfg, SlimeEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_is_centre() {
        for n in 5..=9 {
            let s = Slime::new(n);
            assert_eq!(s.pos, idx(n / 2, n / 2, n), "token must start at the centre");
            assert_eq!(s.board[s.pos], EMPTY, "the token cell is not slime");
        }
    }

    #[test]
    fn goals_in_opposite_corners() {
        let s = Slime::new(7);
        assert_eq!(s.goal[0], idx(6, 0, 7), "P0/Red goal is bottom-left");
        assert_eq!(s.goal[1], idx(0, 6, 7), "P1/Gold goal is top-right");
        assert_eq!(s.goal_owner(s.goal[0]), Some(0));
        assert_eq!(s.goal_owner(s.goal[1]), Some(1));
        assert_eq!(s.goal_owner(s.pos), None, "centre is not a goal");
    }

    #[test]
    fn adjacency_is_eight_directional() {
        // A centred token on a 5×5 has all 8 king moves available.
        let s = Slime::new(5);
        let mut got: Vec<u16> = s.gen();
        got.sort_unstable();
        let c = s.pos;
        let mut want: Vec<u16> = s.eight_neighbors(c).into_iter().map(|x| x as u16).collect();
        want.sort_unstable();
        assert_eq!(got, want);
        assert_eq!(got.len(), 8, "centre token has 8 neighbours (incl. diagonals)");
        // At least one move must be a pure diagonal (dr != 0 && dc != 0).
        let (r0, c0) = ((c / 5) as i32, (c % 5) as i32);
        assert!(
            got.iter().any(|&m| {
                let (r, cc) = ((m as i32 / 5), (m as i32 % 5));
                (r - r0).abs() == 1 && (cc - c0).abs() == 1
            }),
            "diagonal moves must be legal"
        );
    }

    #[test]
    fn moving_leaves_slime_that_cannot_be_reentered() {
        let mut s = Slime::new(5);
        let start = s.pos;
        let dest = s.gen()[0] as usize;
        s.make_move(&(dest as u16));
        assert_eq!(s.board[start], SLIME, "vacated cell is now slime");
        assert_eq!(s.pos, dest);
        assert_eq!(s.current, 1);
        // The slimed start cell is a neighbour of `dest` but must NOT be offered.
        assert!(
            !s.gen().contains(&(start as u16)),
            "the token can never step back onto slime"
        );
    }

    #[test]
    fn reaching_own_goal_wins() {
        // Put P0's token orthogonally next to its own goal, on P0's turn.
        let mut s = Slime::new(5);
        let goal = s.goal[0]; // bottom-left (4,0)
        s.pos = idx(4, 1, 5); // one step right of the goal (same row)
        s.current = 0;
        assert!(s.gen().contains(&(goal as u16)), "goal must be a legal step");
        s.make_move(&(goal as u16));
        // Token is on P0's goal; it is now P1 to move → P1 sees a Loss.
        assert_eq!(s.current, 1);
        assert_eq!(s.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn pushing_token_onto_enemy_goal_loses() {
        // P0 is forced to move the token onto P1's goal → P1 (the owner) wins,
        // even though P0 made the move (the "it doesn't matter by whom" rule).
        let mut s = Slime::new(5);
        let enemy_goal = s.goal[1]; // top-right (0,4)
        s.pos = idx(0, 3, 5); // one step left of P1's goal
        s.current = 0;
        s.make_move(&(enemy_goal as u16));
        // Token on P1's goal, now P1 to move → owner == current → Win for P1.
        assert_eq!(s.current, 1);
        assert_eq!(s.term(), Some(ProvenValue::Win));
    }

    #[test]
    fn win_result_string_is_the_goal_owner() {
        // Drive the wasm surface: P0 steps onto its own goal, result must be "1".
        let mut g = SlimetrailWasm::new(5);
        // Rebuild an internal state with the token beside P0's goal.
        let mut s = Slime::new(5);
        s.pos = idx(4, 1, 5);
        s.current = 0;
        g.manager = MCTSManager::new(s, SlimeCfg, SlimeEval, UCTPolicy::new(1.4), ());
        let goal = idx(4, 0, 5);
        assert!(g.apply_move(&goal.to_string()));
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1", "Red owns the bottom-left goal");
        assert_eq!(g.winning_cells(), goal.to_string(), "the reached goal glows");
    }

    #[test]
    fn boxed_in_player_loses() {
        // Surround the (non-goal) token with slime → the player to move is boxed
        // in and loses (opponent wins). Sourced no-move rule.
        let mut s = Slime::new(5);
        s.pos = idx(2, 2, 5);
        s.current = 0;
        for nb in s.eight_neighbors(s.pos) {
            s.board[nb] = SLIME;
        }
        assert!(s.gen().is_empty());
        assert_eq!(s.term(), Some(ProvenValue::Loss), "boxed-in mover loses");
    }

    #[test]
    fn winning_cells_empty_mid_game() {
        let g = SlimetrailWasm::new(7);
        assert_eq!(g.winning_cells(), "", "no goal reached yet");
        assert!(!g.is_terminal());
    }

    #[test]
    fn round_trip_move_strings() {
        let mut g = SlimetrailWasm::new(7);
        let before = g.get_board();
        let first = g.legal_moves().split(',').next().unwrap().to_string();
        assert!(g.apply_move(&first), "a legal destination index round-trips");
        assert_ne!(g.get_board(), before);
        assert!(!g.apply_move("999"), "an out-of-range index is rejected");
        // best_move round-trips through apply_move.
        g.playout_n(60);
        let bm = g.best_move().unwrap();
        assert!(g.apply_move(&bm), "best_move {bm} must be applicable");
    }

    #[test]
    fn ai_plays() {
        let mut g = SlimetrailWasm::new(7);
        g.playout_n(300);
        assert!(g.best_move().is_some());
    }

    #[test]
    fn full_game_terminates_and_is_decisive() {
        // Self-play to a terminal on every board size; the game must always end
        // (slime strictly grows) with a decided, non-empty result (no draws).
        for n in 5..=9 {
            let mut g = SlimetrailWasm::new(n);
            for _ in 0..(n * n + 5) {
                if g.is_terminal() {
                    break;
                }
                let m = g.weak_move(120, 2, 0.4, 7).or_else(|| g.best_move());
                match m {
                    Some(mv) => assert!(g.apply_move(&mv), "engine emitted an unplayable move: {mv}"),
                    None => break,
                }
            }
            assert!(g.is_terminal(), "size {n} game must terminate");
            assert!(!g.result().is_empty(), "Slimetrail is decisive — no draws");
        }
    }
}
