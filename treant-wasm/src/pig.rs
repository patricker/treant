//! Pig — the press-your-luck dice game. Roll to build a turn total; a 1 wipes
//! it and ends your turn; Hold to bank it. First to the target wins. Showcases
//! treant's chance nodes (each Roll branches over the six die faces).
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

#[derive(Clone, Debug, PartialEq)]
enum PigMove {
    Roll,
    Hold,
    Die(u8),
    /// Two-dice roll outcome (variants 1 and 2). Chance nodes enumerate these.
    Die2(u8, u8),
}
impl std::fmt::Display for PigMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PigMove::Roll => write!(f, "Roll"),
            PigMove::Hold => write!(f, "Hold"),
            PigMove::Die(v) => write!(f, "Die({v})"),
            PigMove::Die2(a, b) => write!(f, "Die2({a},{b})"),
        }
    }
}

#[derive(Clone, Debug)]
struct Pig {
    scores: Vec<u32>,
    current: u8,
    num_players: u8,
    turn_total: u32,
    pending: bool,
    last_roll: u8,
    /// Second die of the most recent two-dice roll (variants 1/2); 0 = none.
    last_roll2: u8,
    target: u32,
    /// 0 = classic single-die Pig, 1 = Two-Dice Pig, 2 = Big Pig.
    variant: u8,
}
impl Pig {
    fn new(num_players: u8, target: u32, variant: u8) -> Self {
        Self {
            scores: vec![0; num_players as usize],
            current: 0,
            num_players,
            turn_total: 0,
            pending: false,
            last_roll: 0,
            last_roll2: 0,
            target,
            variant,
        }
    }
    fn winner(&self) -> Option<u8> {
        self.scores.iter().position(|&s| s >= self.target).map(|i| i as u8)
    }
    fn advance(&mut self) {
        self.turn_total = 0;
        self.current = (self.current + 1) % self.num_players;
    }
}
impl GameState for Pig {
    type Move = PigMove;
    type Player = u8;
    type MoveList = Vec<PigMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<PigMove> {
        if self.winner().is_some() || self.pending {
            vec![]
        } else {
            vec![PigMove::Roll, PigMove::Hold]
        }
    }
    fn make_move(&mut self, m: &PigMove) {
        match m {
            PigMove::Roll => self.pending = true,
            PigMove::Hold => {
                self.scores[self.current as usize] += self.turn_total;
                self.advance();
            }
            PigMove::Die(v) => {
                self.pending = false;
                self.last_roll = *v;
                self.last_roll2 = 0;
                if *v == 1 {
                    self.advance();
                } else {
                    self.turn_total += *v as u32;
                }
            }
            PigMove::Die2(a, b) => {
                self.pending = false;
                self.last_roll = *a;
                self.last_roll2 = *b;
                let (a, b) = (*a, *b);
                let sum = (a + b) as u32;
                match self.variant {
                    1 => {
                        // Two-Dice Pig: any single 1 busts the turn total; snake
                        // eyes (double 1) additionally wipes the banked score.
                        if a == 1 && b == 1 {
                            self.scores[self.current as usize] = 0;
                            self.advance();
                        } else if a == 1 || b == 1 {
                            self.advance();
                        } else {
                            self.turn_total += sum;
                        }
                    }
                    _ => {
                        // Big Pig (variant 2): snake eyes pays a flat 25, other
                        // doubles pay double the sum, a single 1 busts the turn,
                        // and the bank is never wiped.
                        if a == 1 && b == 1 {
                            self.turn_total += 25;
                        } else if a == 1 || b == 1 {
                            self.advance();
                        } else if a == b {
                            self.turn_total += 2 * sum;
                        } else {
                            self.turn_total += sum;
                        }
                    }
                }
            }
        }
    }
    fn chance_outcomes(&self) -> Option<Vec<(PigMove, f64)>> {
        if !(self.pending && self.winner().is_none()) {
            return None;
        }
        if self.variant == 0 {
            Some((1..=6).map(|v| (PigMove::Die(v), 1.0 / 6.0)).collect())
        } else {
            // The 21 unordered two-die outcomes: doubles occur one way (1/36),
            // mixed pairs two ways (2/36). Weights sum to 6/36 + 30/36 = 1.
            let mut outs = Vec::with_capacity(21);
            for a in 1..=6u8 {
                for b in a..=6u8 {
                    let w = if a == b { 1.0 / 36.0 } else { 2.0 / 36.0 };
                    outs.push((PigMove::Die2(a, b), w));
                }
            }
            Some(outs)
        }
    }
}

