//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // This crate is bare-metal firmware: it needs a `none` target (e.g.
    // thumbv6m-none-eabi) for the panic handler, the linker scripts and the
    // RP2040 peripherals.  Compiling it for the host cannot work, and the
    // resulting rustc errors ("`#[panic_handler]` function required", "not
    // supported without std") are hard to make sense of, so stop here with
    // a message that says what to do instead.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "none" {
        panic!(
            "lumio-firmware only builds for a bare-metal target.\n\
             Use `cargo firmware` \
             (cargo build -p lumio-firmware --target thumbv6m-none-eabi)."
        );
    }

    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");
}
