#ifndef NEON_RESOURCE_H
#define NEON_RESOURCE_H

// Resources: a payload, a cleanup, and an armed flag.
//
// The generalisation of what `neon_file` does by hand. A resource owns a payload and a
// cleanup that runs exactly once: either explicitly, so the caller sees the error, or on
// the last release, where there is no error channel and the failure is discarded. The
// armed flag is what makes those two paths safe together -- disarm-then-act, so a double
// release is a no-op rather than a second `close` landing on a reused descriptor.
//
// The payload is stored inline after the struct, sized by its witness, so a resource is
// one allocation regardless of what it holds.
//
// `cleanup` is the user's closure, which varies per call; the code that knows how to
// *call* it varies per instantiation and is emitted by codegen. That typed code is
// reached through `header.drop`, not through a field of its own: `neon_alloc` already
// takes a per-object drop, and one indirection is enough. `neon_file_drop` is the same
// shape by hand.
//
// Only the drop path needs it. `release` is an ordinary Neon function that disarms, takes
// the payload and calls the closure itself, so the explicit path is fully typed and its
// error propagates normally -- which is the whole reason that path exists.

#include <stdbool.h>

#include "neon/core.h"

typedef struct neon_resource {
    neon_header header;
    const neon_witness* w;
    neon_closure cleanup;
    bool armed;
} neon_resource;

static inline void* neon_resource_payload(neon_resource* r) {
    return (void*)(r + 1);
}

// `drop` is the instantiation's own drop, emitted by codegen: it runs cleanup with a
// typed payload if still armed, then calls `neon_resource_finish`.
neon_resource* neon_resource_new(const void* payload, const neon_witness* w,
                                 neon_closure cleanup, void (*drop)(void*));
// The shared tail of every instantiation's drop: release the payload's counted parts and
// the closure's environment, then free. Split out so the emitted drop is a few lines.
void neon_resource_finish(neon_resource* r);
// The cleanup closure, for the explicit `release` path to call from Neon.
neon_closure neon_resource_cleanup(neon_resource* r);
// Read the payload without consuming the resource. `false` when already released, in
// which case `out` is untouched -- this is what turns use-after-release into a
// diagnosable error rather than a read of a stale descriptor.
bool neon_resource_get(neon_resource* r, void* out);
// Disarm and hand back the payload for the caller to clean up, `false` if already
// disarmed. Disarming *first* is the whole safety property: whoever gets `true` owns the
// cleanup, and there is exactly one of them.
bool neon_resource_disarm(neon_resource* r, void* out);
// Disarm and move the payload out *without* consuming the resource, for the drop path.
// The payload is zeroed at the source, so the release in `neon_resource_finish` cannot
// reach bytes whose ownership has already moved to the caller.
bool neon_resource_take(neon_resource* r, void* out);
bool neon_resource_is_live(neon_resource* r);

#endif
