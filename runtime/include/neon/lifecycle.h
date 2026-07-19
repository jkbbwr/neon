#ifndef NEON_LIFECYCLE_H
#define NEON_LIFECYCLE_H

// Runtime startup and the refcount primitives every heap object goes through.

#include <stddef.h>

#include "neon/core.h"

void neon_rt_init(void);
void neon_retain(neon_header* h);
void neon_release(neon_header* h);
void* neon_alloc(size_t bytes, void (*drop)(void*));
void neon_free(void* p);

#endif
