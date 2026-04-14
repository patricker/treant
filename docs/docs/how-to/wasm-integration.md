---
sidebar_position: 7
id: wasm-integration
---

# Run MCTS in the Browser

Compile your MCTS game to WebAssembly and call it from JavaScript.

**You will learn to:**
- Build an MCTS game as a WASM module with `wasm-pack`
- Call the search from JavaScript without blocking the UI

**Prerequisites:** Complete [Your First Search](../tutorials/02-first-search). Install [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).

## Set up the crate

Create a new crate for the WASM bindings:

```bash
cargo new treant-wasm --lib
```

Configure `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
mcts = { path = "../" }
wasm-bindgen = "0.2"
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
getrandom = { version = "0.2", features = ["js"] }
```

The `getrandom` `js` feature is required because MCTS uses random number generation internally, and the default `getrandom` backend does not work in browsers.

## Wrap your game

Expose a `#[wasm_bindgen]` struct that owns the `MCTSManager` and provides methods for JavaScript:

```rust
use wasm_bindgen::prelude::*;
use treant::*;
use treant::tree_policy::UCTPolicy;

// Your GameState, Evaluator, MCTS config (defined elsewhere)
use crate::game::{MyGame, MyEval, MyMCTS};

#[wasm_bindgen]
pub struct GameSearch {
    manager: MCTSManager<MyMCTS>,
}

#[wasm_bindgen]
impl GameSearch {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let state = MyGame::initial();
        let manager = MCTSManager::new(
            state,
            MyMCTS,
            MyEval,
            UCTPolicy::new(1.41),
            (),
        );
        Self { manager }
    }

    /// Run n playouts (single-threaded in WASM).
    pub fn search(&mut self, n: u32) {
        for _ in 0..n {
            self.manager.playout();
        }
    }

    /// Return the best move as a JSON string.
    pub fn best_move(&self) -> String {
        match self.manager.best_move() {
            Some(m) => format!("{:?}", m),
            None => "null".into(),
        }
    }

    /// Apply a move and re-root the tree.
    pub fn play(&mut self, move_str: &str) {
        let mov = parse_move(move_str);
        let _ = self.manager.advance(&mov);
    }

    /// Total nodes in the search tree.
    pub fn node_count(&self) -> usize {
        self.manager.tree().num_nodes()
    }
}
```

Keep the WASM interface thin. Parse and serialize at the boundary; keep all game logic in pure Rust.

## Build

```bash
wasm-pack build --target web --release
```

This produces a `pkg/` directory containing the `.wasm` binary and a JavaScript wrapper module.

## Import in JavaScript

```javascript
import init, { GameSearch } from './pkg/mcts_wasm.js';

async function main() {
    await init();

    const game = new GameSearch();
    game.search(10000);
    console.log("Best move:", game.best_move());

    game.play("Add");
    game.search(10000);
    console.log("Best move:", game.best_move());
}

main();
```

## Avoid blocking the UI

WASM runs on the main thread. A search of 100,000 playouts can freeze the page for seconds. Break the work into chunks using `requestAnimationFrame`:

```javascript
function searchInChunks(game, totalPlayouts, chunkSize, onComplete) {
    let remaining = totalPlayouts;

    function step() {
        const batch = Math.min(remaining, chunkSize);
        game.search(batch);
        remaining -= batch;

        if (remaining > 0) {
            requestAnimationFrame(step);
        } else {
            onComplete(game.best_move());
        }
    }

    requestAnimationFrame(step);
}

// 50,000 playouts in chunks of 1,000
searchInChunks(game, 50000, 1000, (bestMove) => {
    console.log("Search complete:", bestMove);
});
```

A chunk size of 500-2000 playouts keeps each frame under 16ms for simple games.

## Constraints

- **Single-threaded only.** `playout_n_parallel` and `playout_parallel_async` require OS threads, which are not available in standard WASM. Use `playout()` or `playout_n()` in a loop instead.
- **No `SharedArrayBuffer` needed.** The single-threaded constraint means you do not need cross-origin isolation headers or shared memory.
- **Binary size.** A typical MCTS game compiles to 100-200KB of gzipped WASM. Use `wasm-opt -Oz` (included in `wasm-pack --release`) for smaller binaries.

## Expected result

Your game runs in any modern browser with no server-side computation. Performance is roughly 2-5x slower than native single-threaded Rust, which is fast enough for interactive demos and lightweight game AI.

## See also

- [Playground](/playground) -- this site's interactive demos use exactly this approach
