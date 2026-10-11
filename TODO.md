# TODO — Lumio software simulator

Living task list for adding a browser-based simulator (Rust → WebAssembly) that
runs the existing show code in software and renders the 24 LED strips as a
star of beams. Check items off as they are completed; add new items as they
come up.

## Goal

Run the **same** show logic as the RP2040 firmware, in a web browser, without
hardware. Render the 24×60 APA102 LEDs as a 2D star (24 beams from a center
hub). Mimic the two hardware buttons as on-screen controls.

## Locked decisions

| Topic            | Decision                                                                 |
| ---------------- | ------------------------------------------------------------------------ |
| Structure        | Cargo workspace: `lumio-core` + `lumio-firmware` + `lumio-sim`            |
| Driving model    | Steppable one-frame-per-tick, single-threaded `requestAnimationFrame`     |
| wasm tooling     | `trunk`                                                                   |
| Controls         | Show picker, pause/resume, step-one-frame, speed control                  |
| Button emulation | Press-and-hold with real timing (short press / 1 s long press)            |
| Star layout      | Sensible defaults as tunable constants; no status LEDs / show label (yet) |
| Playlist         | The 16 shows currently in `main.rs`; `power_test.rs` excluded             |

## Target layout

```
lumio/
  Cargo.toml             # [workspace] members = core, firmware, sim
  .cargo/config.toml     # keep; arm rustflags are target-gated
  core/     (lumio-core)     # #![no_std], hardware-free
  firmware/ (lumio-firmware) # current RP2040 binary
  sim/      (lumio-sim)      # wasm-bindgen cdylib + index.html + Trunk.toml
  TODO.md
```

---

## Phase 0 — Workspace scaffold  ✅

- [x] Convert root `Cargo.toml` into a `[workspace]` with members `core`, `firmware`, `sim`.
- [x] Move the current package (`Cargo.toml` deps, `build.rs`, `memory.x`, `Embed.toml`, root `src/`) into `firmware/`.
      (Note: `.vscode/` left at the repo root — it is the editor workspace, not
      part of the crate; `launch.json` binary path updated in a later step.)
- [x] Keep `.cargo/config.toml` at the repo root; confirm the arm-only rustflags do not leak into the wasm build.
- [x] Add `core/Cargo.toml` and `sim/Cargo.toml`, verify `cargo metadata` works.
- [x] Confirm the firmware still builds after the move (see Phase 3 for the real gate).

## Phase 1 — Extract `lumio-core` (hardware-free)  ✅

Modules live in `core/src/`: `conf`, `math8`, `random`, `led`, `ledstrip`,
`button`, `showtimer`, `platform`, `interface`, `player`, and all show modules
(`fire`, `huewave`, `particle_crash`, `planets`, `sine`, `snake`, `sparks`,
`spiral`, `stars`).

- [x] Create `core/src/lib.rs` (`#![no_std]`) re-exporting the moved modules.
- [x] Port `conf.rs`, `math8.rs`, `random.rs`.
- [x] `led.rs`: replace the RP2040 ROM call `rp_pico::hal::rom_data::float_funcs::float_to_uint` with a portable `((x) * 255.0) as u8` conversion.
- [x] `random.rs`: keep `rand` as `default-features = false, features = ["small_rng"]`; confirm no `getrandom`/OS entropy is pulled in.
- [x] `ledstrip.rs`: split `dump_0()` / `dump_1()` into
      - [x] a `refresh()` that advances all LEDs (`led.step`), and
      - [x] byte accessors, so `Interface::write_spi()` can borrow `platform` and `led_strip` as disjoint fields.
- [x] `platform.rs` (new): define
      - [x] `enum ButtonId` (`Advance`, `Action`) / `enum StatusLedId` (`Auto`, `Active`),
      - [x] `type Micros = u64`,
      - [x] `trait Platform { fn now_micros(&self) -> Micros; fn present(&mut self, a: &[u8], b: &[u8]); fn delay_ms(&mut self, ms: u32); fn raw_button(&self, id: ButtonId) -> bool; fn set_status_led(&mut self, id: StatusLedId, on: bool); }`.
