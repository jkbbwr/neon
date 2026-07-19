#ifndef NEON_IO_H
#define NEON_IO_H

// Standard streams. File descriptors are `neon/file.h`.

#include "neon/core.h"

void neon_io_println(neon_str s);  // consumes s
void neon_io_print(neon_str s);    // consumes s; no trailing newline
void neon_io_eprintln(neon_str s); // consumes s; stderr

#endif
