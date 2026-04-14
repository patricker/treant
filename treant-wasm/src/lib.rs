use wasm_bindgen::prelude::*;

mod connectfour;
mod counting;
mod dice;
mod game2048;
mod nim;
mod prior;
mod shift;
mod tictactoe;
mod types;

pub use connectfour::ConnectFourWasm;
pub use counting::CountingGameWasm;
pub use dice::DiceGameWasm;
pub use game2048::Game2048Wasm;
pub use nim::NimWasm;
pub use prior::{PriorGamePuctWasm, PriorGameUctWasm};
pub use shift::ShiftWasm;
pub use tictactoe::TicTacToeWasm;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn ping() -> String {
    "treant-wasm ready".into()
}
