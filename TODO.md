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
- [x] CI: `.github/workflows/ci.yml` builds the default (host) workspace, runs the
      unit tests, builds the firmware for `thumbv6m-none-eabi` and the simulator
      for `wasm32-unknown-unknown`.  (`deploy-sim.yml` only built the wasm app,
      so the host and firmware builds were unchecked.)
- [ ] `trunk serve`; manually step through every show via the picker and eyeball each effect.
- [ ] Verify short-press advance and 1 s long-press auto-advance toggle behave like hardware.
- [ ] Confirm output is deterministic across reloads (fixed RNG seeds).
- [ ] Update `README.md` with a "Simulator" section (build/run instructions).

## Phase 7 — Restore USB UART interface

The RP2040 firmware used to expose a USB CDC ("USB UART") interface that the
host could use to communicate with the firmware. It was drafted and cleaned up
on `main` in `61b1d9e` ("Draft usb communication") — after this branch
diverged — so none of it exists in the current workspace layout. The original
implementation lives in `main:src/interface.rs` (all the CDC plumbing) and
`main:src/main.rs` (one `poll_usb()` call plus newline-terminated ASCII show
announcements: `"snow\n"`, `"sine\n"`, …). The `usb-old` branch (`41ba30e`
"Somewhat working usb") has a working bidirectional read/echo loop that is a
useful reference. Restoring it here should be **bidirectional** so the
simulator's chat window (below) can both display and send messages.

### Firmware side

- [x] Add `usb-device = "0.2.9"` and `usbd-serial = "0.1.1"` to `firmware/Cargo.toml` (the exact versions used on `main`).
      (Later upgraded to `usb-device 0.3` / `usbd-serial 0.2`, together with
      `rp-pico 0.9` / `rp2040-hal 0.10` — its `UsbBus` implements the 0.3
      `usb-device` trait, so both bumps belong together.)
- [x] Restore the CDC setup in `FirmwarePlatform` (or a new `firmware/src/usb.rs`), mirroring `main:src/interface.rs`:
      - [x] take `USBCTRL_REGS`, `USBCTRL_DPRAM`, `clocks.usb_clock` and `RESETS` **before** the other peripherals are consumed,
            (`usb_clock` moves out of `clocks` right after `Timer::new`, the last
            whole-`clocks` borrower; `USBCTRL_REGS`/`USBCTRL_DPRAM` are taken first)
      - [x] allocate the bus via `cortex_m::singleton!(: UsbBusAllocator<UsbBus> = UsbBusAllocator::new(UsbBus::new(...)))`,
      - [x] build the `UsbDevice` with `device_class(2)` (CDC) and the original identity: VID/PID `0x16c0:0x27dd`, manufacturer `"c3eo"`, product `"Lumio"`, serial `"001"`.
- [x] Extend the `Platform` trait in `core/src/platform.rs` with USB methods — `usb_poll()`, `usb_write(&[u8])`, `usb_read(&mut [u8])` — with **default no-op impls** so the WASM sim stays unaffected until it implements them.
      (`Interface` forwards them to the platform.)
- [x] Call `usb_poll()` once per `Player::tick` (1/60 s cadence); on `main` it was polled once outside the show loop, which is not possible with the steppable player.
- [x] Emit `"<show>\n"` via `usb_write` on show transitions, using `player.current_name()` (replaces the hand-written labels in `main:src/main.rs`).
      (Also announced once at boot, right after `Player::new`.)
- [x] Drain `usb_read` each tick; treat newline-terminated ASCII lines as inbound messages.
      (Done: the player echoes newline-terminated lines back to the host, and
      drops overly long lines. Optional later: command dispatch, e.g. switching
      shows from the host.)

### Simulator side (chat window)

- [x] Add a message log (`Vec<String>` or ring buffer) to `Shared` in `sim/src/lib.rs`.
      (A `VecDeque<String>` capped at 200 lines; written bytes are accumulated
      and flushed into the log on each newline.)
- [x] Expose the wasm API:
      - [x] `Sim::messages() -> Vec<String>` for the chat pane,
      - [x] `Sim::send(text: String)` to queue inbound chat text.