- [x] `interface.rs`: split the hardware init into the firmware crate; `Interface` keeps `LEDStrip`, `Random`, `ShowTimer`, action `Button`, and a `&mut dyn Platform`.
      - [x] `Interface` stayed non-generic (lifetime-parameterised only) so show signatures are unchanged.
      - [x] Add `Interface::begin_tick()` which pulls `now` + raw button states from the platform into fields, so `Button`/`ShowTimer` debounce logic runs without a global clock.
- [x] `button.rs` / `showtimer.rs`: replace `rp_pico::hal::timer::Instant` with the core `Micros` type; keep debounce, short/long-press, and auto-advance (60 s) logic in core.
- [x] Confirm no `rp-pico`, `cortex-m`, `defmt`, or `panic-probe` references remain in `core`.

## Phase 2 — Steppable player  ✅

- [x] `player.rs` (new):
      - [x] `enum StepResult { Running, Done }`
      - [x] `trait Show { fn reset(&mut self, iface: &mut Interface); fn step(&mut self, iface: &mut Interface) -> StepResult; }`
            (show names live in the playlist macro, so the trait needs no `name()`.)
      - [x] `Player` with **allocation-free** fixed dispatch over the playlist (firmware is `no_std`, so no `Box<dyn Show>`); a `define_player!` macro keeps it DRY.
- [x] Convert each show's `show()` loop into `reset()` + one-frame `step()`; move the `process()` bodies in as the step body.
      - [x] `huewave`/`spiral` (`new_lift`, `new_swirl`)
      - [x] `fire` (red, green)
      - [x] `stars` (EU, Esperanto)
      - [x] `snake`
      - [x] `sine` (SineShow, SeaWave ×2)
      - [x] `sparks` (FireWorks, SnowSparks, SparkFall)
      - [x] `particle_crash` (`new_random`, `new_spiral`)
      - [x] `planets`
- [x] Move button/auto-advance handling out of the shows into `Player::tick`:
      - [x] each tick calls `Interface::begin_tick()` then `do_next()`,
      - [x] advance to the next show when `do_next()` is true (and on show `Done`),
      - [x] keep the per-show action button (`button_state()`) available to shows that use it.
- [x] Preserve exit cleanup (`led_strip().black()`, `led_off()`) on show change.
- [x] `main.rs`'s playlist became the single ordered `Player` playlist (16 entries).

## Phase 3 — `lumio-firmware`  ✅

- [x] `firmware/src/main.rs`: keep `#[entry]`, `defmt_rtt`, `panic_probe`; build a hardware `Platform`, create `Interface`, run the `Player` in a `loop`.
- [x] Move hardware init (clocks, GPIO, two SPIs, `Delay`, timer, buttons, status LED pins) into `firmware/src/platform.rs` implementing `core::Platform`.
- [x] Keep `build.rs`, `memory.x`, `Embed.toml` in the firmware crate (`.vscode/` remains at the repo root).
- [x] **Gate:** `cargo build --target thumbv6m-none-eabi` and `--release` both succeed.
- [x] Re-verify button wiring: GPIO21 = next/auto-advance, GPIO20 = per-show action, GPIO10 = auto indicator, GPIO11 = show-active indicator.

## Phase 4 — `lumio-sim`  ✅

- [x] `sim/Cargo.toml`: `crate-type = ["cdylib", "rlib"]`, deps `lumio-core`, `wasm-bindgen`.
      (Only `wasm-bindgen` is needed — the canvas is drawn from plain JS.)
- [x] `SimPlatform`:
      - [x] virtual clock (`sim_time`), advanced by the host,
      - [x] settable raw button states,
      - [x] status LED states,
      - [x] `present(a, b)` decodes the exact APA102 bytes into an RGB framebuffer.
