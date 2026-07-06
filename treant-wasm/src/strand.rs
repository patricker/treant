//! Strand — maroon your rival. Each turn has TWO parts: (1) step your pawn one
//! square in any of the 8 directions onto a remaining tile; (2) remove ANY
//! remaining empty tile, punching a permanent hole no pawn may enter. A player
//! who cannot step on their turn is marooned and loses.
//!
//! The mechanic is the public-domain ruleset of the 1972 board game documented
//! at <https://en.wikipedia.org/wiki/Isolation_(board_game)> (classic board
//! 6×8, 46 removable tiles; "move their pawn 1 square in any direction, and
//! then … remove any tile on the board, leaving an empty square that cannot be
//! entered by either pawn"; "the first player to isolate their opponent's pawn
//! so that it has no legal move is the winner"; pawns start on the raised
//! platforms at the middle of opposite edges). Rules are unprotectable; our
//! name/theme differ and the trademarked product names never appear in the UI.
//! We default to a square 7×7 for board symmetry (clamped 5–8 each side) and
//! keep the move-then-remove turn and isolate-to-win condition exactly.
use crate::gridlib::idx;
use std::collections::VecDeque;
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

const HOLE: i8 = -2; // a removed tile — a permanent hole, distinct from Trails' walls

/// A full Strand turn: step `from`→`to`, then remove tile `removed`. The
/// three-part `"from-to-removed"` string is the single source of truth for the
/// wire encoding — `Display` writes it and [`StMove::parse`] reads it, so the
/// two can never diverge (round-trip covered by a unit test).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StMove {
    from: u16,
    to: u16,
    removed: u16,
}
impl std::fmt::Display for StMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}-{}", self.from, self.to, self.removed)
    }
}
impl StMove {
    /// Parse the canonical `"from-to-removed"` encoding. `None` on any shape
    /// that is not exactly three dash-separated non-negative integers.
    fn parse(s: &str) -> Option<StMove> {
        let mut it = s.split('-');
        let from = it.next()?.parse().ok()?;
        let to = it.next()?.parse().ok()?;
        let removed = it.next()?.parse().ok()?;
        if it.next().is_some() {
            return None; // trailing junk (e.g. a 4th field)
        }
        Some(StMove { from, to, removed })
    }
}

