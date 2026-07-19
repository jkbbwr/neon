// Model: resources -- arm/disarm, move-out, and the exactly-once cleanup.
//
// Drives `neon_resource_new` / `_take` / `_disarm` / `_get` / `_cleanup` / `_is_live` /
// `_finish` from `src/resource.c` -- the shipping source, compiled by CBMC alongside this
// harness, never a copy -- through every operation from both of the states a resource can
// be in, with the emitted drop running at the end of each.
//
// ---- The payload is counted, on purpose ----
//
// This code use-after-freed earlier today: an emitted drop moved the payload out and
// handed it to a cleanup that consumes it, and then `neon_resource_finish` released the
// same bytes a second time. Nothing caught it, because every `Resource[...]` in the tree
// held a scalar, and a scalar's witness has no `release` -- so the second release was a
// call through a NULL function pointer that never happened. The first `Resource[str, E]`
// found it.
//
// So the payload here is a *counted handle*: a `neon_header*` whose witness retains and
// releases it. Any double-release of the payload slot is therefore a real second
// `neon_release` on a real object, which this model catches as an unbalanced reference
// count. A model with a scalar payload would pass and prove nothing; that is the whole
// point of this file.
//
// ---- What is proved ----
//
//   - cleanup runs EXACTLY ONCE, for every operation that can end a resource's life and
//     from both the armed and the disarmed state, whichever caller wins the disarm;
//   - the payload is released exactly once -- not twice (the use-after-free above) and
//     not zero times (a leak), checked as a reference count so both directions fail;
//   - `neon_resource_take` on an already-disarmed resource returns false and does not
//     write `out`: taking twice is a no-op and the second caller does not get the
//     payload, checked over a run of consecutive takes;
//   - after a take the payload slot is zeroed, so the release in `neon_resource_finish`
//     cannot reach bytes whose ownership has already moved -- this is the assertion that
//     fails if the zeroing is removed;
//   - the same second-caller property for `neon_resource_disarm`, which is the
//     disarm-first safety property: of every caller, exactly one is told it owns cleanup;
//   - the closure environment is retained by `neon_resource_cleanup` before the resource
//     is released -- otherwise `explicit_release` below is a use-after-free -- and is
//     released exactly once overall;
//   - `neon_resource_get` hands back an *owned* payload and does not disarm;
//   - `neon_resource_is_live` agrees with the harness's own tracking of the flag;
//   - the resource itself is freed exactly once (--memory-leak-check);
//   - `neon_resource_finish`'s resurrection assertion holds on every path;
//   - no out-of-bounds access, no invalid dereference, on any of the above.
//
// The `model_drop` / `explicit_release` pair below is what codegen emits per
// instantiation, in the shape codegen emits it. It is harness, not code under test: every
// runtime function it calls is the real one. Substituting the buggy drop -- delete the
// `neon_resource_take` from `model_drop` and read the payload slot directly -- makes this
// model fail, which is the check that it has teeth.
//
// ---- Shape of the harness, and the coverage bound that shapes it ----
//
// `main` runs three independent lives of a resource -- `scenario(0)`, `scenario(1)`,
// `scenario(2)` -- differing in how many bare `neon_resource_take` calls happen first.
// `take` is the only operation that does not consume a reference, so it is the only one
// that can be repeated. Each scenario then performs exactly one *consuming* operation,
// chosen nondeterministically from all six, which takes the last reference and so runs
// the emitted drop from inside itself.
//
// Two things about that shape are load-bearing rather than stylistic, and both cost real
// time to rediscover, so they are recorded here for the next model in this directory:
//
//   1. The take count is a literal at each call site, not a nondet value. Made
//      nondeterministic, the armed and disarmed states merge into one symbolic `armed`
//      ahead of the six-way operation switch, every branch of which has a drop inlined
//      into it -- and CBMC goes from under a second to over five minutes while covering
//      exactly the same executions. Three concrete calls keep each life foldable.
//
//   2. Only one consuming operation runs per scenario. A `neon_release` whose outcome
//      CBMC cannot constant-fold leaves its object *symbolically* freed -- maybe
//      deallocated, maybe not -- and every later dereference carries that disjunction,
//      with the drop recursion behind it (release -> drop -> `neon_resource_finish` ->
//      release -> ...) re-expanded at each one to the full `--unwind` depth over every
//      function-pointer candidate. Measured: one consuming operation ahead of the final
//      one takes this model from 0.45s to over 300s, and it is sequence *depth* that
//      does it, not the number of paths -- cutting the choices at that position from
//      seven to two changed nothing at all.
//
//   A loop written `i < k` for a nondet `k` is the same trap by a third route: CBMC
//   unwinds it `--unwind` times and builds every copy into the formula before the
//   assumption prunes them. There is no such loop here for that reason.
//
// The second point is a genuine coverage bound, listed again under "not proved" below,
// and it is bounded by an argument rather than by hope: only `take` and `disarm` change
// a resource's state, and every operation is covered from both states. Everything else
// is a pure read followed by a release, so a sequence of them differs only in the
// reference count -- and the reference count is exactly what the `lifecycle` model
// verifies, over sequences this one does not need to duplicate.
//
// ---- The mutations this model was checked against ----
//
// A model that has never failed is not evidence. Each of these was introduced, confirmed
// to fail, and reverted:
//
//   - the historical bug: `model_drop` reads the payload slot directly and hands it to
//     the consuming cleanup instead of calling `neon_resource_take` first. Caught, as an
//     unbalanced payload count *and* as a deallocated-object dereference inside
//     `neon_release`.
//   - `neon_resource_take` no longer zeroes the source slot. Caught, both directly and
//     as the double release it causes in `neon_resource_finish`.
//   - `neon_resource_cleanup` no longer retains the environment before releasing the
//     resource. Caught as a use-after-free on the environment in `explicit_release`.
//
// ---- What is deliberately NOT proved ----
//
//   - Two or more consuming operations in sequence, per the bound argued above. A bug
//     needing e.g. `get` then `cleanup` then the drop is outside this model. Since every
//     consuming operation ends its own life here, this also means the model never holds
//     a resource at a reference count above one.
//   - More than two consecutive takes. `armed` is monotone -- once false it never
//     returns to true -- so a third take is in the same state as the second.
//   - Out-of-memory does not appear as a *return* anywhere. `neon_alloc` traps rather
//     than returning NULL, so `neon_resource_new` has no failure path to model. What
//     `--malloc-may-fail --malloc-fail-null` buys here is a check that the trap
//     terminates rather than running on with a NULL header, which the `_exit` stub in
//     cbmc_support.h encodes.
//   - Concurrency. The refcount is a plain `uint64_t` and the runtime is single-threaded;
//     "ordering" above means sequential orderings, not simultaneous ones.
//   - Refcount overflow at 2^64. Unreachable in any finite execution.
//   - The closure's *body*. `cleanup.fn` is called through the pointer, as codegen does,
//     but what a user's cleanup computes is not this file's business -- only that it is
//     invoked once, with a payload it owns.
//   - Payloads other than one counted pointer: larger ones, and witnesses with a `retain`
//     but no `release` or the reverse. `w->size` is still read from the witness by the
//     code under test, so the sizing arithmetic in `neon_resource_new` is exercised, just
//     at a single size.
//
// ---- Assumptions ----
//
// One, and it is an encoding assumption rather than a restriction: see ASSUMPTION 1 at
// its use. Everything else that narrows this model is a literal bound in the harness's
// control flow, stated above and visible in the code, not an assumption CBMC is told.

