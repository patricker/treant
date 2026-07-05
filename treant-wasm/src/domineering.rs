//! Domineering — a partisan tile-placing game. The two players have *different*
//! moves: player 0 (Vertical) places dominoes covering two vertically-adjacent
//! empty cells, player 1 (Horizontal) places horizontally-adjacent ones. The
//! player who cannot place a domino loses (normal play). No draws, so treant's
//! exact solver plays it perfectly on small boards.
//!
//! With `cram` set the game becomes **Cram** — the *impartial* variant: BOTH
//! players may place a domino in EITHER orientation. The terminal rule is
//! unchanged (last to place wins / first who cannot move loses).
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

/// A domino placement, carrying both cells it covers as row-major grid indices:
/// `from` is the anchor (top cell of a vertical domino / left cell of a
/// horizontal one), `to` its partner (the cell below / to the right). Encoded
/// for the UI as the string `"from-to"`, so the move alone determines the
/// partner — no orientation lookup needed (essential for Cram, where a single
/// anchor cell can host both a vertical and a horizontal domino).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
struct DomMove {
    from: u16,
    to: u16,
}
impl std::fmt::Display for DomMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}
fn parse_dom_move(s: &str) -> Option<DomMove> {
    let (a, b) = s.split_once('-')?;
    Some(DomMove { from: a.parse().ok()?, to: b.parse().ok()? })
}

#[derive(Clone)]
struct Domineering {
    cols: usize,
    rows: usize,
    grid: Vec<i8>, // -1 empty, else owner (0 or 1)
    current: u8,
    cram: bool, // impartial variant: either player may place either orientation
}
impl Domineering {
    fn new(cols: usize, rows: usize, cram: bool) -> Self {
        Self { cols, rows, grid: vec![-1; cols * rows], current: 0, cram }
    }
    /// Legal domino placements for the player to move. In standard Domineering
    /// player 0 places only vertical dominoes and player 1 only horizontal
    /// ones; in Cram both orientations are available to whoever is to move.
    fn gen(&self) -> Vec<DomMove> {
        let mut v = Vec::new();
        // Impartial (Cram) unlocks both orientations for either player.
        let vertical_ok = self.cram || self.current == 0;
        let horizontal_ok = self.cram || self.current == 1;
        for r in 0..self.rows {
            for c in 0..self.cols {
                let i = r * self.cols + c;
                if self.grid[i] != -1 {
                    continue;
                }
                // vertical: also need the cell directly below to be empty
                if vertical_ok && r + 1 < self.rows && self.grid[i + self.cols] == -1 {
                    v.push(DomMove { from: i as u16, to: (i + self.cols) as u16 });
                }
                // horizontal: also need the cell directly to the right
                if horizontal_ok && c + 1 < self.cols && self.grid[i + 1] == -1 {
                    v.push(DomMove { from: i as u16, to: (i + 1) as u16 });
                }
            }
        }
        v
    }
    fn term(&self) -> Option<ProvenValue> {
        if self.gen().is_empty() {
            Some(ProvenValue::Loss) // current can't move -> loses
        } else {
            None
        }
    }
}
impl GameState for Domineering {
    type Move = DomMove;
    type Player = u8;
    type MoveList = Vec<DomMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<DomMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &DomMove) {
        // The move carries both cells, so no orientation lookup is needed.
        self.grid[m.from as usize] = self.current as i8;
        self.grid[m.to as usize] = self.current as i8;
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}
struct DomEval;
impl Evaluator<DomCfg> for DomEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &Domineering, m: &Vec<DomMove>, _: Option<SearchHandle<DomCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &Domineering, _: &i64, _: SearchHandle<DomCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct DomCfg;
impl MCTS for DomCfg {
    type State = Domineering;
    type Eval = DomEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

#[wasm_bindgen]
pub struct DomineeringWasm {
    manager: MCTSManager<DomCfg>,
    cols: usize,
    rows: usize,
    cram: bool,
}
#[wasm_bindgen]
impl DomineeringWasm {
    /// `cram != 0` selects the impartial Cram variant (both players may place
    /// either orientation); `0` is standard partisan Domineering.
    #[wasm_bindgen(constructor)]
    pub fn new(cols: usize, rows: usize, cram: u32) -> Self {
        let cram = cram != 0;
        Self {
            manager: MCTSManager::new(Domineering::new(cols, rows, cram), DomCfg, DomEval, UCTPolicy::new(1.4), ()),
            cols,
            rows,
            cram,
        }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// One char per cell, row-major: ' '=empty, 'X'=player 0, 'O'=player 1.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .grid
            .iter()
            .map(|&v| match v {
                0 => 'X',
                1 => 'O',
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
        let mv = match parse_dom_move(mov) {
            Some(m) => m,
            None => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&mv) {
            return false;
        }
        s.make_move(&mv);
        self.manager = MCTSManager::new(s, DomCfg, DomEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(
            Domineering::new(self.cols, self.rows, self.cram),
            DomCfg,
            DomEval,
            UCTPolicy::new(1.4),
            (),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_player_only_makes_vertical_dominoes() {
        let g = Domineering::new(2, 2, false); // 2x2, player 0 to move
        // anchors 0 and 1 are valid (each has a cell below): covers {0,2} and {1,3}.
        let mut moves = g.gen();
        moves.sort_by_key(|m| (m.from, m.to));
        assert_eq!(moves, vec![DomMove { from: 0, to: 2 }, DomMove { from: 1, to: 3 }]);
    }

    #[test]
    fn placing_fills_both_cells_and_swaps_player() {
        let mut g = Domineering::new(3, 3, false);
        g.make_move(&DomMove { from: 0, to: 3 }); // vertical domino covers cells 0 and 3
        assert_eq!(g.grid[0], 0);
        assert_eq!(g.grid[3], 0);
        assert_eq!(g.current, 1);
        // horizontal player now: anchor 1 covers {1,2}; anchor 0 is taken.
        assert!(g.gen().iter().any(|m| m.from == 1 && m.to == 2));
        assert!(!g.gen().iter().any(|m| m.from == 0));
    }

    #[test]
    fn full_blocked_board_is_a_loss_for_mover() {
        // 1x2 board: player 0 (vertical) can never move -> immediate loss.
        let g = Domineering::new(2, 1, false);
        assert!(g.gen().is_empty());
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn ai_plays() {
        let mut g = DomineeringWasm::new(4, 4, 0);
        g.playout_n(800);
        assert!(g.best_move().is_some());
    }

    #[test]
    fn cram_lets_either_player_place_both_orientations() {
        let g = DomineeringWasm::new(4, 4, 1);
        let moves = g.legal_moves();
        // 4×4 empty board: 12 vertical + 12 horizontal anchors = 24 moves for P0.
        assert_eq!(moves.split(',').count(), 24);
    }
}