#[derive(Clone)]
struct Strand {
    board: Vec<i8>, // -1 empty tile, -2 hole (removed), 0/1 = that player's pawn cell
    pos: [usize; 2],
    cols: usize,
    rows: usize,
    current: u8,
}
impl Strand {
    fn new(cols: usize, rows: usize) -> Self {
        let mut board = vec![-1i8; cols * rows];
        // Pawns start on the middle of opposite edges (classic Isolation's
        // raised platforms), same centre column on the top and bottom rows.
        let p0 = idx(0, cols / 2, cols);
        let p1 = idx(rows - 1, cols / 2, cols);
        board[p0] = 0;
        board[p1] = 1;
        Self { board, pos: [p0, p1], cols, rows, current: 0 }
    }
    /// The (up to) 8 king-step neighbours of `cell`, bounds-checked.
    fn neigh8(&self, cell: usize) -> Vec<usize> {
        let (r, c) = ((cell / self.cols) as i32, (cell % self.cols) as i32);
        let mut v = Vec::with_capacity(8);
        for dr in -1..=1 {
            for dc in -1..=1 {
                if dr == 0 && dc == 0 {
                    continue;
                }
                let (nr, nc) = (r + dr, c + dc);
                if nr >= 0 && nr < self.rows as i32 && nc >= 0 && nc < self.cols as i32 {
                    v.push(idx(nr as usize, nc as usize, self.cols));
                }
            }
        }
        v
    }
    fn gen(&self) -> Vec<StMove> {
        let from = self.pos[self.current as usize];
        // Empty tiles you could remove BEFORE moving (holes and both pawns are
        // excluded because their board value is not -1).
        let empties: Vec<usize> = (0..self.board.len()).filter(|&j| self.board[j] == -1).collect();
        let mut moves = Vec::new();
        for to in self.neigh8(from) {
            if self.board[to] != -1 {
                continue; // destination must be a remaining, unoccupied tile
            }
            // After the step: `to` is occupied (not removable) and the vacated
            // `from` becomes an empty tile (removable). Every other empty stays.
            for &rem in &empties {
                if rem == to {
                    continue;
                }
                moves.push(StMove { from: from as u16, to: to as u16, removed: rem as u16 });
            }
            // The just-vacated cell is always a legal removal target.
            moves.push(StMove { from: from as u16, to: to as u16, removed: from as u16 });
        }
        moves
    }
    /// Empty tiles reachable from a pawn through king-steps — a mobility proxy
    /// (more open space around you = safer), matching Trails' reachable-area
    /// heuristic but over the 8-direction Strand move.
    fn reach(&self, player: u8) -> i64 {
        let mut seen = vec![false; self.board.len()];
        let mut dq = VecDeque::new();
        let start = self.pos[player as usize];
        dq.push_back(start);
        seen[start] = true;
        let mut n = 0i64;
        while let Some(cell) = dq.pop_front() {
            for nb in self.neigh8(cell) {
                if !seen[nb] && self.board[nb] == -1 {
                    seen[nb] = true;
                    n += 1;
                    dq.push_back(nb);
                }
            }
        }
        n
    }
    fn term(&self) -> Option<ProvenValue> {
        // A pawn with no step is isolated. Once it can step it can always remove
        // the tile it just left, so mobility alone decides the turn.
        if self.neigh8(self.pos[self.current as usize]).iter().all(|&t| self.board[t] != -1) {
            Some(ProvenValue::Loss) // current player is marooned -> loses
        } else {
            None
        }
    }
}
impl GameState for Strand {
    type Move = StMove;
    type Player = u8;
    type MoveList = Vec<StMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<StMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &StMove) {
        self.board[m.from as usize] = -1; // vacate (now an empty tile)
        self.board[m.to as usize] = self.current as i8; // step onto the destination
        self.board[m.removed as usize] = HOLE; // punch the hole (may be the vacated cell)
        self.pos[self.current as usize] = m.to as usize;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct StEval;
impl Evaluator<StCfg> for StEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, s: &Strand, m: &Vec<StMove>, _: Option<SearchHandle<StCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], s.reach(0) - s.reach(1))
    }
    fn interpret_evaluation_for_player(&self, e: &i64, p: &u8) -> i64 {
        if *p == 0 {
            *e
        } else {
            -*e
        }
    }
    fn evaluate_existing_state(&self, s: &Strand, _: &i64, _: SearchHandle<StCfg>) -> i64 {
        s.reach(0) - s.reach(1)
    }
}
#[derive(Default)]
struct StCfg;
impl MCTS for StCfg {
    type State = Strand;
    type Eval = StEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    // Solver ON to match the Trails family: tiles strictly decrease so every
    // line terminates, and exact endgame solving sharpens the marooning finish.
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct StrandWasm {
    manager: MCTSManager<StCfg>,
    cols: usize,
    rows: usize,
}
#[wasm_bindgen]
impl StrandWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32) -> Self {
        let cols = (cols as usize).clamp(5, 8);
        let rows = (rows as usize).clamp(5, 8);
        Self {
            manager: MCTSManager::new(Strand::new(cols, rows), StCfg, StEval, UCTPolicy::new(1.4), ()),
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
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// One char per cell: `X`/`O` = the two pawns, `.` = a hole (removed tile),
    /// space = a remaining empty tile. The hole glyph differs from Trails' `#`
    /// wall so the family boards read differently, and — crucially — a move
    /// changes the glyph at ALL THREE of its cells (from `X`→space, to
    /// space→`X`, removed space→`.`), so the session's board diff marks the
    /// AI's whole two-part move.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .board
            .iter()
            .map(|&v| match v {
                0 => 'X',
                1 => 'O',
                HOLE => '.',
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
            Some(ProvenValue::Loss) => format!("{}", (1 - s.current) + 1),
            _ => String::new(),
        }
    }
    pub fn legal_moves(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .available_moves()
            .iter()
            .map(|m| format!("{m}"))
            .collect::<Vec<_>>()
            .join(",")
    }
    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }
    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m = match StMove::parse(mov) {
            Some(m) => m,
            None => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, StCfg, StEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(Strand::new(self.cols, self.rows), StCfg, StEval, UCTPolicy::new(1.4), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_encoding_round_trips() {
        // Every legal move on a fresh board must survive Display -> parse
        // unchanged (the single-sourced three-part encoding contract).
        let g = Strand::new(7, 7);
        for m in g.gen() {
            let s = format!("{m}");
            assert_eq!(StMove::parse(&s), Some(m), "round-trip failed for {s}");
        }
        // Malformed strings are rejected (not enough / too many fields).
        assert_eq!(StMove::parse("3-7"), None);
        assert_eq!(StMove::parse("3-7-9-1"), None);
        assert_eq!(StMove::parse("a-7-9"), None);
    }

    #[test]
    fn a_turn_steps_then_punches_a_hole() {
        let mut g = Strand::new(7, 7);
        let from = g.pos[0];
        let m = g.gen()[0];
        g.make_move(&m);
        assert_eq!(g.board[m.to as usize], 0, "pawn stepped onto destination");
        assert_eq!(g.pos[0], m.to as usize);
        assert_eq!(g.board[m.removed as usize], HOLE, "removed tile is a hole");
        // If the removal wasn't the vacated cell, `from` is now an empty tile.
        if m.removed as usize != from {
            assert_eq!(g.board[from], -1);
        }
        assert_eq!(g.current, 1);
    }

    #[test]
    fn cannot_remove_an_occupied_or_hole_tile() {
        let g = Strand::new(7, 7);
        let (p0, p1) = (g.pos[0], g.pos[1]);
        for m in g.gen() {
            assert_ne!(m.removed as usize, m.to as usize, "removed the tile just moved onto");
            assert_ne!(m.removed as usize, p1, "removed the opponent's pawn tile");
            // Removing your own new cell is impossible (removed != to); removing
            // your OLD cell is legal, so p0 may equal removed only via `from`.
            assert!(m.removed as usize == p0 || g.board[m.removed as usize] == -1);
        }
    }

    #[test]
    fn removal_is_mandatory_every_move_has_three_parts() {
        // Every generated move names a removal; there is no move-without-remove.
        let g = Strand::new(7, 7);
        let moves = g.gen();
        assert!(!moves.is_empty());
        // A pawn that can step always has a tile to remove (at least the cell it
        // just left), so #moves == #destinations * #empties-after-step > 0.
        for m in &moves {
            assert!(m.from != m.to);
        }
    }

    #[test]
    fn marooned_player_loses() {
        let mut g = Strand::new(7, 7);
        // Punch holes around player 0's pawn so it has no king-step.
        let p0 = g.pos[0];
        for nb in g.neigh8(p0) {
            g.board[nb] = HOLE;
        }
        g.current = 0;
        assert_eq!(g.term(), Some(ProvenValue::Loss));
        assert_eq!(g.gen().len(), 0);
    }

    #[test]
    fn ai_plays() {
        let mut g = StrandWasm::new(7, 7);
        g.playout_n(300);
        let mv = g.best_move();
        assert!(mv.is_some());
        assert!(g.apply_move(&mv.unwrap()), "best move must be applicable");
    }

    #[test]
    fn apply_move_rejects_illegal_and_malformed() {
        let mut g = StrandWasm::new(7, 7);
        assert!(!g.apply_move("nonsense"));
        assert!(!g.apply_move("0-1")); // two parts, not three
        // Stepping onto your own start with a bogus removal is illegal.
        assert!(!g.apply_move("3-3-3"));
    }

    #[test]
    fn clamps_board_to_five_through_eight() {
        let small = StrandWasm::new(2, 2);
        assert_eq!((small.cols(), small.rows()), (5, 5));
        let big = StrandWasm::new(99, 99);
        assert_eq!((big.cols(), big.rows()), (8, 8));
        // Board string length matches the clamped dims (no silent-clamp lie).
        assert_eq!(big.get_board().chars().count(), 64);
    }

    #[test]
    fn small_endgame_is_solvable() {
        // On a tight board with most tiles gone, the solver should prove a
        // result quickly (a decisive marooning is near). Play a short self-game
        // and confirm it terminates with a well-formed winner.
        let mut g = StrandWasm::new(5, 5);
        for _ in 0..200 {
            if g.is_terminal() {
                break;
            }
            g.playout_n(200);
            let m = g.best_move().expect("non-terminal has a move");
            assert!(g.apply_move(&m));
        }
        assert!(g.is_terminal(), "a 5x5 Strand self-game reaches a terminal");
        let r = g.result();
        assert!(r == "1" || r == "2", "well-formed winner, got {r:?}");
    }
}