#include "../support/cbmc_support.h"
#include "libneon_rt.h"

// ---- a counted payload, and a counted closure environment ----

static unsigned payload_drops;
static unsigned env_drops;
static unsigned cleanup_calls;

static void payload_drop(void* p) {
    payload_drops++;
    neon_free(p);
}

static void env_drop(void* p) {
    env_drops++;
    neon_free(p);
}

// The payload's witness. `size` is one pointer and retain/release forward to the
// lifecycle: the shape codegen emits for a `Resource[str, E]`, whose payload carries a
// counted owner.
static void handle_retain(void* elem) {
    neon_retain(*(neon_header**)elem);
}
static void handle_release(void* elem) {
    // The slot is zeroed by `neon_resource_take`, so on the moved-out path this is
    // `neon_release(NULL)` -- a no-op. If that zeroing ever goes away this becomes a
    // second release of a live object, and the payload's reference count at the end of
    // main catches it.
    neon_release(*(neon_header**)elem);
}
static bool handle_eq(const void* a, const void* b) {
    return *(neon_header* const*)a == *(neon_header* const*)b;
}

static const neon_witness handle_witness = {
    sizeof(neon_header*), handle_retain, handle_release, handle_eq, NULL,
};

// ---- the emitted, per-instantiation half ----

// A cleanup closure borrows its environment and CONSUMES its payload. Consuming the
// payload is the case that broke: a cleanup that closes a handle and releases it is the
// normal shape, not an exotic one.
typedef void (*cleanup_fn)(neon_header* env, neon_header* payload);

