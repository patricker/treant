use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

// --- Game (multi-heap Nim: heaps, max-take, misère) ---
//
// State is a set of heaps. On a turn the mover removes between 1 and
// `max_take` stones from a single non-empty heap (`max_take == 0` means "take
// as many as you like from one heap" — classic unbounded Nim). When the last
// stone is taken the game ends:
//   - normal play: the player who took the last stone WINS (the player now to
//     move — facing empty heaps — LOSES);
//   - misère play: the player who took the last stone LOSES (the player now to
//     move WINS).
// Single heap + max_take 2 + normal play reproduces the original toy exactly.

#[derive(Clone, Debug)]
struct Nim {
    heaps: Vec<u8>,
    max_take: u8, // 0 == unbounded
    misere: bool,
    current: Player,
}

impl Nim {
    fn total(&self) -> u32 {
        self.heaps.iter().map(|&h| h as u32).sum()
    }

    /// How many stones may be taken from a heap of size `h` this turn.
    fn cap(&self, h: u8) -> u8 {
        if self.max_take == 0 {
            h
        } else {
            h.min(self.max_take)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Player {
    P1,
    P2,
}

fn other(p: Player) -> Player {
    match p {
        Player::P1 => Player::P2,
        Player::P2 => Player::P1,
    }
}

/// A move takes `count` stones from heap `heap` (0-indexed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NimMove {
    heap: u8,
    count: u8,
}

impl std::fmt::Display for NimMove {
    // Single source of truth for the move wire format: "<heap>-<count>".
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}-{}", self.heap, self.count)
    }
}

impl std::str::FromStr for NimMove {
    type Err = ();
    // Inverse of Display — parse "<heap>-<count>". Single-sourced with Display
    // so the display/encode/decode round-trip can never drift.
    fn from_str(s: &str) -> Result<Self, ()> {
        let (h, c) = s.split_once('-').ok_or(())?;
        Ok(NimMove {
            heap: h.parse().map_err(|_| ())?,
            count: c.parse().map_err(|_| ())?,
        })
    }
}

impl GameState for Nim {
    type Move = NimMove;
    type Player = Player;
    type MoveList = Vec<NimMove>;

    fn current_player(&self) -> Player {
        self.current
    }

    fn available_moves(&self) -> Vec<NimMove> {
        let mut moves = Vec::new();
        for (i, &h) in self.heaps.iter().enumerate() {
            for count in 1..=self.cap(h) {
                moves.push(NimMove {
                    heap: i as u8,
                    count,
                });
            }
        }
        moves
    }

    fn make_move(&mut self, mov: &NimMove) {
        self.heaps[mov.heap as usize] -= mov.count;
        self.current = other(self.current);
    }

    fn terminal_value(&self) -> Option<ProvenValue> {
        if self.total() == 0 {
            // From the perspective of the player now to move (who faces empty
            // heaps): normal play they LOSE, misère they WIN.
            Some(if self.misere {
                ProvenValue::Win
            } else {
                ProvenValue::Loss
            })
        } else {
            None
        }
    }
}

struct NimEval;

impl Evaluator<NimConfig> for NimEval {
    type StateEvaluation = Option<Player>;

    fn evaluate_new_state(
        &self,
        state: &Nim,
        moves: &Vec<NimMove>,
        _: Option<SearchHandle<NimConfig>>,
    ) -> (Vec<()>, Option<Player>) {
        let winner = if state.total() == 0 {
            // The mover took the last stone; `state.current` is now the OTHER
            // player. Normal play: the taker (other(current)) wins. Misère: the
            // player now to move (current) wins.
            Some(if state.misere {
                state.current
            } else {
                other(state.current)
            })
        } else {
            None
        };
        (vec![(); moves.len()], winner)
    }

    fn interpret_evaluation_for_player(&self, winner: &Option<Player>, player: &Player) -> i64 {
        match winner {
            Some(w) if w == player => 100,
            Some(_) => -100,
            None => 0,
        }
    }

    fn evaluate_existing_state(
        &self,
        _: &Nim,
        evaln: &Option<Player>,
        _: SearchHandle<NimConfig>,
    ) -> Option<Player> {
        *evaln
    }
}

#[derive(Default)]
struct NimConfig;

impl MCTS for NimConfig {
    type State = Nim;
    type Eval = NimEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();

    fn solver_enabled(&self) -> bool {
        true
    }
}

/// Build the starting heaps: the first heap is `stones`, each subsequent heap
/// is 2 smaller, floored at 1. stones=7, heaps=4 → [7,5,3,1] (Marienbad).
fn build_heaps(stones: u8, heaps: u8) -> Vec<u8> {
    let mut hs = Vec::with_capacity(heaps as usize);
    let mut v = stones;
    for _ in 0..heaps {
        hs.push(v);
        v = if v > 2 { v - 2 } else { 1 };
    }
    hs
}

// --- WASM API ---

#[wasm_bindgen]
pub struct NimWasm {
    manager: MCTSManager<NimConfig>,
    initial: Nim,
}

