//! Pins the hot code layout of the `qjs` binary on macOS.
//!
//! The target's order file lists the functions execution spends its time in,
//! hottest first (`python3 -m tools.benchmark.order_file`). Placing them first
//! and in a fixed order keeps their addresses stable when unrelated code
//! changes, which otherwise moves some benchmarks' cycles by several percent
//! with identical instruction counts. Symbols the list names but the binary
//! lacks are ignored by the linker, so a stale list only loses precision.
//! x86_64 uses its own symbol list; the default keeps the arm64 layout.

use std::env;
use std::path::PathBuf;

fn main() {
    let filename = if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("x86_64") {
        "hot-functions-x86_64.order"
    } else {
        "hot-functions.order"
    };
    let order = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets the manifest dir"))
        .join(filename);
    println!("cargo:rerun-if-changed={}", order.display());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") && order.is_file() {
        if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("x86_64") {
            // ld can move __text's start when the order changes. Page alignment
            // makes the pin's starting offset independent of the previous link.
            println!("cargo:rustc-link-arg-bin=qjs=-Wl,-sectalign,__TEXT,__text,1000");
        }
        println!(
            "cargo:rustc-link-arg-bin=qjs=-Wl,-order_file,{}",
            order.display()
        );
    }
}