struct PigEval;
#[derive(Clone, Debug)]
struct PigStateEval {
    scores: Vec<u32>,
    current: u8,
    turn: u32,
}
impl Evaluator<PigConfig> for PigEval {
    type StateEvaluation = PigStateEval;
    fn evaluate_new_state(&self, s: &Pig, m: &Vec<PigMove>, _: Option<SearchHandle<PigConfig>>) -> (Vec<()>, PigStateEval) {
        (vec![(); m.len()], PigStateEval { scores: s.scores.clone(), current: s.current, turn: s.turn_total })
    }
    fn interpret_evaluation_for_player(&self, e: &PigStateEval, p: &u8) -> i64 {
        let mine = e.scores[*p as usize] as i64 + if *p == e.current { e.turn as i64 } else { 0 };
        let best_other = e
            .scores
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != *p as usize)
            .map(|(_, &s)| s as i64)
            .max()
            .unwrap_or(0);
        mine - best_other
    }
    fn evaluate_existing_state(&self, s: &Pig, _: &PigStateEval, _: SearchHandle<PigConfig>) -> PigStateEval {
        PigStateEval { scores: s.scores.clone(), current: s.current, turn: s.turn_total }
    }
}
#[derive(Default)]
struct PigConfig;
impl MCTS for PigConfig {
    type State = Pig;
    type Eval = PigEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
}

#[wasm_bindgen]
pub struct PigWasm {
    manager: MCTSManager<PigConfig>,
    num_players: u8,
    target: u32,
    variant: u8,
    rng: SmallRng,
}

#[wasm_bindgen]
impl PigWasm {
    /// `variant`: 0 = classic single-die Pig, 1 = Two-Dice Pig, 2 = Big Pig.
    #[wasm_bindgen(constructor)]
    pub fn new(num_players: u32, target: u32, variant: u32) -> Self {
        let np = (num_players as u8).clamp(2, 6);
        let target = target.clamp(20, 200);
        let variant = (variant as u8).min(2);
        Self {
            manager: MCTSManager::new(Pig::new(np, target, variant), PigConfig, PigEval, UCTPolicy::new(0.7), ()),
            num_players: np,
            target,
            variant,
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn num_players(&self) -> u32 {
        self.num_players as u32
    }

    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }

    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }

