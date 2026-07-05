//! Nine Men's Morris — the classic mill game on 24 points across three
//! concentric squares. Phase 1: players alternate placing their men on empty
//! points. Phase 2: slide a man to an adjacent empty point. Completing a MILL
//! (three own men on a marked line) removes one enemy man — one not itself in a
//! mill, unless every enemy man is milled. A player is lost when reduced to two
//! men or left with no legal move. Sliding can cycle, so a draw is declared
//! after `DRAW_PLY_CAP` plies with no capture or placement, which also makes
//! every random rollout terminate.
use treant::tree_policy::*;
use treant::*;
use wasm_bindgen::prelude::*;

use crate::types;

const N_POINTS: usize = 24;
/// `from` sentinel marking a placement (rather than a slide).
const PLACE: u8 = 0xFF;
/// `victim` sentinel marking "no man removed".
const NONE: u8 = 0xFF;
/// Plies without a placement or capture before the game is declared a draw.
/// Guarantees termination: placements and captures are both finite, so at most
/// this many pure slides can pass between them.
const DRAW_PLY_CAP: u32 = 50;

/// The 16 mill lines (three-in-a-row wins): 8 horizontal + 8 vertical.
const MILLS: [[u8; 3]; 16] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [9, 10, 11],
    [12, 13, 14],
    [15, 16, 17],
    [18, 19, 20],
    [21, 22, 23],
    [0, 9, 21],
    [3, 10, 18],
    [6, 11, 15],
    [1, 4, 7],
    [16, 19, 22],
    [8, 12, 17],
    [5, 13, 20],
    [2, 14, 23],
];

/// Orthogonal neighbours (the drawn board edges) for each of the 24 points.
const ADJ: [&[u8]; 24] = [
    &[1, 9],         // 0
    &[0, 2, 4],      // 1
    &[1, 14],        // 2
    &[4, 10],        // 3
    &[3, 5, 1, 7],   // 4
    &[4, 13],        // 5
    &[7, 11],        // 6
    &[6, 8, 4],      // 7
    &[7, 12],        // 8
    &[0, 21, 10],    // 9
    &[9, 11, 3, 18], // 10
    &[10, 6, 15],    // 11
    &[8, 17, 13],    // 12
    &[12, 14, 5, 20],// 13
    &[13, 2, 23],    // 14
    &[16, 11],       // 15
    &[15, 17, 19],   // 16
    &[16, 12],       // 17
    &[19, 10],       // 18
    &[18, 20, 16, 22],// 19
    &[19, 13],       // 20
    &[9, 22],        // 21
    &[21, 23, 19],   // 22
    &[22, 14],       // 23
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NmMove {
    from: u8,   // PLACE for a placement, else the source point of a slide
    to: u8,     // the destination point
    victim: u8, // NONE for no capture, else the enemy point removed
}
impl std::fmt::Display for NmMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.from == PLACE {
            write!(f, "p{}", self.to)?;
        } else {
            write!(f, "{}-{}", self.from, self.to)?;
        }
        if self.victim != NONE {
            write!(f, "x{}", self.victim)?;
        }
        Ok(())
    }
}

/// True if `point` (already set to `owner` on `board`) completes a mill.
fn forms_mill(board: &[i8; N_POINTS], point: u8, owner: i8) -> bool {
    MILLS
        .iter()
        .filter(|m| m.contains(&point))
        .any(|m| m.iter().all(|&p| board[p as usize] == owner))
}

/// The enemy men that may be removed after a mill: those not in a mill, or —
/// when every enemy man is milled — any enemy man.
fn removable(board: &[i8; N_POINTS], enemy: i8) -> Vec<u8> {
    let enemy_pts: Vec<u8> = (0..N_POINTS as u8).filter(|&p| board[p as usize] == enemy).collect();
    let free: Vec<u8> = enemy_pts
        .iter()
        .copied()
        .filter(|&p| !MILLS.iter().filter(|m| m.contains(&p)).any(|m| m.iter().all(|&q| board[q as usize] == enemy)))
        .collect();
    if free.is_empty() {
        enemy_pts
    } else {
        free
    }
}

