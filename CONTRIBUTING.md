# Contributing

Thank you for helping. archgram is small on purpose; a change is easiest to
accept when it fits what [docs/PRD.md](docs/PRD.md) sets out, so for
anything larger than a fix, open an issue first and describe the problem.

## Setting up

You need Rust, through [rustup](https://rustup.rs): the toolchain is pinned
in `rust-toolchain.toml` and installs itself on the first `cargo` command.
Node 22 or later is needed only for the npm launcher's tests.

## Build and try a change

```sh
cargo run -p archgram-cli -- build examples/linkshort.json -o target/linkshort.svg
```

Open `target/linkshort.svg` in a browser, in light and in dark mode.

## Where things are

- `crates/archgram-core`: the engine, from spec to SVG.
- `crates/archgram-cli`: the `archgram` command.
- `examples/`: specs to try; `docs/SPEC.md`: what a spec may say.

## Before a pull request

Each of these must pass; CI runs them on macOS, Linux and Windows.

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo xtask deps
node --test packages/archgram/lib/platform.test.js
```

## What to keep in mind

- **The same spec draws the same bytes, everywhere.** The golden SVGs in
  `crates/archgram-core/tests/golden/` are compared byte for byte. When a
  change to the drawing is intended, run `ARCHGRAM_BLESS=1 cargo test`, look
  at the diff of the goldens and include it in the pull request.
- **Each fact lives in one document.** What archgram does and why is
  [docs/PRD.md](docs/PRD.md); the spec is [docs/SPEC.md](docs/SPEC.md); the
  visual rules are [DESIGN.md](DESIGN.md), their values the tokens in
  `design-system/tokens/`; how it works inside is
  [ARCHITECTURE.md](ARCHITECTURE.md). Change the document that owns a fact,
  and only that one.
- **No new dependency without an issue first.** Every dependency is a
  supply-chain decision; `cargo xtask deps` and `deny.toml` check the ones
  there are.
- **Commit messages** are `type: subject`, the subject in the imperative
  ("if applied, this commit will …"), such as `fix: keep an edge label off
  its arrowhead`. A pull request is squashed into one commit on `main`, so
  its title is that commit's message.

## Reporting a vulnerability

Please don't open a public issue; see [SECURITY.md](SECURITY.md).
