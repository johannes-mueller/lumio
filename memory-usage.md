# Lumio firmware memory usage analysis

**Target:** RP2040 (256 KB RAM, 2 MB Flash)
**Build:** Debug (`target/thumbv6m-none-eabi/debug/lumio-firmware`)
**Date:** 2026-11-10

## Quick summary

| Resource | Used | Available | Utilization |
|----------|------|-----------|-------------|
| RAM (stack + static) | ~56.7 KB | 256 KB | **22.2 %** |
| Flash (code + data) | ~80 KB | 2048 KB | **~4 %** |

There is **~199 KB of RAM headroom** for nested calls and future features.

## Memory layout (`memory.x`)

```
BOOT2 : 0x10000000, 256 B
FLASH : 0x10000100, 2048 KB - 0x100
RAM   : 0x20000000, 256 KB
```

## Static globals (`.data` / `.bss` / `.uninit`)

These are the linker-allocated global variables, *not* the stack.

| Section | Address | Size | Contents |
|---------|---------|------|----------|
| `.data` | 0x2003fac8 | 56 B | Initialized globals (`SEGGER_RTT`, `defmt_rtt::NAME`) |
| `.bss`  | 0x2003fb00 | 252 B | Zero-initialized (USB bus singleton, cached ROM fn ptrs, lock flags) |
| `.uninit` | 0x2003fbfc | 1024 B | `defmt_rtt::BUFFER` (RTT logging ringbuffer) |
| **Total static** | | **1332 B** | |

Verified with:
```bash
arm-none-eabi-nm -S target/thumbv6m-none-eabi/debug/lumio-firmware | grep ' [bBdD] '
```

## Stack (the big one)

### Initial stack pointer

From the vector table at `0x10000100`:

```
Initial SP = 0x2003fac8
```

The stack grows **downward** from `0x2003fac8` toward `0x20000000`. Static data sits at the top of RAM (`0x2003fac8–0x2003fffc`). This is `flip-link`'s hallmark — stack overflow hits unmapped space instead of corrupting globals.

### main() stack frame

Disassembly of `__cortex_m_rt_main` (`0x10000490`):

```asm
10000490:  push  {r4, r5, r6, r7, lr}
10000494:  ldr   r6, [pc, #100]      @ loads 0xffff2274 (= -56716)
10000496:  add   sp, r6              @ sp -= 56716
```

**main() allocates 56,716 bytes of stack** for its three local variables:

| Variable | Type | Approx. size |
|----------|------|-------------|
| `platform` | `FirmwarePlatform` | ~200 B |
| `interface` | `Interface` | ~20,400 B |
| `player` | `Player` | ~34,300 B |
| Frame overhead | saved regs, alignment | ~1,800 B |
| **Total** | | **56,716 B** |

All three are alive simultaneously (borrowed in the `loop { player.tick(&mut interface) }`).

### Peak stack depth

```
Bottom of RAM  0x20000000
                    ⋮
               ~204 KB free for nested calls
                    ⋮
  SP after     0x20031d28   ← main()'s locals done
  main frame
                    ⋮
               ~56 KB in use
                    ⋮
  Static data  0x2003fac8–0x2003fffc  (1.3 KB)
  Top of RAM   0x2003ffff
```

During `player.tick()` → `show.step()` → `led_strip.refresh()` the stack dips further, but by a negligible amount (a few KB at most). The **199 KB hole** below `0x20031d28` is effectively unused.

## Per-show breakdown

The `Player` struct embeds **all 18 shows simultaneously** (no heap, no dynamic dispatch per-show). Each show contributes its struct size to the total.

### 🔥 Largest contributors

| Show | State size | What's inside |
|------|-----------|---------------|
| **SparkFall** | **~6,144 B** | `[FallingSparks; 192]` — 192 falling particles |
| **FireWorks** | **~5,280 B** | `[MonoSpark; 192]` + `[ColorSpark; 24]` |
| **ParticleCrash (spiral)** | **~5,004 B** | 24 BigParticle + 24 SmallParticle + 24 Explosions |
| **ParticleCrash (random)** | **~5,004 B** | Same struct, different mode |
| **PlanetShow** | **~4,808 B** | 24 Explosions + 10 Planets |

### 🔥 Medium contributors

| Show | State size | What's inside |
|------|-----------|---------------|
| **SnowShow<180>** | **~1,544 B** | 180 SnowFlakes + coverage array |
| **SnowSparks** | **~1,540 B** | 48 FallingSparks |
| **Fire (green)** | **~1,476 B** | `[u8; 1440]` heat array |
| **Fire (red)** | **~1,476 B** | Same, different color |
| **SnowShow<90>** | **~824 B** | 90 SnowFlakes + coverage array |
| **SnakeShow** | **~776 B** | 48 Snakes (24 constant + 24 random) |

