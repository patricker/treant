use wasm_bindgen::prelude::*;

mod counting;
mod dice;
mod nim;
mod prior;
mod types;

pub use counting::CountingGameWasm;
pub use dice::DiceGameWasm;
pub use nim::NimWasm;
pub use prior::{PriorGamePuctWasm, PriorGameUctWasm};

#[wasm_bindgen]
pub fn ping() -> String {
    "mcts-wasm ready".into()
}
