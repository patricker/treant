# treant-gumbel

Gumbel MuZero search for Rust: Sequential Halving with Gumbel noise for [Monte Carlo Tree Search](https://en.wikipedia.org/wiki/Monte_Carlo_tree_search). Produces a policy with *monotonic* improvement — more simulations always yield a better move distribution.

Built on top of the [`treant`](https://crates.io/crates/treant) crate, reusing its `GameState` trait so any game works with both standard MCTS and Gumbel search.

Based on Danihelka et al., ["Policy improvement by planning with Gumbel"](https://openreview.net/forum?id=bERaNdoegnO) (ICLR 2022).

## When to use

- Self-play training with guaranteed policy improvement
- Distilling search into a neural network
- Low simulation budgets where PUCT degrades

## Example

```rust
use treant_gumbel::{GumbelConfig, GumbelSearch};
// See the docs for a complete example.
```

- [Documentation](https://docs.rs/treant-gumbel)
- [Tutorial](https://mcts.dev/docs/tutorials/08-gumbel-search)
- [Source and issues](https://github.com/patricker/treant)

## License

MIT
