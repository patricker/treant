# treant-wasm

WebAssembly bindings for the [`treant`](https://crates.io/crates/treant) Monte Carlo Tree Search library. Compiles to a browser-loadable WASM module powering the [interactive playground](https://mcts.dev/playground) with Tic-Tac-Toe, Connect Four, Nim, 2048, and custom games.

Built with [`wasm-bindgen`](https://rustwasm.github.io/wasm-bindgen/). Intended as a reference implementation for embedding `treant` in the browser — not a general-purpose crate.

## Build

```bash
cd treant-wasm
wasm-pack build --target web
```

Output lands in `treant-wasm/pkg/`.

- [Playground](https://mcts.dev/playground)
- [Documentation](https://mcts.dev)
- [Source and issues](https://github.com/patricker/treant)

## License

MIT