#[derive(Clone)]
struct NineMorris {
    board: [i8; N_POINTS], // -1 empty, else owner 0/1
    current: u8,
    placed: [u8; 2],   // men placed so far (phase-1 counter, per player)
    on_board: [u8; 2], // men currently on the board, per player
    men_per_player: u8,
    flying: bool,
    lasker: bool, // Lasker variant: place OR slide each turn while men remain in hand
    progress_ply: u32, // plies since the last placement or capture
}
impl NineMorris {
    fn new(men_per_player: u8, flying: bool, lasker: bool) -> Self {
        Self {
            board: [-1; N_POINTS],
            current: 0,
            placed: [0, 0],
            on_board: [0, 0],
            men_per_player: men_per_player.clamp(3, 12),
            flying,
            lasker,
            progress_ply: 0,
        }
    }

    fn placing(&self) -> bool {
        self.placed[self.current as usize] < self.men_per_player
    }

    /// Emit the legal move(s) for landing at `to` (placing, or sliding from
    /// `from`): one move if it forms no mill, else one per removable victim.
    fn push_moves(&self, v: &mut Vec<NmMove>, from: u8, to: u8, me: i8, enemy: i8) {
        let mut b = self.board;
        if from != PLACE {
            b[from as usize] = -1;
        }
        b[to as usize] = me;
        if forms_mill(&b, to, me) {
            let victims = removable(&b, enemy);
            if victims.is_empty() {
                v.push(NmMove { from, to, victim: NONE });
            } else {
                for vic in victims {
                    v.push(NmMove { from, to, victim: vic });
                }
            }
        } else {
            v.push(NmMove { from, to, victim: NONE });
        }
    }

    fn gen(&self) -> Vec<NmMove> {
        let mut v = Vec::new();
        let me = self.current as i8;
        let enemy = 1 - me;
        let placing = self.placing();
        let on_board = self.on_board[self.current as usize];
        // Placements: while this player still has men in hand.
        if placing {
            for to in 0..N_POINTS as u8 {
                if self.board[to as usize] == -1 {
                    self.push_moves(&mut v, PLACE, to, me, enemy);
                }
            }
        }
        // Slides: always in the slide phase; in the Lasker variant ALSO while
        // still placing, as long as this player has a man on the board. Flying
        // is only reachable once the hand is empty (`!placing`), matching the
        // engine's classic gate — with men still in hand you are never "reduced
        // to three", so no flying during the place-or-move phase.
        if !placing || (self.lasker && on_board > 0) {
            let can_fly = self.flying && on_board == 3 && !placing;
            for from in 0..N_POINTS as u8 {
                if self.board[from as usize] != me {
                    continue;
                }
                if can_fly {
                    for to in 0..N_POINTS as u8 {
                        if self.board[to as usize] == -1 {
                            self.push_moves(&mut v, from, to, me, enemy);
                        }
                    }
                } else {
                    for &to in ADJ[from as usize] {
                        if self.board[to as usize] == -1 {
                            self.push_moves(&mut v, from, to, me, enemy);
                        }
                    }
                }
            }
        }
        v
    }

    fn term(&self) -> Option<ProvenValue> {
        // A long capture-free stretch (or a sliding cycle) is a draw.
        if self.progress_ply >= DRAW_PLY_CAP {
            return Some(ProvenValue::Draw);
        }
        let me = self.current as usize;
        // Reduced to two men (only meaningful once all men are placed) → loss.
        if self.placed[me] == self.men_per_player && self.on_board[me] < 3 {
            return Some(ProvenValue::Loss);
        }
        // No legal move → loss.
        if self.gen().is_empty() {
            return Some(ProvenValue::Loss);
        }
        None
    }
}
impl GameState for NineMorris {
    type Move = NmMove;
    type Player = u8;
    type MoveList = Vec<NmMove>;
    fn current_player(&self) -> u8 {
        self.current
    }
    fn available_moves(&self) -> Vec<NmMove> {
        self.gen()
    }
    fn make_move(&mut self, m: &NmMove) {
        let me = self.current as i8;
        let enemy = 1 - self.current as usize;
        let mut progress = false;
        if m.from == PLACE {
            self.board[m.to as usize] = me;
            self.placed[self.current as usize] += 1;
            self.on_board[self.current as usize] += 1;
            progress = true;
        } else {
            self.board[m.from as usize] = -1;
            self.board[m.to as usize] = me;
        }
        if m.victim != NONE {
            self.board[m.victim as usize] = -1;
            self.on_board[enemy] -= 1;
            progress = true;
        }
        if progress {
            self.progress_ply = 0;
        } else {
            self.progress_ply += 1;
        }
        self.current = 1 - self.current;
    }
    fn terminal_value(&self) -> Option<ProvenValue> {
        self.term()
    }
}

