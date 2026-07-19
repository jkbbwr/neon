#include "libneon_rt.h"

#include <stdlib.h>

// ---- lifecycle ----

void neon_rt_init(void) {
    // Nothing yet; a hook for allocator/setup once there is any.
}

void neon_retain(neon_header* h) {
    if (h == NULL || (h->flags & NEON_IMMORTAL)) {
        return;
    }
    h->rc++;
}

void neon_release(neon_header* h) {
    if (h == NULL || (h->flags & NEON_IMMORTAL)) {
        return;
    }
    if (--h->rc == 0) {
        h->drop(h);
    }
}

void* neon_alloc(size_t bytes, void (*drop)(void*)) {
    neon_header* h = malloc(sizeof(neon_header) + bytes);
    if (h == NULL) {
        neon_trap("out of memory");
    }
    h->rc = 1;
    h->flags = 0;
    h->drop = drop;
    return h;
}

void neon_free(void* p) {
    free(p);
}
