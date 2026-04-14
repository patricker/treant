# treant-dynamic

Runtime-polymorphic adapter for the [`treant`](https://crates.io/crates/treant) Monte Carlo Tree Search library. Enables language bindings (Python, JavaScript, Go, Java, C, C++) by bridging `treant`'s static generics to a trait-object-based API.

Games and evaluators are defined via dyn-safe callbacks (`GameCallbacks`, `EvalCallbacks`) using string-based moves — no Rust generics required across the FFI boundary.

## Overhead

Approximately **1.4x** native Rust performance for realistic two-player games (benchmarked against Mancala). The trait-object indirection and String-based move encoding cost less than you'd expect.

## Example

```rust
use treant_dynamic::{DynMCTSManager, GameCallbacks, EvalCallbacks};
// See the docs for a complete example.
```

- [Documentation](https://docs.rs/treant-dynamic)
- [Source and issues](https://github.com/patricker/treant)

## License

MIT