static void model_cleanup(neon_header* env, neon_header* payload) {
    PROVE(env != NULL, "cleanup receives its environment");
    PROVE(payload != NULL, "cleanup receives a payload");
    cleanup_calls++;
    neon_release(payload); // consumes the payload
}

// What codegen emits as the resource's `drop`: run cleanup if still armed, then land in
// the shared tail. The `neon_resource_take` is load-bearing -- without it the payload is
// released here and again in `neon_resource_finish`.
static void model_drop(void* p) {
    neon_resource* r = (neon_resource*)p;
    neon_header* payload = NULL;
    if (neon_resource_take(r, &payload)) {
        ((cleanup_fn)r->cleanup.fn)(r->cleanup.env, payload);
    }
    neon_resource_finish(r);
}

// What the explicit Neon-level `release` compiles to: take an owned copy of the closure,
// disarm, and call it. Both natives consume a reference, so the caller retains once to
// pay for the second; the net effect is one reference consumed.
//
// This is the shape that catches a missing retain in `neon_resource_cleanup`: were the
// environment handed back unretained, the `neon_resource_disarm` below could be the last
// release and `c.env` would be dangling by the time it is called.
static void explicit_release(neon_resource* r) {
    neon_retain((neon_header*)r);
    neon_closure c = neon_resource_cleanup(r); // consumes one ref; c.env comes back owned
    neon_header* got = NULL;
    bool mine = neon_resource_disarm(r, &got); // consumes the other
    if (mine) {
        PROVE(got != NULL, "a successful disarm yields the payload");
        ((cleanup_fn)c.fn)(c.env, got);
    } else {
        PROVE(got == NULL, "disarm on a disarmed resource leaves out untouched");
    }
    neon_release(c.env);
}

// ---- the harness ----

static neon_header* g_payload;
static neon_header* g_env;
static bool expect_armed;

// A bare `neon_resource_take`: the move-out on its own, as the drop path and any emitted
// "take the payload and clean it up here" both use it. It does NOT consume a reference,
// which is what lets phase one call it repeatedly.
static void do_take(neon_resource* r) {
    neon_header* got = NULL;
    bool mine = neon_resource_take(r, &got);
    PROVE(mine == expect_armed, "take succeeds if and only if the resource is armed");
    if (mine) {
        PROVE(got == g_payload, "take yields the payload that went in");
        PROVE(!r->armed, "take disarms the resource");
        PROVE(*(neon_header**)neon_resource_payload(r) == NULL,
              "take zeroes the payload slot at the source");
        expect_armed = false;
        PROVE(r->cleanup.env == g_env, "the closure survives a take");
        // We now own the payload and so owe it a cleanup, exactly as the emitted code
        // does. Borrowing `r->cleanup.env` is safe: the resource is still alive.
        ((cleanup_fn)r->cleanup.fn)(r->cleanup.env, got);
    } else {
        PROVE(got == NULL, "take on a disarmed resource leaves out untouched");
    }
}

// Each of these consumes exactly one reference to `r`. In this model that is always the
// last one, so each runs the emitted drop before returning.
static void do_final_op(neon_resource* r, unsigned op) {
    if (op == 0) {
        do_take(r);
        neon_release((neon_header*)r);

    } else if (op == 1) {
        // The explicit release path.
        bool was_armed = expect_armed;
        unsigned before = cleanup_calls;
        expect_armed = false;
        explicit_release(r);
        PROVE(cleanup_calls == before + (was_armed ? 1u : 0u),
              "explicit release runs cleanup if and only if it won the disarm");

    } else if (op == 2) {
        // `get`: an owned read that must NOT disarm.
        neon_header* got = NULL;
        bool live = neon_resource_get(r, &got);
        PROVE(live == expect_armed, "get reports liveness");
        if (live) {
            PROVE(got == g_payload, "get yields the payload");
            neon_release(got); // the read was owned, so give it back
        } else {
            PROVE(got == NULL, "get on a released resource leaves out untouched");
        }

    } else if (op == 3) {
        // The closure getter on its own.
        neon_closure c = neon_resource_cleanup(r);
        PROVE(c.env == g_env, "the closure comes back intact");
        neon_release(c.env); // it was handed over retained

    } else if (op == 4) {
        bool live = neon_resource_is_live(r);
        PROVE(live == expect_armed, "is_live agrees with the armed flag");

    } else {
        neon_release((neon_header*)r);
    }
}