fn fresh(state: &Nim) -> MCTSManager<NimConfig> {
    MCTSManager::new(
        state.clone(),
        NimConfig,
        NimEval,
        UCTPolicy::new(1.0),
        (),
    )
}

#[wasm_bindgen]
impl NimWasm {
    /// - `stones`: size of the first heap (clamped 1..=60).
    /// - `heaps`: number of heaps (clamped 1..=8); sizes descend by 2, floor 1.
    /// - `max_take`: 0 = unbounded, else clamped 1..=10.
    /// - `misere`: 0 = normal play, non-zero = misère (last to take loses).
    ///
    /// `new(15, 1, 2, 0)` is the original single-pile toy.
    #[wasm_bindgen(constructor)]
    pub fn new(stones: u32, heaps: u32, max_take: u32, misere: u32) -> Self {
        let stones = stones.clamp(1, 60) as u8;
        let heaps = heaps.clamp(1, 8) as u8;
        let max_take = if max_take == 0 {
            0
        } else {
            max_take.clamp(1, 10) as u8
        };
        let initial = Nim {
            heaps: build_heaps(stones, heaps),
            max_take,
            misere: misere != 0,
            current: Player::P1,
        };
        Self {
            manager: fresh(&initial),
            initial,
        }
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        let stats = types::build_stats(&self.manager, |_| None);
        serde_wasm_bindgen::to_value(&stats).unwrap_or(JsValue::NULL)
    }

    pub fn get_tree(&self, max_depth: u32) -> JsValue {
        let tree =
            types::export_tree::<NimConfig>(self.manager.tree().root_node(), max_depth, &|_| None);
        serde_wasm_bindgen::to_value(&tree).unwrap_or(JsValue::NULL)
    }

    pub fn root_proven_value(&self) -> String {
        format!("{:?}", self.manager.root_proven_value())
    }

    /// Board encoding: comma-joined heap counts, e.g. "7,5,3,1". A single heap
    /// encodes to a bare number ("15"), byte-identical to the original board.
    pub fn get_board(&self) -> String {
        self.manager
            .tree()
            .root_state()
            .heaps
            .iter()
            .map(|h| h.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn current_player(&self) -> String {
        format!("{:?}", self.manager.tree().root_state().current)
    }

    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().total() == 0
    }

    /// Canonical result contract: "" in progress, else the 1-indexed winner
    /// seat ("1"/"2"). Misère polarity lives HERE only, so the tile can never
    /// disagree with the engine about who won.
    pub fn result(&self) -> String {
        let s = self.manager.tree().root_state();
        if s.total() != 0 {
            return String::new();
        }
        // At terminal the mover took the last stone; `current` is the other
        // player. Normal: the taker (other of current) wins. Misère: the player
        // now to move (current) wins.
        let winner = if s.misere { s.current } else { other(s.current) };
        match winner {
            Player::P1 => "1".to_string(),
            Player::P2 => "2".to_string(),
        }
    }

    /// The current player's legal moves as comma-joined "<heap>-<count>" tokens
    /// (e.g. "0-1,0-2,1-1"). Empty at terminal. Single-sourced through Display.
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
        self.manager.best_move().map(|m| format!("{m}"))
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }

    /// Apply a move and advance the tree (preserving search). Rejects anything
    /// that isn't a currently-legal move. Runs a few playouts first if needed
    /// to ensure the child is expanded.
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m: NimMove = match mov.parse() {
            Ok(m) => m,
            Err(()) => return false,
        };
        if !self
            .manager
            .tree()
            .root_state()
            .available_moves()
            .contains(&m)
        {
            return false;
        }
        if self.manager.advance(&m).is_ok() {
            return true;
        }
        self.manager.playout_n(100);
        self.manager.advance(&m).is_ok()
    }

    pub fn reset(&mut self) {
        self.manager = fresh(&self.initial);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Board is comma-joined heaps; helper to read the pile total.
    fn total(g: &NimWasm) -> u32 {
        g.get_board()
            .split(',')
            .map(|s| s.parse::<u32>().unwrap())
            .sum()
    }

    #[test]
    fn heap_generation_marienbad() {
        // stones=7, heaps=4 → [7,5,3,1].
        let g = NimWasm::new(7, 4, 0, 1);
        assert_eq!(g.get_board(), "7,5,3,1");
    }

    #[test]
    fn heap_generation_floor_one() {
        // Descent floors at 1, never 0.
        let g = NimWasm::new(3, 5, 0, 0);
        assert_eq!(g.get_board(), "3,1,1,1,1");
    }

    #[test]
    fn clamps() {
        // stones 1..=60, heaps 1..=8, max_take capped at 10 (0 stays 0).
        let g = NimWasm::new(999, 99, 999, 0);
        assert_eq!(g.get_board().split(',').count(), 8);
        assert_eq!(g.get_board().split(',').next().unwrap(), "60");
    }

    #[test]
    fn single_pile_regression_legal_moves() {
        // heaps=1, max_take=2, misere=0 must reproduce the original toy: a
        // 15-stone pile whose only moves are "0-1"/"0-2".
        let g = NimWasm::new(15, 1, 2, 0);
        assert_eq!(g.get_board(), "15");
        assert_eq!(g.legal_moves(), "0-1,0-2");
    }

    #[test]
    fn single_pile_last_stone_only_take1() {
        let g = NimWasm::new(1, 1, 2, 0);
        assert_eq!(g.legal_moves(), "0-1");
    }

    #[test]
    fn max_take_bounds_moves() {
        // max_take=3 on a heap of 5: counts 1..=3 only.
        let g = NimWasm::new(5, 1, 3, 0);
        assert_eq!(g.legal_moves(), "0-1,0-2,0-3");
    }

    #[test]
    fn unbounded_take_whole_heap() {
        // max_take=0 → take up to the whole heap.
        let g = NimWasm::new(4, 1, 0, 0);
        assert_eq!(g.legal_moves(), "0-1,0-2,0-3,0-4");
    }

    #[test]
    fn move_round_trip() {
        // Every generated move Displays and parses back, and apply_move on a
        // clone accepts it (display/encode/decode single-sourced).
        let g = NimWasm::new(7, 4, 0, 1);
        let moves: Vec<&str> = "7,5,3,1".split(',').collect();
        assert_eq!(g.get_board().split(',').collect::<Vec<_>>(), moves);
        for tok in g.legal_moves().split(',') {
            let parsed: NimMove = tok.parse().expect("token parses");
            assert_eq!(parsed.to_string(), tok, "round-trips to the same token");
            let mut clone = NimWasm::new(7, 4, 0, 1);
            assert!(clone.apply_move(tok), "apply_move accepts generated token {tok}");
        }
    }

    #[test]
    fn normal_terminal_taker_wins() {
        // Single pile of 2, normal play: P1 takes both, P1 (seat 1) wins.
        let mut g = NimWasm::new(2, 1, 2, 0);
        assert!(g.apply_move("0-2"));
        assert!(g.is_terminal());
        assert_eq!(g.result(), "1", "taker of the last stone wins in normal play");
    }

    #[test]
    fn misere_terminal_taker_loses() {
        // Same line, misère: P1 is forced to take the last, so P2 (seat 2) wins.
        let mut g = NimWasm::new(2, 1, 2, 1);
        assert!(g.apply_move("0-2"));
        assert!(g.is_terminal());
        assert_eq!(g.result(), "2", "taker of the last stone loses in misère");
    }

    #[test]
    fn apply_move_fallback_path() {
        // Fresh manager, no playouts: advance() falls back to playout_n(100)
        // then advances. Must succeed and shrink the pile.
        let mut g = NimWasm::new(5, 1, 2, 0);
        assert!(g.apply_move("0-2"));
        assert_eq!(g.get_board(), "3");
        assert_eq!(g.current_player(), "P2");
    }

    #[test]
    fn apply_move_fast_path_after_search() {
        let mut g = NimWasm::new(4, 1, 2, 0);
        g.playout_n(500);
        assert!(g.apply_move("0-1"));
        assert_eq!(g.get_board(), "3");
        assert_eq!(g.current_player(), "P2");
    }

    #[test]
    fn play_single_pile_to_terminal() {
        let mut g = NimWasm::new(3, 1, 2, 0);
        assert!(g.apply_move("0-1")); // 2 left
        assert!(g.apply_move("0-2")); // 0 left
        assert!(g.is_terminal());
        assert!(!g.apply_move("0-1"), "no legal move once the pile is empty");
        assert_eq!(total(&g), 0);
    }

    #[test]
    fn rejects_illegal_and_malformed_moves() {
        let mut g = NimWasm::new(5, 1, 2, 0);
        assert!(!g.apply_move("0-3"), "count beyond max_take is illegal");
        assert!(!g.apply_move("1-1"), "no heap index 1 in a single-heap game");
        assert!(!g.apply_move("Take1"), "old move format no longer parses");
        assert!(!g.apply_move("garbage"));
        assert_eq!(g.get_board(), "5");
    }

    #[test]
    fn solver_proves_single_pile() {
        // Nim is the flagship solver demo. With take 1–2, multiples of 3 are the
        // losing (P) positions. On a pile of 4 the player to move is winning
        // (take 1, leaving 3 for the opponent). The solver should prove a Win;
        // on a pile of 3 it should prove a Loss.
        let mut win = NimWasm::new(4, 1, 2, 0);
        win.playout_n(2000);
        assert_eq!(win.root_proven_value(), "Win");
        let mut loss = NimWasm::new(3, 1, 2, 0);
        loss.playout_n(2000);
        assert_eq!(loss.root_proven_value(), "Loss");
    }

    #[test]
    fn misere_single_pile_verdict() {
        // Misère pile of 1, one stone, take 1: P1 forced to take it and LOSES,
        // so the played-out terminal names P2 (seat 2) the winner.
        let mut g = NimWasm::new(1, 1, 2, 1);
        assert!(g.apply_move("0-1"));
        assert!(g.is_terminal());
        assert_eq!(g.result(), "2");
    }
}
