//! Publishes the runtime's built archive directory to the test harnesses as
//! `NEON_RT_LIB_DIR`.
//!
//! `tests/backend_run.rs` links the runtime, and since the runtime became a set of
//! prebuilt archives instead of a glob over `runtime/src/*.c`, that harness needs a path
//! to them. Taking `neon-runtime` as a build-dependency is what makes cargo build the
//! archives before this crate's tests exist, and what makes cargo hand us
//! `DEP_NEON_RT_ROOT` (from the runtime crate's `links = "neon_rt"`) to point at them.
//! Nothing here is linked into the compiler itself — the compiler emits C, it does not
//! contain the runtime.

fn main() {
    let root = std::env::var("DEP_NEON_RT_ROOT")
        .expect("neon-runtime's build script must publish cargo:root");
    println!("cargo:rustc-env=NEON_RT_LIB_DIR={root}/lib");
    println!("cargo:rerun-if-env-changed=DEP_NEON_RT_ROOT");
}