- [x] `#[wasm_bindgen] Sim` API:
      - [x] `new()`
      - [x] `tick(now_ms)` — advance sim time, run player steps, keep latest frame
      - [x] `frame()` — RGB bytes (decoded from the stored APA102 payload)
      - [x] `press_button(id, down)`
      - [x] `show_names()` / `select_show(i)` / `current_show()` / `current_show_name()`
      - [x] `set_paused(bool)` / `is_paused()` / `step_frame()`
      - [x] `set_speed(f64)` / `speed()`
      - [x] `next_show()`, `show_count()`, `auto_led()`, `active_led()`
- [x] Decode the APA102 bytes for rendering (most faithful; no new LED accessors).
- [x] `index.html` + `main.js` + `styles.css` + `Trunk.toml`:
      - [x] `<canvas>` for the star,
      - [x] two pointer-event buttons (press-and-hold),
      - [x] show `<select>`,
      - [x] pause/resume + step-frame buttons, speed slider,
      - [x] `requestAnimationFrame` loop: `sim.tick(performance.now())` → draw.
- [x] Canvas 2D rendering:
      - [x] 24 spokes at 15°, LED 0 at the hub, LED 59 at the tip (strip `s` = indices `s*60 .. s*60+59`),
      - [x] one dot per LED (plus a faint beam guide),
      - [x] geometry constants (hub radius, tip ratio, dot size, angle offset) in one block at the top of `main.js`.

## Phase 5 — Controls semantics  ✅

- [x] **Show picker** — selecting resets and jumps to a show.
- [x] **Pause/resume** — freeze `show.step()` and auto-advance; keep the clock + buttons live so long-press still registers.
- [x] **Step one frame** — while paused, run exactly one `Player::tick` and present it.
- [x] **Speed control** — fixed virtual step interval (1/60 s); each rAF runs `floor(elapsed_sim_time / step_interval)` ticks (bounded to 16), with `sim_time` advancing at `speed × realtime`. Time-based behaviour (`AUTO_SHOW_DELAY`, `LONG_PRESS_TIME`) uses the same scaled clock.

## Phase 6 — Validation & polish

- [x] `cargo check` for `thumbv6m-none-eabi` **and** `wasm32-unknown-unknown` pass.
- [ ] `trunk serve`; manually step through every show via the picker and eyeball each effect.
- [ ] Verify short-press advance and 1 s long-press auto-advance toggle behave like hardware.
- [ ] Confirm output is deterministic across reloads (fixed RNG seeds).
- [ ] Update `README.md` with a "Simulator" section (build/run instructions).

## Known risks / notes

- [x] **Firmware pacing change:** shows no longer loop internally; the `Player` ticks each show once per loop iteration.
- [x] **Borrow splitting** in `Interface::write_spi()` is done by destructuring `&mut self`.
- [x] `delay_ms` in sim is a **no-op** for the first cut (sim cannot block).
- [x] `rand` `SmallRng` + `libm` need no OS entropy → sim is fully deterministic.
- [x] `power_test.rs` moved to `core/src/power_test.rs` but stays undeclared, so it is not compiled and not in the playlist.
- [ ] **Trunk install needed a workaround:** `trunk` 0.21.14 pulls `libdeflate-sys`, whose C sources use the GCC target attribute `no-evex512`, which this toolchain rejects. Installing with `CFLAGS="-D__EVEX512__" CXXFLAGS="-D__EVEX512__"` makes it build. Re-check if this is still required after a toolchain upgrade.

## Later / nice-to-have

- [ ] Show the two status LEDs and the current show name on screen.
- [ ] Real `delay_ms` fidelity (hold frames).
- [ ] Bloom/glow rendering pass.
- [ ] Extra status: FPS, current show.
- [ ] Fix `led.rs` `brightness()` (uses `g` twice, ignores `b`).
- [ ] Optional 3D funnel projection.
