use wasm_bindgen::prelude::*;

mod amazons;
mod capturego;
mod chomp;
mod clobber;
mod col;
mod connectfour;
mod counting;
mod dice;
mod domineering;
mod dotsboxes;
mod euclid;
mod foxhounds;
mod frontline;
mod game2048;
mod gridlib;
mod gridpack;
mod hex;
mod konane;
pub mod mancala;
mod mutorere;
mod nim;
mod nogo;
mod pig;
mod prior;
mod reversi;
mod shift;
mod sim;
mod subtractsquare;
mod tictactoe;
mod trails;
mod treblecross;
mod types;
mod wythoff;

pub use amazons::AmazonsWasm;
pub use capturego::CaptureGoWasm;
pub use chomp::ChompWasm;
pub use clobber::ClobberWasm;
pub use col::ColWasm;
pub use connectfour::ConnectFourWasm;
pub use counting::CountingGameWasm;
pub use dice::DiceGameWasm;
pub use domineering::DomineeringWasm;
pub use dotsboxes::DotsBoxesWasm;
pub use euclid::EuclidWasm;
pub use foxhounds::FoxHoundsWasm;
pub use frontline::FrontlineWasm;
pub use game2048::Game2048Wasm;
pub use gridpack::{Connect6Wasm, NotaktoWasm, OrderChaosWasm, SquareUpWasm, SquavaWasm};
pub use hex::HexWasm;
pub use konane::KonaneWasm;
pub use mancala::MancalaWasm;
pub use mutorere::MuTorereWasm;
pub use nim::NimWasm;
pub use nogo::NoGoWasm;
pub use pig::PigWasm;
pub use prior::{PriorGamePuctWasm, PriorGameUctWasm};
pub use reversi::ReversiWasm;
pub use shift::ShiftWasm;
pub use sim::SimWasm;
pub use subtractsquare::SubtractSquareWasm;
pub use tictactoe::TicTacToeWasm;
pub use trails::TrailsWasm;
pub use treblecross::TreblecrossWasm;
pub use wythoff::WythoffWasm;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn ping() -> String {
    "treant-wasm ready".into()
}
