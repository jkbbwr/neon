#ifndef NEON_FILE_H
#define NEON_FILE_H

// Files: descriptors, and only descriptors.
//
// The *handle* is `opaque record File` on the Neon side, holding a
// `Resource[i64, IoError]` -- so refcounted cleanup, the armed flag and use-after-close
// detection all come from `neon_resource` (see `neon/resource.h`) rather than being
// open-coded here.
//
// Failure is a value (`-errno`); the one call that returns data as well uses an
// out-parameter, which codegen turns into a tuple.

#include <stdbool.h>
#include <stdint.h>

#include "neon/core.h"
#include "neon/list.h" // neon_io_writev takes the parts as a list

int64_t neon_io_open(neon_str path, int64_t mode);      // consumes path; fd or -errno
int64_t neon_io_close(int64_t fd);                      // 0 or -errno
neon_str neon_io_read_all(int64_t fd, int64_t* err);    // *err: 0 or -errno
int64_t neon_io_writev(int64_t fd, neon_list* parts);   // consumes parts; 0 or -errno
int64_t neon_io_remove(neon_str path);                  // consumes path; 0 or -errno
bool neon_io_exists(neon_str path);                     // consumes path
neon_str neon_io_strerror(int64_t code);                // pure: a code, not hidden state

#endif
