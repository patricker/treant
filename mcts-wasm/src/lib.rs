use wasm_bindgen::prelude::*;

mod types;
mod counting;
mod nim;
mod dice;
mod prior;

pub use counting::CountingGameWasm;
pub use nim::NimWasm;
pub use dice::DiceGameWasm;
pub use prior::{PriorGameUctWasm, PriorGamePuctWasm};

#[wasm_bindgen]
pub fn ping() -> String {
    "mcts-wasm ready".into()
}
