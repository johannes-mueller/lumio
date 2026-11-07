# AGENTS.md

## Build and verify

- Three-crate Cargo workspace: `core` (`lumio-core`, shared `no_std` logic),
  `firmware` (RP2040), `sim` (WebAssembly).
- Plain `cargo build` / `cargo test` only cover `core` and `sim`. Firmware is
  excluded from `default-members` and cannot build for the host;
  `firmware/build.rs` deliberately panics with a hint if you try.
- Firmware commands:
  - `cargo firmware` debug, `cargo firmware-release` release.
  - Quick check: `cargo check -p lumio-firmware --target thumbv6m-none-eabi`.
- Tests live in `sim/src/lib.rs`. Run all with `cargo test`; run one with
  `cargo test -p lumio-sim <test_name>`.
- Simulator: `cd sim && trunk serve`; static build:
  `cd sim && trunk build --release`.
- Required tools/targets: `thumbv6m-none-eabi`, `wasm32-unknown-unknown`,
  `flip-link`, `elf2uf2-rs`, and `trunk`. Install both targets manually unless
  a `rust-toolchain.toml` provides them.
- `.github/workflows/ci.yml` runs the host build, tests, firmware build, and
  wasm release build. Do those locally for non-trivial changes.

## Firmware and flashing

- `cargo flash` flashes the release build; `cargo flash-debug` flashes the
  debug build. Both use `elf2uf2-rs -d`, so the Pico must be in BOOTSEL/RPI-RP2
  mode.
- `cargo flash` intentionally shadows the external `cargo-flash` subcommand.
  The Cargo warning about this is expected.
- Do not add `[build] target` to `.cargo/config.toml`. The rustflags and runner
  are target-gated to bare-metal ARM; a global target would force the wasm and
  host crates onto `thumbv6m`.
- Keep the `flip-link` linker configuration. Do not remove the `-m armelf`
  linker args: they pin the ELF OSABI to System V, which older `elf2uf2-rs`
  versions require.
- Firmware builds fail unless `flip-link` is installed.

## Architecture and conventions

- `lumio-core` is hardware-independent `#![no_std]`. Firmware and simulator
  provide `Platform` implementations; keep hardware, `std`, and wasm concerns
  out of `core`.
- Shows are state machines stepped one frame at a time by `Player`. Do not
  reintroduce internal loops or blocking delays in shows; both targets rely on
  the single-step model.
- To add a show: add its module to `core/src/lib.rs`, implement `Show`, and add
  it to the `define_player!` playlist in `core/src/player.rs`. `power_test.rs`
  is intentionally not compiled or in the playlist.
- Simulator tests run every show and check UI-facing behavior, but hardware
  buttons, USB, and timing still need verification on the board.
- `cargo-generate/` is an old template directory and not a workspace member;
  do not treat it as current source.
- `Cargo.lock` is currently gitignored, so dependency changes will not show
  up in `git status`.
- The tree is not rustfmt-clean. `cargo fmt --all` creates unrelated churn;
  format only the files you touch.
- `.github/workflows/deploy-sim.yml` deploys the main simulator plus one
  preview per open PR that touches `sim/`, `core/`, or Cargo files. Do not
  casually change its Pages deployment or concurrency setup.

## Show implementations

- Using coding agents to implement or modify `Show` trait implementations is
  highly discouraged. The shows are human art rather than technical solutions
  to a problem; they exist for people to have fun with creativity. Limit agent
  work to surrounding plumbing, tests, the simulator and documentation.