- [x] Implement the new `Platform` USB methods in `SimPlatform`:
      - [x] `usb_write` payloads are appended to the message log (so show announcements appear as chat lines),
      - [x] chat text queued via `Sim::send` is drained by `usb_read` during `tick`.
- [x] UI, following the existing control-panel pattern:
      - [x] `sim/index.html`: a scrollable message panel (`<div id="chat">`) with an `<input>` + send button at the bottom of the control panel,
      - [x] `sim/styles.css`: style the chat pane (fixed height, scroll to bottom),
      - [x] `sim/main.js`: refresh the message pane from `sim.messages()` inside the existing `frameLoop` poll-per-frame pattern, and wire the send button / Enter key to `sim.send(text)`.

### Notes

- [x] **Polling cadence:** `main` polled USB outside the show loop; the new player ticks once per frame, so the poll (and read/write) belong inside `Player::tick`.
- [x] **Driver versions:** started with `usb-device 0.2.9` to match the proven
      `main` code, then upgraded to `usb-device 0.3` / `usbd-serial 0.2` (and
      `rp-pico 0.9` / `rp2040-hal 0.10`, whose USB bus implements the 0.3 trait).
      The 0.3 string-descriptor API (`manufacturer`/`product`/`serial_number`)
      moved into `StringDescriptors`, passed via `UsbDeviceBuilder::strings`.
      `usbd-serial` 0.2 buffers writes internally, so each `usb_write` call now
      survives multiple frames at the 1/60 s poll cadence. Moving to
      `embassy-usb` remains a separate later task.
- [ ] **`usb_read` was unused on `main`** — the write side is proven, the read side needs its first real test (the `usb-old` echo loop is the reference).
      The echo path is implemented and covered by a simulator test
      (`chat_message_is_echoed_back`), but real-hardware verification over a
      physical USB CDC link is still outstanding.

## Known risks / notes

- [x] **Firmware pacing change:** shows no longer loop internally; the `Player` ticks each show once per loop iteration.
- [x] **Borrow splitting** in `Interface::write_spi()` is done by destructuring `&mut self`.
- [x] `delay_ms` in sim is a **no-op** for the first cut (sim cannot block).
- [x] `rand` `SmallRng` + `libm` need no OS entropy → sim is fully deterministic.
- [x] `power_test.rs` moved to `core/src/power_test.rs` but stays undeclared, so it is not compiled and not in the playlist.
- [x] **Trunk install needed a workaround:** `trunk` 0.21.14 pulls `libdeflate-sys`, whose C sources use the GCC target attribute `no-evex512`, which this toolchain rejects. Installing with `CFLAGS="-D__EVEX512__" CXXFLAGS="-D__EVEX512__"` makes it build. Re-check if this is still required after a toolchain upgrade.
- [x] **`thumbv6m` builds needed the 1.86 toolchain at one point:** rustc ≥ 1.90 removed the `-C inline-threshold` rustflag. Fixed by switching the flag in `.cargo/config.toml` to `-Cllvm-args=--inline-threshold=5` (LLVM's own spelling), so `cargo build --target thumbv6m-none-eabi` works on the default stable toolchain again.
- [x] **The firmware cannot be built for the host:** `lumio-firmware` is bare-metal code, so it needs a `none` target for its panic handler and linker scripts. Since the workspace has no default target (setting one would drag the wasm simulator along), a plain `cargo build` in the repository root used to fail with "`#[panic_handler]` function required, but not found". The firmware is now excluded from the workspace's `default-members` (so `cargo build` / `cargo test` cover `lumio-core` + `lumio-sim`), it is built with `cargo firmware`, and `firmware/build.rs` rejects host builds with a message pointing at that alias.

## Later / nice-to-have

- [ ] Show the two status LEDs and the current show name on screen.
- [ ] Real `delay_ms` fidelity (hold frames).
- [ ] Bloom/glow rendering pass.
- [ ] Extra status: FPS, current show.
- [ ] Fix `led.rs` `brightness()` (uses `g` twice, ignores `b`).
- [ ] Optional 3D funnel projection.
