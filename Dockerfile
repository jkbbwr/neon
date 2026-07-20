# A small, self-contained Neon toolchain image.
#
# Neon compiles to C and shells out to `cc` at run time, so the image has to carry a C
# compiler as well as the toolchain. Everything is built on musl and the final image is
# Alpine, so the neon binary, the prebuilt runtime archives, and the `cc` that later links a
# user's program all share one libc — a glibc-built archive linked by musl's cc would fail.
#
# The point is portability: `docker run` (or Podman, or Docker Desktop on Windows) gives you
# a working `neon` where installing a Rust + C toolchain directly is awkward or disallowed.

# ---- builder ----
FROM rust:alpine AS build

# musl-dev/gcc: the C toolchain the runtime's CMake build and the corpus link against.
# cmake: the runtime is a CMake project driven by neon-runtime's build script.
RUN apk add --no-cache musl-dev gcc make cmake

WORKDIR /src
COPY . .
RUN cargo build --release --locked

# `cargo build` stages the sysroot (include/, lib/<flavor>/, stdlib/) next to the binaries
# in target/release. Rearrange into an install prefix: bin/ beside lib/, include/, stdlib/,
# which is the layout `Sysroot::find` resolves from the binary's parent directory.
RUN set -eux; \
    mkdir -p /out/bin; \
    cp target/release/neon target/release/neon-lsp /out/bin/; \
    cp -r target/release/include target/release/lib target/release/stdlib /out/

# ---- final ----
FROM alpine:3.20

# gcc + musl-dev: `neon build` invokes `cc` and links the C standard headers/libs.
RUN apk add --no-cache gcc musl-dev

# Into /usr/local, so /usr/local/bin/neon finds /usr/local/{lib,include,stdlib} one level up.
COPY --from=build /out/ /usr/local/

# Fail the build if the toolchain is not actually wired up in the image, rather than shipping
# one that only breaks when a user runs it.
RUN neon doctor

WORKDIR /work
ENTRYPOINT ["neon"]
CMD ["--help"]
