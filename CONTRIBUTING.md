# Contributing

Thank you for helping. archgram is small on purpose; a change is easiest to
accept when it fits what [docs/PRD.md](docs/PRD.md) sets out, so for
anything larger than a fix, open an issue first and describe the problem.

## Setting up

You need Rust, through [rustup](https://rustup.rs): the toolchain is pinned
in `rust-toolchain.toml` and installs itself on the first `cargo` command.
Node is needed only for the npm launcher's tests. Work on Node 24, pinned
in `.nvmrc`, the version the release builds with; the launcher itself still
supports Node 22 or later, which CI checks.

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

## How changes land

- **One short-lived branch per change,** named `<type>/<what-it-changes>`,
  where the type is one of the pull request title's types below:
  `fix/stage-paths`, `docs/readme-geo`. A release's branch is
  `release/X.Y.Z` ([RELEASE.md](RELEASE.md)). A change that has merged is
  done: the next one starts a new branch from `main`.
- **A new feature starts in the PRD.** What it does and why goes into
  [docs/PRD.md](docs/PRD.md) first, then the spec, the design and the code.
  A feature that needs more than a few lines there gets its own document
  in `docs/features/` (its problem, who it serves, requirements, limits,
  success criteria and changelog), and the PRD points to it.
- **Every change reaches `main` through a pull request** whose checks
  pass (`CI passed`), merged with **Squash and merge**: the pull request
  becomes one commit.
- **The pull request's title is that commit's subject:** `type: subject`,
  the subject in the imperative, lowercase ("if applied, this commit will
  …"), such as `fix: keep an edge label off its arrowhead`. The types are
  `feat`, `fix`, `docs`, `ci`, `build`, `chore`, `refactor`, `style`,
  `test`, `perf` and `design`; a check on each pull request holds the title
  to this.
- **Its description is the commit's body:** what changed and why, what it
  does not do, and how it was checked. The pull request template asks for
  these.
- **Two pull requests that touch the same file** land one after the other:
  once the first merges, the second is rebased on `main`, checked again and
  pushed with `--force-with-lease`.

## What to keep in mind

- **The same spec draws the same bytes, everywhere.** The golden SVGs in
  `crates/archgram-core/tests/golden/` are compared byte for byte. When a
  change to the drawing is intended, run `ARCHGRAM_BLESS=1 cargo test`, look
  at the diff of the goldens and include it in the pull request.
- **Each fact lives in one document.** What archgram does and why is
  [docs/PRD.md](docs/PRD.md), and a feature's details its document in
  `docs/features/`; the spec is [docs/SPEC.md](docs/SPEC.md); the
  visual rules are [DESIGN.md](DESIGN.md), their values the tokens in
  `design-system/tokens/`; how it works inside is
  [ARCHITECTURE.md](ARCHITECTURE.md). Change the document that owns a fact,
  and only that one.
- **No new dependency without an issue first.** Every dependency is a
  supply-chain decision; `cargo xtask deps` and `deny.toml` check the ones
  there are.

## Reporting a vulnerability

Please don't open a public issue; see [SECURITY.md](SECURITY.md).
