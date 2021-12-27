# Anchor Escrow

## Quickstart
1. Clone the repo
2. `anchor test`
3. If you get any errors, make sure you've gone through Anchor's [minimal example](https://project-serum.github.io/anchor/tutorials/tutorial-0.html)

## Notes
The "Program" lives inside `programs/src/lib.rs`

The "App" (doubles as integration tests) lives inside `tests/`

The integration tests (mocha.js) are simple: they just make sure the balances line up after each escrow operation.
What else might need to be checked (that isn't already via `constraint`)?

## Todos
Add "local" Rust unit tests (`solana-program-test`) that don't rely on the Anchor IDL