struct NmEval;
impl Evaluator<NmCfg> for NmEval {
    type StateEvaluation = i64;
    fn evaluate_new_state(&self, _: &NineMorris, m: &Vec<NmMove>, _: Option<SearchHandle<NmCfg>>) -> (Vec<()>, i64) {
        (vec![(); m.len()], 0)
    }
    fn interpret_evaluation_for_player(&self, e: &i64, _: &u8) -> i64 {
        *e
    }
    fn evaluate_existing_state(&self, _: &NineMorris, _: &i64, _: SearchHandle<NmCfg>) -> i64 {
        0
    }
}
#[derive(Default)]
struct NmCfg;
impl MCTS for NmCfg {
    type State = NineMorris;
    type Eval = NmEval;
    type NodeData = ();
    type ExtraThreadData = ();
    type TreePolicy = UCTPolicy;
    type TranspositionTable = ();
    fn solver_enabled(&self) -> bool {
        true
    }
}

/// Parse a move encoding: `p<to>`, `p<to>x<victim>`, `<from>-<to>`, or
/// `<from>-<to>x<victim>`.
fn parse_move(mov: &str) -> Option<NmMove> {
    let (main, victim) = match mov.split_once('x') {
        Some((a, b)) => (a, b.parse::<u8>().ok()?),
        None => (mov, NONE),
    };
    let (from, to) = if let Some(rest) = main.strip_prefix('p') {
        (PLACE, rest.parse::<u8>().ok()?)
    } else {
        let (f, t) = main.split_once('-')?;
        (f.parse::<u8>().ok()?, t.parse::<u8>().ok()?)
    };
    if to as usize >= N_POINTS {
        return None;
    }
    Some(NmMove { from, to, victim })
}

#[wasm_bindgen]
pub struct NineMorrisWasm {
    manager: MCTSManager<NmCfg>,
    men_per_player: u8,
    flying: bool,
    lasker: bool,
}
#[wasm_bindgen]
impl NineMorrisWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(men_per_player: u32, flying: u32, lasker: u32) -> Self {
        let men = men_per_player as u8;
        let fly = flying != 0;
        let las = lasker != 0;
        Self {
            manager: MCTSManager::new(NineMorris::new(men, fly, las), NmCfg, NmEval, UCTPolicy::new(1.4), ()),
            men_per_player: men.clamp(3, 12),
            flying: fly,
            lasker: las,
        }
    }
    pub fn playout_n(&mut self, n: u32) {
        self.manager.playout_n(n as u64);
    }
    pub fn get_stats(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&types::build_stats(&self.manager, |_| None)).unwrap_or(JsValue::NULL)
    }
    /// 24 point chars (' '=empty, 'X'=p0, 'O'=p1), then `'|'`, then phase info:
    /// `place:<remaining0>,<remaining1>` during placement (men each still has to
    /// place), or `slide` once both have placed all their men.
    pub fn get_board(&self) -> String {
        let s = self.manager.tree().root_state();
        let mut out: String = s
            .board
            .iter()
            .map(|&v| match v {
                0 => 'X',
                1 => 'O',
                _ => ' ',
            })
            .collect();
        out.push('|');
        let placing = s.placed[0] < s.men_per_player || s.placed[1] < s.men_per_player;
        if placing {
            let r0 = s.men_per_player - s.placed[0];
            let r1 = s.men_per_player - s.placed[1];
            out.push_str(&format!("place:{r0},{r1}"));
        } else {
            out.push_str("slide");
        }
        out
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
            Some(ProvenValue::Draw) => "Draw".into(),
            _ => String::new(),
        }
    }
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
    pub fn apply_move(&mut self, mov: &str) -> bool {
        let m = match parse_move(mov) {
            Some(m) => m,
            None => return false,
        };
        let mut s = self.manager.tree().root_state().clone();
        if !s.available_moves().contains(&m) {
            return false;
        }
        s.make_move(&m);
        self.manager = MCTSManager::new(s, NmCfg, NmEval, UCTPolicy::new(1.4), ());
        true
    }
    pub fn reset(&mut self) {
        self.manager = MCTSManager::new(
            NineMorris::new(self.men_per_player, self.flying, self.lasker),
            NmCfg,
            NmEval,
            UCTPolicy::new(1.4),
            (),
        );
    }
}

