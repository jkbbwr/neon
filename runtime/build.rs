fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=include");
    println!("cargo:rerun-if-changed=CMakeLists.txt");

    // The runtime is built as several archives with hand-picked, per-variant flags (see
    // `CMakeLists.txt`), so cmake must not inject any of its own: `CMAKE_BUILD_TYPE=None`
    // suppresses the `-O`/`-DNDEBUG` a build type would add, and `no_default_flags` stops
    // the cmake crate forwarding cargo's profile flags into `CMAKE_C_FLAGS`. Without both,
    // the sanitized variant would get an `-O` it did not ask for and the debug variant an
    // `-O2` that defeats the point of it.
    let dst = cmake::Config::new(".")
        .no_default_flags(true)
        .define("CMAKE_BUILD_TYPE", "None")
        .build();

    // `links = "neon_rt"` turns these into DEP_NEON_RT_{ROOT,INCLUDE} for
    // dependents' build scripts. No rustc-link-lib: nothing in Rust links this.
    println!("cargo:root={}", dst.display());
    println!("cargo:include={}/include", dst.display());
}