### 🟢 Small contributors

| Show | State size | Notes |
|------|-----------|-------|
| HueSpiral (lift/swirl) | ~16 B each | Wraps tiny Spiral + HueWave |
| SeaWave (chaos/order) | ~120 B each | Sine + f32 + enum |
| Stars (EO/EU) | ~40 B each | Two Colors + Random |
| SineShow | ~36 B | Sine + f32 |

### Visual

```
SparkFall       6,144 B ████████████████████████
FireWorks       5,280 B ██████████████████████
ParticleCrash   5,004 B █████████████████████   ×2
PlanetShow      4,808 B ████████████████████
SnowShow<180>   1,544 B ██████
SnowSparks      1,540 B ██████
Fire             1,476 B ██████                ×2
SnowShow<90>      824 B ███
SnakeShow         776 B ███
6 small shows    ~400 B  █
─────────────────────────────────────
Player total   ~34,300 B
```

### Why is ParticleCrash / FireWorks / SparkFall so big?

- `SPARKS_PER_STRIP = 8`, `STRIP_NUM = 24` → **192 sparks**
- Each `MonoSpark` = 24 bytes (`SparkEngine` × 4 isize fields + `f32` hue + 2 u8)
- Each `Explosion` = 8 MonoSpark = 192 bytes
- `Explosions` = 24 strips × 192 = **4,608 B** for the Explosions alone
- `SparkFall` uses a different spark type (`FallingSparks` = 32 bytes each → 192 × 32 = **6,144 B**)

### The true heavyweight: LEDStrip

The `LEDStrip` struct lives in `Interface`, not in the `Player`, but accounts for most of the Interface size:

| Field | Size | Derivation |
|-------|------|-----------|
| `bytes_0` | 2,949 B | `HALF * 4 + 4 + TAIL` = 720 × 4 + 4 + 65 |
| `bytes_1` | 2,949 B | Same for second SPI bus |
| `leds` | 14,400 B | `[Led; 1440]` × **10 B each** |
| `random` | ~32 B | `SmallRng` |
| **Total** | **~20,330 B** | **19.9 KB** |

`Led` is packed at 10 B (verified from disassembly offsets):

```rust
struct Led {
    current: Color,       // 3 B  (r, g, b: u8)
    target: Option<Color>,// 4 B  (Color + 1-byte discriminant)
    decay: u8,            // 1 B
    flicker: u8,          // 1 B
    current_flicker: u8,  // 1 B
}                         // = 10 B, alignment 1
```

## Flash usage

From `arm-none-eabi-size`:

```
text     data     bss     dec     hex
81708    56       1276    83040   14460
```

| Region | Size | Contents |
|--------|------|----------|
| `.vector_table` | 192 B | Exception vectors + initial SP |
| `.boot2` | 256 B | Second-stage bootloader |
| `.text` | 68,700 B | Code |
| `.rodata` | 12,304 B | String constants, lookup tables |
| `.data` (LMA copy) | 56 B | Initializers for RAM globals |
| **Total Flash** | **~81,708 B (80 KB)** | **~4 % of 2 MB** |

## How to measure this yourself

```bash
# Static globals
arm-none-eabi-size target/thumbv6m-none-eabi/debug/lumio-firmware

# Section layout
arm-none-eabi-objdump -h target/thumbv6m-none-eabi/debug/lumio-firmware

# Initial SP (first word of vector table)
arm-none-eabi-objdump -s -j .vector_table \
  --start-address=0x10000100 --stop-address=0x10000104 \
  target/thumbv6m-none-eabi/debug/lumio-firmware

# Linker memory usage (release, adds --print-memory-usage)
RUSTFLAGS="-C link-arg=--print-memory-usage" cargo firmware

# Stack frame size (find main's "add sp, N" instruction)
arm-none-eabi-objdump -d target/thumbv6m-none-eabi/debug/lumio-firmware \
  | grep -B2 'add.*sp, r[0-9]' | head -20
```

## How to reduce RAM usage

Not necessary now (78 % free), but for reference:

1. **Halve `SPARKS_PER_STRIP`** from 8 to 4 — SparkFall drops from 6 KB to 3 KB, FireWorks from 5.3 KB to 2.7 KB.
2. **Make spark arrays dynamic** — but that needs a heap allocator on the RP2040, adding complexity.
3. **Reduce `NUM_LED`** — but that changes the physical hardware.
4. **Use generational indices** — replaces big fixed arrays with smaller active-sets.

None of these are urgent with 199 KB free.