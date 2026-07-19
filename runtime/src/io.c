#include "libneon_rt.h"

#include <stdio.h>

// ---- io ----

void neon_io_println(neon_str s) {
    fwrite(s.data, 1, s.len, stdout);
    fputc('\n', stdout);
    neon_release(s.owner); // consumes s
}

void neon_io_print(neon_str s) {
    fwrite(s.data, 1, s.len, stdout);
    neon_release(s.owner); // consumes s
}

// stderr is unbuffered by default, so a diagnostic written here appears even if the
// program traps before stdout is flushed -- which is when a diagnostic matters most.
void neon_io_eprintln(neon_str s) {
    fwrite(s.data, 1, s.len, stderr);
    fputc('\n', stderr);
    neon_release(s.owner); // consumes s
}