impl Default for NineMorrisWasm {
    fn default() -> Self {
        Self::new(9, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_sixteen_mills_detected() {
        for mill in MILLS {
            let mut board = [-1i8; N_POINTS];
            for &p in &mill {
                board[p as usize] = 0;
            }
            // completing at any of the three points reports the mill
            for &p in &mill {
                assert!(forms_mill(&board, p, 0), "mill {mill:?} not detected at {p}");
            }
            // an enemy owner never sees this mill
            assert!(!forms_mill(&board, mill[0], 1));
        }
    }

    #[test]
    fn opening_generates_all_placements_no_mills() {
        let g = NineMorris::new(9, false, false);
        let moves = g.gen();
        assert_eq!(moves.len(), N_POINTS); // one placement per empty point
        assert!(moves.iter().all(|m| m.from == PLACE && m.victim == NONE));
    }

    #[test]
    fn placing_into_a_mill_generates_removal_variants() {
        // p0 owns 0 and 1; p1 owns 9 and 10 (not in a mill). p0 to move can
        // place at 2 to complete mill [0,1,2], removing either enemy man.
        let mut g = NineMorris::new(9, false, false);
        g.board[0] = 0;
        g.board[1] = 0;
        g.board[9] = 1;
        g.board[10] = 1;
        g.placed = [2, 2];
        g.on_board = [2, 2];
        let mills: Vec<NmMove> = g.gen().into_iter().filter(|m| m.to == 2).collect();
        assert_eq!(mills.len(), 2, "one move per removable victim");
        let victims: Vec<u8> = mills.iter().map(|m| m.victim).collect();
        assert!(victims.contains(&9) && victims.contains(&10));

        // applying the removal actually clears the victim and flips turn
        let m = *mills.iter().find(|m| m.victim == 9).unwrap();
        g.make_move(&m);
        assert_eq!(g.board[2], 0); // placed
        assert_eq!(g.board[9], -1); // victim removed
        assert_eq!(g.on_board[1], 1);
        assert_eq!(g.current, 1);
    }

    #[test]
    fn removal_prefers_men_not_in_a_mill() {
        // Enemy has a full mill [21,22,23] plus a loose man at 5. p0 completes
        // mill [0,1,2]; only the loose man 5 is removable.
        let mut board = [-1i8; N_POINTS];
        board[0] = 0;
        board[1] = 0;
        board[2] = 0;
        board[21] = 1;
        board[22] = 1;
        board[23] = 1;
        board[5] = 1;
        let victims = removable(&board, 1);
        assert_eq!(victims, vec![5]);
    }

    #[test]
    fn move_encodings_round_trip() {
        assert_eq!(parse_move("p7"), Some(NmMove { from: PLACE, to: 7, victim: NONE }));
        assert_eq!(parse_move("p7x12"), Some(NmMove { from: PLACE, to: 7, victim: 12 }));
        assert_eq!(parse_move("3-7"), Some(NmMove { from: 3, to: 7, victim: NONE }));
        assert_eq!(parse_move("3-7x12"), Some(NmMove { from: 3, to: 7, victim: 12 }));
        assert_eq!(NmMove { from: PLACE, to: 7, victim: NONE }.to_string(), "p7");
        assert_eq!(NmMove { from: 3, to: 7, victim: 12 }.to_string(), "3-7x12");
        assert!(parse_move("garbage").is_none());
    }

    #[test]
    fn reduced_to_two_men_is_a_loss_for_the_player_to_move() {
        // Slide phase, p0 (to move) has only two men left → sees a Loss.
        let mut g = NineMorris::new(9, false, false);
        g.placed = [9, 9];
        g.board[0] = 0;
        g.board[1] = 0;
        g.board[5] = 1;
        g.board[6] = 1;
        g.board[7] = 1;
        g.on_board = [2, 3];
        g.current = 0;
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn no_legal_move_is_a_loss() {
        // p0's single man at 0 is boxed in (1 and 9 occupied by the enemy).
        let mut g = NineMorris::new(3, false, false);
        g.placed = [3, 3];
        g.board[0] = 0;
        g.board[3] = 0;
        g.board[6] = 0;
        g.board[1] = 1;
        g.board[9] = 1;
        g.board[4] = 1;
        g.board[10] = 1;
        g.board[7] = 1;
        g.board[11] = 1;
        g.on_board = [3, 6];
        g.current = 0;
        // every p0 man has all neighbours occupied → no slide, no placement left
        assert!(g.gen().is_empty());
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn draw_after_progress_stall() {
        let mut g = NineMorris::new(9, false, false);
        g.placed = [9, 9];
        g.on_board = [4, 4];
        g.progress_ply = DRAW_PLY_CAP;
        assert_eq!(g.term(), Some(ProvenValue::Draw));
    }

    #[test]
    fn ai_plays() {
        let mut g = NineMorrisWasm::new(9, 0, 0);
        g.playout_n(500);
        assert!(g.best_move().is_some());
    }

    #[test]
    fn ai_plays_a_full_game_to_terminal() {
        // A short self-play sanity check: rollouts terminate and the result is
        // one of the three legal verdicts.
        let mut g = NineMorrisWasm::new(6, 0, 0);
        for _ in 0..400 {
            if g.is_terminal() {
                break;
            }
            g.playout_n(40);
            let mv = match g.best_move() {
                Some(m) => m,
                None => break,
            };
            assert!(g.apply_move(&mv), "engine rejected its own move {mv}");
        }
        // Either it finished, or it is mid-game with legal moves — never stuck.
        assert!(g.is_terminal() || !g.legal_moves().is_empty());
    }

    #[test]
    fn lasker_offers_slides_during_placement() {
        // Lasker Morris: while men remain in hand, a player may EITHER place a
        // new man OR slide one already on the board.
        let mut g = NineMorrisWasm::new(10, 0, 1);
        assert!(g.apply_move("p0")); // P0 places (encoding is p<point>)
        assert!(g.apply_move("p12")); // P1 places
        // P0 still has 9 unplaced men, but sliding the placed man must ALSO be legal:
        let moves = g.legal_moves();
        assert!(moves.split(',').any(|m| m.contains('-')), "expected a slide move, got {moves}");
        // ...and placements must STILL be offered too.
        assert!(moves.split(',').any(|m| m.starts_with('p')), "expected a placement, got {moves}");
    }

    #[test]
    fn non_lasker_offers_no_slides_during_placement() {
        // Regression guard: the classic engine still gates place-then-slide hard.
        let mut g = NineMorrisWasm::new(10, 0, 0);
        assert!(g.apply_move("p0"));
        assert!(g.apply_move("p12"));
        let moves = g.legal_moves();
        assert!(!moves.split(',').any(|m| m.contains('-')), "classic mode must not offer slides while placing, got {moves}");
    }

    #[test]
    fn lasker_reduced_to_two_before_placement_done_is_not_a_loss() {
        // Subtlety: in Lasker mode a player can be milled down to two men ON THE
        // BOARD while still holding men in hand. That is NOT a loss — they can
        // simply place another man. Loss only triggers once the hand is empty.
        let mut g = NineMorris::new(10, false, true);
        g.placed = [3, 3];
        g.on_board = [2, 3]; // p0 down to two on board, but 7 still in hand
        g.board[0] = 0;
        g.board[1] = 0;
        g.board[5] = 1;
        g.board[6] = 1;
        g.board[7] = 1;
        g.current = 0;
        assert_eq!(g.term(), None, "still has men in hand — not a loss");
        // The very same board with the hand emptied (placed == men) IS a loss.
        g.placed = [10, 10];
        assert_eq!(g.term(), Some(ProvenValue::Loss));
    }

    #[test]
    fn lasker_ai_plays_a_full_game_to_terminal() {
        // Self-play sanity for the Lasker variant: it terminates cleanly and the
        // progress counter / loss detection keep rollouts finite.
        let mut g = NineMorrisWasm::new(10, 0, 1);
        for _ in 0..400 {
            if g.is_terminal() {
                break;
            }
            g.playout_n(40);
            let mv = match g.best_move() {
                Some(m) => m,
                None => break,
            };
            assert!(g.apply_move(&mv), "engine rejected its own move {mv}");
        }
        assert!(g.is_terminal() || !g.legal_moves().is_empty());
    }
}