// One complete life of one resource: `takes` bare takes, then a nondeterministically
// chosen consuming operation that runs the drop.
//
// `takes` is a literal at every call site, and that is deliberate -- see the header
// comment. Made nondeterministic, the two states a resource can be in merge into one
// symbolic `armed` before the six-way operation switch, every branch of which contains an
// inlined drop; the model then takes over five minutes instead of under a second and
// covers exactly the same executions. Calling it three times with constants keeps each
// life concrete and lets CBMC fold.
static void scenario(unsigned takes) {
    payload_drops = 0;
    env_drops = 0;
    cleanup_calls = 0;

    g_payload = (neon_header*)neon_alloc(0, payload_drop);
    g_env = (neon_header*)neon_alloc(0, env_drop);

    neon_closure cleanup;
    cleanup.fn = (void*)model_cleanup;
    cleanup.env = g_env; // the resource takes ownership of this reference

    // The payload's single reference moves into the resource.
    neon_resource* r =
        neon_resource_new(&g_payload, &handle_witness, cleanup, model_drop);

    // A reference the harness keeps for this whole scenario and releases last. It changes
    // how a double free is *detected*, and is worth stating: with it, an extra release by
    // the code under test shows up as `rc == 0` at the check below rather than as a
    // use-after-free, and a missing one as `rc == 2`. Both directions are caught, and
    // caught at the point of imbalance rather than only once something later happens to
    // touch the freed bytes.
    neon_retain(g_payload);

    PROVE(r->header.rc == 1, "a fresh resource has rc == 1");
    PROVE(r->armed, "a fresh resource is armed");
    PROVE(*(neon_header**)neon_resource_payload(r) == g_payload,
          "the payload is copied into the inline slot");
    expect_armed = true;

    // Phase one: a run of bare takes. Non-consuming, so the reference count is untouched
    // here and the drop cannot fire, which is what lets them be repeated at all.
    if (takes >= 1) {
        do_take(r);
    }
    if (takes >= 2) {
        do_take(r); // the redundant second take: must fail, and must not hand out bytes
    }

    PROVE(r->header.rc == 1, "a bare take does not touch the reference count");

    // Phase two: exactly one consuming operation, which takes the last reference and so
    // runs `model_drop` from inside itself.
    //
    // ASSUMPTION 1: `op` names one of the six operations. A pure encoding assumption --
    // it removes no behaviour, because tags above 5 name nothing. Every operation in the
    // public interface that consumes a reference is in the range.
    unsigned op = NONDET_UPTO(
        5,
        "an operation tag, not a restriction: 0-5 enumerate every public entry point "
        "that consumes a reference (take+release, explicit release, get, cleanup, "
        "is_live, bare release). Values above 5 name no operation at all.");
    do_final_op(r, op);

    // The drop has run and the resource is gone.
    PROVE(cleanup_calls == 1,
          "cleanup runs exactly once, from every state and every operation");
    PROVE(env_drops == 1, "the closure environment is released exactly once");

    // The payload's references are balanced against the harness's pinning reference.
    // `rc == 0` here means it was released once too often -- the use-after-free this
    // model exists to catch; `rc == 2` means once too few, which is a leak.
    PROVE(g_payload->rc == 1, "the payload was released exactly once, by whoever cleaned up");
    PROVE(payload_drops == 0, "the payload is not dropped while the harness still holds it");

    neon_release(g_payload);
    PROVE(payload_drops == 1, "the payload is dropped exactly once, and only at rc == 0");

    // Nothing else is freed by hand. The resource and the environment must both have been
    // reclaimed by the code under test; --memory-leak-check is the assertion.
}

int main(void) {
    scenario(0); // the drop finds the resource armed
    scenario(1); // something took the payload first: the drop finds it disarmed
    scenario(2); // and a redundant second take before that
    return 0;
}