    /// "score0,score1,...|turn_total|last_roll|target" for classic Pig; a fifth
    /// field "|last_roll2" (0 = none) is appended for the two-dice variants so
    /// the board can render both dice. Classic (variant 0) output is unchanged.
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        let scores = s.scores.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
        let base = format!("{}|{}|{}|{}", scores, s.turn_total, s.last_roll, s.target);
        if s.variant > 0 {
            format!("{}|{}", base, s.last_roll2)
        } else {
            base
        }
    }

    pub fn current_player(&self) -> u32 {
        self.manager.tree().root_state().current as u32
    }

    pub fn is_terminal(&self) -> bool {
        self.manager.tree().root_state().winner().is_some()
    }

    pub fn result(&self) -> String {
        self.manager.tree().root_state().winner().map(|w| format!("{}", w + 1)).unwrap_or_default()
    }

    pub fn best_move(&self) -> Option<String> {
        self.manager.best_move().map(|m| format!("{m}"))
    }

    pub fn weak_move(&mut self, playouts: u32, top_k: usize, temp: f64, seed: u32) -> Option<String> {
        crate::difficulty::pick_weak(&mut self.manager, playouts as u64, top_k, temp, seed)
    }

    pub fn apply_move(&mut self, mov: &str) -> bool {
        let mut s = self.manager.tree().root_state().clone();
        if s.winner().is_some() {
            return false;
        }
        match mov {
            "Roll" => {
                s.make_move(&PigMove::Roll);
                if self.variant > 0 {
                    let a = self.rng.gen_range(1..=6);
                    let b = self.rng.gen_range(1..=6);
                    s.make_move(&PigMove::Die2(a, b));
                } else {
                    let v = self.rng.gen_range(1..=6);
                    s.make_move(&PigMove::Die(v));
                }
            }
            "Hold" => s.make_move(&PigMove::Hold),
            _ => return false,
        }
        self.manager = MCTSManager::new(s, PigConfig, PigEval, UCTPolicy::new(0.7), ());
        true
    }

    pub fn reset(&mut self) {
        self.manager =
            MCTSManager::new(Pig::new(self.num_players, self.target, self.variant), PigConfig, PigEval, UCTPolicy::new(0.7), ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_banks_and_advances() {
        let mut g = Pig::new(2, 100, 0);
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die(4));
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die(5));
        assert_eq!(g.turn_total, 9);
        assert_eq!(g.current, 0);
        g.make_move(&PigMove::Hold);
        assert_eq!(g.scores[0], 9);
        assert_eq!(g.current, 1);
    }

    #[test]
    fn rolling_a_one_busts_the_turn() {
        let mut g = Pig::new(2, 100, 0);
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die(6));
        assert_eq!(g.turn_total, 6);
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die(1));
        assert_eq!(g.turn_total, 0);
        assert_eq!(g.current, 1, "turn passes on a 1");
        assert_eq!(g.scores[0], 0, "nothing banked");
    }

    #[test]
    fn reaching_target_wins() {
        let mut g = Pig::new(2, 20, 0);
        g.scores[0] = 15;
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die(5));
        g.make_move(&PigMove::Hold);
        assert_eq!(g.winner(), Some(0));
    }

    #[test]
    fn ai_picks_a_move() {
        let mut g = PigWasm::new(2, 100, 0);
        g.playout_n(500);
        let m = g.best_move().unwrap();
        assert!(m == "Roll" || m == "Hold");
    }

    // --- Two-Dice Pig (variant 1) --------------------------------------------

    #[test]
    fn two_dice_double_one_wipes_banked_score() {
        let mut g = Pig::new(2, 100, 1);
        g.scores[0] = 30;
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die2(1, 1));
        assert_eq!(g.scores[0], 0, "snake eyes wipes the whole banked score");
        assert_eq!(g.turn_total, 0, "turn total is lost too");
        assert_eq!(g.current, 1, "and the turn passes");
    }

    #[test]
    fn two_dice_single_one_busts_only_the_turn() {
        let mut g = Pig::new(2, 100, 1);
        g.scores[0] = 30;
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die2(1, 4));
        assert_eq!(g.scores[0], 30, "a single 1 leaves the bank untouched");
        assert_eq!(g.turn_total, 0, "but busts the turn total");
        assert_eq!(g.current, 1, "and passes the turn");
    }

    #[test]
    fn two_dice_no_one_adds_both_and_continues() {
        let mut g = Pig::new(2, 100, 1);
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die2(3, 4));
        assert_eq!(g.turn_total, 7, "both dice add to the turn total");
        assert_eq!(g.current, 0, "the turn continues");
    }

    // --- Big Pig (variant 2) -------------------------------------------------

    #[test]
    fn big_pig_double_one_scores_twenty_five() {
        let mut g = Pig::new(2, 100, 2);
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die2(1, 1));
        assert_eq!(g.turn_total, 25, "double 1 pays a flat 25");
        assert_eq!(g.current, 0, "and the turn continues");
    }

    #[test]
    fn big_pig_other_double_scores_double() {
        let mut g = Pig::new(2, 100, 2);
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die2(5, 5));
        assert_eq!(g.turn_total, 20, "two 5s = 10 doubled = 20");
        assert_eq!(g.current, 0, "the turn continues");
    }

    #[test]
    fn big_pig_single_one_busts_the_turn() {
        let mut g = Pig::new(2, 100, 2);
        g.scores[0] = 30;
        g.make_move(&PigMove::Roll);
        g.make_move(&PigMove::Die2(1, 6));
        assert_eq!(g.turn_total, 0, "a single 1 busts the turn");
        assert_eq!(g.scores[0], 30, "but never wipes the bank in Big Pig");
        assert_eq!(g.current, 1, "the turn passes");
    }

    // --- Chance-node distribution -------------------------------------------

    #[test]
    fn two_dice_chance_outcomes_are_21_weighted_pairs() {
        let mut g = Pig::new(2, 100, 1);
        g.make_move(&PigMove::Roll);
        let outs = g.chance_outcomes().unwrap();
        assert_eq!(outs.len(), 21, "21 unordered two-die outcomes");
        let total: f64 = outs.iter().map(|(_, w)| w).sum();
        assert!((total - 1.0).abs() < 1e-9, "weights sum to 1, got {total}");
        let doubles = outs.iter().filter(|(m, _)| matches!(m, PigMove::Die2(a, b) if a == b)).count();
        assert_eq!(doubles, 6, "six doubles");
        for (m, w) in &outs {
            if let PigMove::Die2(a, b) = m {
                let expected = if a == b { 1.0 / 36.0 } else { 2.0 / 36.0 };
                assert!((w - expected).abs() < 1e-9, "wrong weight for Die2({a},{b})");
            } else {
                panic!("variant-1 chance node emitted a non-Die2 move");
            }
        }
    }

    #[test]
    fn classic_chance_outcomes_are_six_uniform_faces() {
        let mut g = Pig::new(2, 100, 0);
        g.make_move(&PigMove::Roll);
        let outs = g.chance_outcomes().unwrap();
        assert_eq!(outs.len(), 6, "six single-die faces");
        assert!(outs.iter().all(|(m, w)| matches!(m, PigMove::Die(_)) && (w - 1.0 / 6.0).abs() < 1e-9));
    }
}
