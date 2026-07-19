#include "libneon_rt.h"

#include <assert.h>

// ---- resources ----
//
// The shared tail of every instantiation's drop. The instantiation's own drop runs
// cleanup -- it is the only code that knows the payload's type and how to call the
// closure -- and then lands here.
//
// Cleanup's failure is discarded on this path: a drop has no error channel, which is
// exactly why an explicit `release` exists.
//
// Resurrection is unreachable by construction: cleanup receives the *payload*, never the
// resource, so it has nothing to store, and captures are by value and sealed. The
// assertion is cheap -- it sits on a path that has just made a syscall -- and would fire
// loudly if the language ever grew mutable shared state.
void neon_resource_finish(neon_resource* r) {
    if (r->w && r->w->release) {
        r->w->release(neon_resource_payload(r));
    }
    if (r->cleanup.env) {
        neon_release(r->cleanup.env);
    }
    assert(r->header.rc == 0 && "a resource was resurrected during cleanup");
    neon_free(r);
}

neon_resource* neon_resource_new(const void* payload, const neon_witness* w,
                                 neon_closure cleanup, void (*drop)(void*)) {
    size_t extra = sizeof(neon_resource) - sizeof(neon_header) + w->size;
    neon_resource* r = (neon_resource*)neon_alloc(extra, drop);
    r->w = w;
    r->cleanup = cleanup;
    r->armed = true;
    memcpy(neon_resource_payload(r), payload, w->size);
    return r;
}

// These consume `r`, like every other native taking a counted pointer: the caller's
// reference moves in, so each releases it before returning. Releasing may be the last
// reference, in which case the drop runs cleanup right here -- which is what last-use ARC
// means, and why the payload is retained before that can happen.
bool neon_resource_get(neon_resource* r, void* out) {
    bool live = r->armed;
    if (live) {
        memcpy(out, neon_resource_payload(r), r->w->size);
        // The caller receives an owned value, like every other reader in this ABI.
        if (r->w->retain) {
            r->w->retain(out);
        }
    }
    neon_release((neon_header*)r);
    return live;
}

// The move-out step, shared by the explicit path and the emitted drop.
//
// Zeroing the source is the whole point. The payload's ownership moves to `out`, and
// `neon_resource_finish` releases whatever is still in the payload slot -- so leaving the
// bytes behind releases them twice. For a scalar payload the witness has no `release` and
// nothing happens, which is why an emitted drop that skipped this ran clean against every
// `Resource[i64, E]` in the tree and use-after-freed the first `Resource[str, E]`.
bool neon_resource_take(neon_resource* r, void* out) {
    if (!r->armed) {
        return false;
    }
    r->armed = false;
    memcpy(out, neon_resource_payload(r), r->w->size);
    memset(neon_resource_payload(r), 0, r->w->size);
    return true;
}

// Disarm *first*: whoever gets `true` owns the cleanup, and there is exactly one of them.
// Consumes `r`, like every other native taking a counted pointer.
bool neon_resource_disarm(neon_resource* r, void* out) {
    bool armed = neon_resource_take(r, out);
    neon_release((neon_header*)r);
    return armed;
}

// Hands back an owned closure, so its environment is retained before `r` goes: releasing
// `r` may be the last reference, and the environment would die with it.
neon_closure neon_resource_cleanup(neon_resource* r) {
    neon_closure c = r->cleanup;
    if (c.env) {
        neon_retain(c.env);
    }
    neon_release((neon_header*)r);
    return c;
}

bool neon_resource_is_live(neon_resource* r) {
    bool live = r->armed;
    neon_release((neon_header*)r);
    return live;
}
