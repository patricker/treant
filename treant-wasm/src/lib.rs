use wasm_bindgen::prelude::*;

mod amazons;
mod baghchal;
mod capturego;
mod chomp;
mod climb;
mod clobber;
mod col;
mod connectfour;
mod counting;
mod dice;
mod difficulty;
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
mod ninemorris;
mod nogo;
mod oware;
mod pig;
mod pinchfive;
mod prior;
mod quadline;
mod reversi;
mod shift;
mod sim;
mod subtractsquare;
mod tictactoe;
mod trails;
mod treblecross;
mod types;
mod worldthrees;
mod wythoff;
mod y;

pub use amazons::AmazonsWasm;
pub use baghchal::BaghchalWasm;
pub use capturego::CaptureGoWasm;
pub use chomp::ChompWasm;
pub use climb::ClimbWasm;
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
pub use ninemorris::NineMorrisWasm;
pub use nogo::NoGoWasm;
pub use oware::OwareWasm;
pub use pig::PigWasm;
pub use pinchfive::PinchFiveWasm;
pub use prior::{PriorGamePuctWasm, PriorGameUctWasm};
pub use quadline::QuadlineWasm;
pub use reversi::ReversiWasm;
pub use shift::ShiftWasm;
pub use sim::SimWasm;
pub use subtractsquare::SubtractSquareWasm;
pub use tictactoe::TicTacToeWasm;
pub use trails::TrailsWasm;
pub use treblecross::TreblecrossWasm;
pub use worldthrees::WorldThreesWasm;
pub use wythoff::WythoffWasm;
pub use y::YGameWasm;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn ping() -> String {
    "treant-wasm ready".into()
}
