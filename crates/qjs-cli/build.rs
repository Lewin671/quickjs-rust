//! Pins the hot code layout of the `qjs` binary on macOS.
//!
//! `hot-functions.order` lists the functions benchmarks spend their time in,
//! hottest first (`python3 -m tools.benchmark.order_file`). Placing them first
//! and in a fixed order keeps their addresses stable when unrelated code
//! changes, which otherwise moves some benchmarks' cycles by several percent
//! with identical instruction counts. Symbols the list names but the binary
//! lacks are ignored by the linker, so a stale list only loses precision.

use std::env;
use std::path::PathBuf;

fn main() {
    let order = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets the manifest dir"))
        .join("hot-functions.order");
    println!("cargo:rerun-if-changed={}", order.display());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") && order.is_file() {
        println!(
            "cargo:rustc-link-arg-bin=qjs=-Wl,-order_file,{}",
            order.display()
        );
    }
}
