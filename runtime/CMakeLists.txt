cmake_minimum_required(VERSION 3.15)
project(neon_rt C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

# One translation unit per area of the runtime, each with the matching public header in
# `include/neon/`. `include/libneon_rt.h` is the umbrella the emitted C includes, so a
# program never has to know this layout; `src/internal.h` carries the bits only these
# files share and is not installed.
set(NEON_RT_SOURCES
    src/lifecycle.c
    src/trap.c
    src/arith.c
    src/string.c
    src/resource.c
    src/file.c
    src/math.c
    src/io.c
    src/list.c
    src/any.c
    src/map.c
)

# The runtime ships as prebuilt archives, one per build shape the CLI can ask for, so a
# Neon build links a `.a` instead of recompiling the whole runtime every time (and a
# shipped toolchain need not ship the runtime's C source). The flags here must stay in
# step with `BuildConfig::cc_args` in `cli/src/buildcfg.rs`, which builds the *program*
# with the matching shape; `cli/src/buildcfg.rs::runtime_variant` is what picks between
# them.
#
# Two deliberate differences from what `cc_args` would have produced when the runtime was
# compiled from source alongside the program:
#
#   * No `-flto`. `cc_args` passes `-flto` from `release` up, and it used to cover the
#     runtime too. A prebuilt archive cannot: an LTO archive carries compiler-version-
#     specific bitcode, and the compiler that built the toolchain is not the `cc` that
#     links a user's program. The user's own code is still LTO'd; the runtime's internal
#     cross-TU inlining is what is lost.
#   * No `-march=native` (see the `OptRelease` note in `runtime_variant`): a prebuilt
#     archive is built once, for an unknown target.
#
# Also: `cflags`/`-D` a user passes reach their program only. They no longer reach the
# runtime, because the runtime is no longer compiled per build.
function(neon_rt_variant name)
    add_library(${name} STATIC ${NEON_RT_SOURCES})
    target_include_directories(${name} PUBLIC include)
    # -Werror on every variant: the runtime is warning-clean in all three shapes, and a
    # variant that is only ever built in CI is exactly where rot would go unnoticed.
    target_compile_options(${name} PRIVATE -Wall -Wextra -Werror ${ARGN})
    install(TARGETS ${name} ARCHIVE DESTINATION lib)
endfunction()

# `release`: what `Mode::Release` implies, minus `-flto` (above). Also what `OptRelease`
# links.
neon_rt_variant(neon_rt -O3)
# `debug`: `Mode::Debug`. `NEON_DEBUG` turns on the runtime's own assertions.
neon_rt_variant(neon_rt_debug -O0 -g -DNEON_DEBUG)
# `sanitized`: the only variant that carries instrumentation. ASan does not see a
# heap-buffer-overflow that happens inside code compiled without `-fsanitize`, so an
# uninstrumented archive in a sanitized program is a silent hole -- the CLI refuses to
# fall back to one rather than produce a build that covers half the program.
neon_rt_variant(neon_rt_san -fsanitize=address,undefined -O1 -g -fno-omit-frame-pointer)

install(DIRECTORY include/ DESTINATION include)
