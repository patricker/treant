#[test]
fn every_game_weak_move_returns_something_or_terminal() {
    use treant_wasm::*;
    macro_rules! ck {
        ($g:expr) => {{
            let mut g = $g;
            assert!(g.weak_move(50, 5, 1.5, 1).is_some(), "weak_move returned None at start");
        }};
    }
    ck!(ConnectFourWasm::new(7, 6, 4, 2, 0));
    ck!(ConnectFourWasm::new(7, 6, 4, 2, 1)); // Pop Out variant
    ck!(ConnectFourWasm::new(12, 6, 4, 2, 2)); // Cylinder variant
    ck!(TicTacToeWasm::new(3, 3, 3, 2));
    ck!(ReversiWasm::new(6, 6, 0));
    ck!(ReversiWasm::new(6, 6, 1)); // Anti-Reversi variant
    ck!(HexWasm::new(7, 1));
    ck!(FrontlineWasm::new(6, 6));
    ck!(MancalaWasm::new(6, 4, 2));
    ck!(DotsBoxesWasm::new(3, 3));
    ck!(KonaneWasm::new(6, 6));
    ck!(DomineeringWasm::new(6, 6, 0));
    ck!(NimWasm::new(15));
}
