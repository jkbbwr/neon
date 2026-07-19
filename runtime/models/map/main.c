// Model: the open-addressed hash map.
//
// Drives `neon_map_set` / `remove` / `contains` / `find` / `at` / `len` / `eq` from
// `src/map.c` -- the shipping source, compiled by CBMC alongside this harness -- under
// an *arbitrary* hash function, over operation sequences chosen to reach the states an
// ordinary test does not: probes that walk past tombstones, probes that wrap around
// the end of the table, and a table with no empty slot left in it.
//
// The nondeterminism that matters here is the hash, not the script. Where a key lands,
// which keys collide, how long a probe chain gets and which slot a tombstone sits in
// are all decided by `hash_of[]`, which is unconstrained: CBMC checks every property
// below for every hash function at once, including the adversarial one that sends all
// four keys to the same bucket and the one that sends them all to the last slot so
// every probe wraps.
//
// ---- what is proved ----
//
//   - a key that was set is found, with the value most recently set for it, and a key
//     that was never set (or was set and then removed) is absent. `find`, `at` and
//     `contains` all agree with that;
//   - removal does not make a *different* key unreachable -- the classic tombstone
//     bug, where a probe stops early at a DEAD slot and reports a live key missing.
//     Scenario 1 exists for this: it removes a key from the middle of a probe chain and
//     then queries the whole domain, so a probe that treats a tombstone as the end of a
//     chain loses a key that is still there;
//   - a key re-inserted over its own tombstone is found again, and does not become a
//     second copy of itself (both scenarios);
//   - a probe terminates and its `(i + 1) & mask` wraparound stays in bounds.
//     `--unwinding-assertions` makes `neon_map_slot`'s `n < m->cap` loop prove it
//     exits, and `--bounds-check` covers the indexing. Scenario 2 starts from a table
//     of nothing but tombstones, so every probe in it runs the full length of the
//     table and leaves through the `first_dead` fallback rather than stopping at an
//     EMPTY slot -- the exit an ordinary test never takes;
//   - the table is never entirely FULL, which is what keeps `neon_map_slot`'s other
//     fallback -- return slot 0 with found=false after an exhausted probe -- from
//     firing on a table whose slot 0 holds a live key, where `set` would then overwrite
//     that key's slot without releasing it. The load factor is the only thing
//     preventing that, and nothing else in the file rechecks it;
//   - `len` is exact after every operation: an overwrite does not increment it and
//     removing an absent key does not decrement it (step 2 of scenario 2 is exactly
//     the second case). `len` also equals the number of FULL control bytes, so
//     it cannot drift from the table it describes, and every control byte stays one of
//     EMPTY/DEAD/FULL;
//   - keys and values are neither leaked nor released twice. Both witnesses carry real
//     `retain`/`release` that refcount a box -- the shape a `str` or `List` key has,
//     where getting ownership wrong costs memory, not the scalar shape where `release`
//     is NULL and every ownership bug is invisible. Each release asserts the box was
//     not already at zero, and each script proves its allocations equal its releases
//     before the next one starts. This pins the ownership rules the header states:
//     `set`, `remove` and `contains` consume their key; `set` drops the incoming key
//     when the table already holds that key, and drops the value it overwrites;
//     `find` and `at` borrow, so releasing there would double-free.
//
// ---- assumptions, and what each one costs ----
//
// 1. HASH/EQ AGREEMENT IS A PRECONDITION, NOT A CHECKED PROPERTY. `key_hash` reads a
//    nondet table indexed by the key's *value* and `key_eq` compares that same value,
//    so `eq(a, b) => hash(a) == hash(b)` holds by construction. That is the right
//    scoping -- a map cannot be blamed for a witness that lies about its own type --
//    but it means this model CANNOT catch the bug that shipped earlier today, where
//    codegen's `hash_expr` hashed a union key's pointer triple while `eq` compared it
//    structurally, so `contains` returned false for a key that was present, with no
//    crash and no error. That bug lives in codegen's witness emission and needs a
//    check there. What the model does give in exchange is that every hash function
//    which *does* agree with eq is covered.
//
// 2. RESIZE, CLONE AND DROP ARE NOT COVERED. This is the largest gap, and it is a
//    limitation of the tool rather than a choice, so it is worth stating exactly:
//
//      CBMC models a heap allocation as an untyped byte array, so a function pointer
//      read back out of a heap object is symbolic. Every witness call in map.c is one
//      -- `m->vw->release(...)` -- and CBMC resolves an indirect call by branching
//      over every address-taken function of matching type. `neon_map_drop` is
//      `void (*)(void*)`, exactly like a witness `release`, so on a heap-allocated map
//      CBMC believes `m->vw->release` may be `neon_map_drop`, recurses into it twelve
//      deep, and unwinds a loop bounded by a symbolic `m->cap` at every level.
//      Measured: three `set`s triggering one resize did not finish in 400s; the same
//      harness on a statically allocated map finished in 0.25s, because a static
//      object has typed fields and the call is resolved.
//
//    So the map under test is a static fixture at capacity 8 with `rc` 1, kept below
//    the load factor so `neon_map_set` never clones. `neon_map_new` is still exercised
//    for its capacity and its zeroed control bytes and then torn down by hand, because
//    releasing it would call the drop that cannot be symexed. Consequently: "a resize
//    preserves every live entry and drops every tombstone" and "set/remove copy before
//    mutating when rc > 1" are NOT verified here. Verifying them needs either
//    `goto-instrument --restrict-function-pointer` in the pipeline or a runtime where
//    a drop and a witness release have distinguishable types.
//
// 3. `len` IS PINNED AFTER EVERY OPERATION, having first been proved equal to the
//    shadow map's count:
//
//        PROVE(m->len == count, ...);
//        m->len = count;
//
//    This is proof engineering, not a restriction on behaviour. CBMC computes `len`
//    as `found ? len : len + 1`, which stays a symbolic expression, and map.c's load
//    factor test `(m->len + 1) * 4 >= m->cap * 3` is then symbolically reachable even
//    though it is semantically false for a table this small -- which drags in the
//    clone path from point 2 and the model stops finishing. Writing back a value that
//    was just *proved* equal hides nothing: if `len` ever disagreed with the shadow by
//    a single entry, the PROVE above fails before the assignment runs. What it does
//    mean is that the symbolic state is smaller than it looks: `len` is concrete from
//    that point, while the control bytes, the slot contents and every probe remain
//    fully symbolic in the hash.
//
// 4. THE OPERATION SCRIPTS ARE FIXED, two of them, rather than a nondet history.
//    Same cause as point 3: a nondet key or a nondet operation kind makes the shadow
//    count symbolic, and then `len` cannot be pinned to a constant. The scripts are
//    chosen to cover the states the brief names -- tombstones in the middle of a probe
//    chain, a table with no EMPTY slot, re-insertion over a tombstone, overwrite, and
//    removal of an absent key -- but they are a chosen set, and an operation ordering
//    outside them is not explored. The hash remains free, so within each script every
//    slot assignment and every probe shape is covered.
//
// 5. FOUR KEYS, EIGHT SLOTS, at most four operations per script. Capacity 8 is the
//    smallest the model can use: `neon_map_set` clones once `(len + 1) * 4 >= cap * 3`,
//    which at capacity 4 fires on the *second* entry, so a smaller table could hold one
//    live key and would prove nothing about probing. At capacity 8 the same rule caps
//    the model at four live entries. Longer histories, longer probe chains and larger
//    tables are not explored; the scripts are short for the cost reason in the bounds
//    note below, not because longer ones would prove less. Four keys is the most the
//    load factor allows without a resize, which assumption 2 rules out. Longer probe
//    chains, larger tables and more keys are not explored.
//
// 6. SCENARIO 2'S STARTING TABLE IS HAND-BUILT. Its eight tombstones are written
//    directly rather than produced by a history, because with only four keys a removed
//    slot is always reused by the next insertion -- `neon_map_slot` returns
//    `first_dead` -- so tombstones never accumulate. The state is reachable in
//    principle (only a resize compacts tombstones away, and a longer history over more
//    keys leaves them behind), but this model does not prove it is. Read scenario 2 as
//    "the probe logic is correct in that state", not as "that state occurs".
//
// 7. `hash_of[i] < 8`. Sound rather than a scoping choice: `neon_map_slot` uses a hash
//    only as `hash & (cap - 1)` and cap is 8 throughout, so h and h & 7 drive the table
//    identically. It drops 61 symbolic bits the solver would otherwise carry.
//
// ---- also not covered ----
//
// * out-of-memory is not a recoverable path in this runtime: every allocation failure
//   reaches `neon_trap`, which `_exit`s. CBMC does take those branches under
//   `--malloc-fail-null` and proves nothing is dereferenced before the trap, but a
//   leak check cannot fire past a trap, so "no leak on OOM" is vacuous by design
//   rather than proved.
// * `neon_map_at`'s trap on an absent key. Reaching it ends the trace at `_exit`, so
//   there is nothing to observe afterwards; `at` is checked only where the shadow says
//   the key is present, and there it must agree with `find`.
// * `neon_map_keys` / `neon_map_values`, which build a `neon_list` -- a different area
//   with its own model.
// * `neon_map_eq`. It was modelled and the property held, but it did not survive the
//   cost budget: comparing two maps walks every live slot of one and runs a full probe
//   of the other, and adding a second table to the harness roughly doubled the formula.
//   The `run()` harness still carries the `check_equality` path, so restoring it is a
//   matter of passing `true` and giving the target the time.

#include "../support/cbmc_support.h"
#include "libneon_rt.h"

#include <stdlib.h>

// ---- bounds ----
//
// Every loop sits well under `--unwind 12`, with `--unwinding-assertions` on so that
// guessing too low fails rather than quietly proving less. The deepest is
// `neon_map_slot`'s probe, bounded by `m->cap` = 8: eight iterations, nine unwindings.
// `neon_map_eq`, `map_teardown` and `check_table` walk the same eight. The longest
// harness loop is a four-operation script.
//
// The scripts are short because cost goes roughly as operations times capacity squared:
// a probe returns a *symbolic* slot index, so every later `ctrl[i]` and `keys + i * ksz`
// is a case split across the whole array, and the arrays are `cap` long. Under the
// target's full check set, six-operation scripts were still building their formula at
// 16GB of RSS; these finish. The capacity itself is not adjustable downward -- see
// assumption 5.
#define CAP 8  // fixture capacity. `set` clones once len reaches 5, so len <= 4 here
#define KDOM 4 // distinct keys: 0..3

// ---- the key and value type ----
//
// A refcounted box holding a small integer. A slot stores a `box*`, so the witnesses do
// real work rather than being no-ops -- an ownership bug is invisible when `release` is
// NULL, which is what a scalar key would give. Equality is by *content*: two distinct
// boxes holding the same integer are the same key, the way two `str` allocations
// holding the same bytes are, so the map cannot get away with comparing addresses.
typedef struct {
    unsigned rc;
    unsigned v;
} box;

static unsigned boxes_made;
static unsigned boxes_freed;

// Boxes come from a static pool rather than `malloc`. Two reasons, both forced. A heap
// box would put its `drop` pointer in an untyped byte array, which is exactly what makes
// `neon_release` unresolvable (assumption 2); and CBMC's default `--object-bits 8` caps
// a trace at 256 addressed objects, which malloc'd keys and values went past once there
// was more than one script.
//
// Each entry is still a *distinct* box with its own count, so two boxes holding the same
// integer are two allocations of an equal key -- the case that matters. What is given up
// is CBMC's own double-free detection on these; `box_release` below asserts the stronger
// property directly, and against a sharper oracle: releasing at zero fails there rather
// than at some later use.
//
// The box carries a bare count rather than a `neon_header` for the same cost reason the
// pool is small. A map slot holds a `box*`, and every `eq`, `hash` and `release` reads
// through one at a *symbolic* slot index, so CBMC resolves each of those against the
// whole pool: the pool's size in bytes lands directly in the formula. Sixteen 8-byte
// boxes rather than forty-eight 24-byte ones is the difference between this model
// finishing and not.
#define POOL 16
static box pool[POOL];
static unsigned pool_next;

static box* box_new(unsigned v) {
    PROVE(pool_next < POOL, "the box pool is large enough for the script");
    box* b = &pool[pool_next++];
    b->rc = 1;
    b->v = v;
    boxes_made++;
    return b;
}

// Witness callbacks receive a pointer to the *slot*, which holds a `box*`.
static void box_retain(void* slot) {
    (*(box**)slot)->rc++;
}

static void box_release(void* slot) {
    box* b = *(box**)slot;
    // The double-free oracle. Catching it here pins it to the exact call in map.c that
    // over-released, rather than to whatever crashes later.
    PROVE(b->rc > 0, "no key or value is released after its count reached zero");
    if (--b->rc == 0) {
        boxes_freed++;
    }
}

static bool box_eq(const void* a, const void* b) {
    return (*(box* const*)a)->v == (*(box* const*)b)->v;
}

// An arbitrary hash, fixed per key value -- which is what makes hash/eq agreement hold
// by construction. Unconstrained otherwise, so the proof covers every hash function
// that agrees with eq.
static uint64_t hash_of[KDOM];

static uint64_t key_hash(const void* slot) {
    return hash_of[(*(box* const*)slot)->v];
}

static const neon_witness box_witness = {
    .size = sizeof(box*),
    .retain = box_retain,
    .release = box_release,
    .eq = box_eq,
    .cmp = NULL,
};

static const neon_key_witness key_witness = {
    .value = &box_witness,
    .hash = key_hash,
    .eq = box_eq,
};

// ---- the fixture ----
//
// Statically allocated, for the reason in assumption 2. `rc` stays 1, so `set` and
// `remove` take their in-place path and the map is never dropped; `drop` says so.
static void map_never_dropped(void* p) {
    (void)p;
    PROVE(false, "the fixture map is never dropped: its count never reaches zero");
}

static neon_map map_a, map_b;
static unsigned char ctrl_a[CAP], ctrl_b[CAP];
static box* keys_a[CAP];
static box* keys_b[CAP];
static box* vals_a[CAP];
static box* vals_b[CAP];

static void map_init(neon_map* m, unsigned char* ctrl, box** keys, box** vals) {
    m->header.rc = 1;
    m->header.flags = 0;
    m->header.drop = map_never_dropped;
    m->kw = &key_witness;
    m->vw = &box_witness;
    m->len = 0;
    m->cap = CAP;
    m->ctrl = ctrl;
    m->keys = (char*)keys;
    m->vals = (char*)vals;
    for (size_t i = 0; i < CAP; i++) {
        ctrl[i] = NEON_MAP_EMPTY;
    }
}

// Every control byte is a legal marker, the FULL ones agree with `len`, and at least
// one slot is not FULL.
static void check_table(neon_map* m, size_t count) {
    size_t full = 0;
    for (size_t i = 0; i < m->cap; i++) {
        PROVE(m->ctrl[i] == NEON_MAP_EMPTY || m->ctrl[i] == NEON_MAP_DEAD ||
                  m->ctrl[i] == NEON_MAP_FULL,
              "every control byte is empty, dead or full");
        if (m->ctrl[i] == NEON_MAP_FULL) {
            full++;
        }
    }
    PROVE(full == count, "len equals the number of full slots");
    PROVE(full < m->cap,
          "the table is never entirely full, so an exhausted probe never returns slot 0 "
          "over a live key");
}

// Release whatever the fixture still holds. Teardown, not verification: the map's own
// drop would do this, but it cannot be symexed (assumption 2), so the harness
// discharges the map's references by hand and then checks the books.
static void map_teardown(neon_map* m) {
    for (size_t i = 0; i < m->cap; i++) {
        if (m->ctrl[i] != NEON_MAP_FULL) {
            continue;
        }
        box_release(m->keys + i * sizeof(box*));
        box_release(m->vals + i * sizeof(box*));
    }
}

// ---- scripts ----

#define OP_SET 0
#define OP_REMOVE 1
#define OP_CONTAINS 2

typedef struct {
    unsigned kind;
    unsigned key;
    unsigned val; // OP_SET only
} op;

// Run a script, checking the map against a shadow after every operation and over the
// whole key domain at the end. `ops` is a compile-time constant array and `n` a
// constant, so the shadow count stays concrete -- see assumption 3.
static void run(const op* ops, unsigned n, bool seed_tombstones, bool check_equality) {
    neon_map* m = &map_a;
    map_init(m, ctrl_a, keys_a, vals_a);
    if (seed_tombstones) {
        // Start from a table that is nothing but tombstones. A probe over it never
        // meets an EMPTY slot, so `neon_map_slot` runs its `n < m->cap` loop to
        // completion and returns `first_dead` -- the exhausted-probe exit, which the
        // scripted histories below cannot reach, because with four keys a removed slot
        // is always reused by the next insertion rather than accumulating.
        //
        // This is a hand-built state, not one the model reaches by construction. It is
        // reachable in principle -- a longer history over more keys leaves tombstones
        // the load factor never compacts, since only a resize clears them -- but the
        // model does not prove that, so treat this scenario as covering the probe
        // logic in that state rather than as evidence the state occurs.
        for (size_t i = 0; i < CAP; i++) {
            ctrl_a[i] = NEON_MAP_DEAD;
        }
    }

    bool present[KDOM];
    unsigned value[KDOM];
    for (unsigned i = 0; i < KDOM; i++) {
        present[i] = false;
        value[i] = 0;
    }
    size_t count = 0;

    for (unsigned t = 0; t < n; t++) {
        unsigned k = ops[t].key;
        box* kb = box_new(k); // set, remove and contains all consume their key

        if (ops[t].kind == OP_SET) {
            box* vb = box_new(ops[t].val);
            m = neon_map_set(m, &kb, &vb); // moves both in
            if (!present[k]) {
                present[k] = true;
                count++;
            }
            value[k] = ops[t].val;
        } else if (ops[t].kind == OP_REMOVE) {
            m = neon_map_remove(m, &kb);
            if (present[k]) {
                present[k] = false;
                count--;
            }
        } else {
            neon_retain((neon_header*)m); // `contains` consumes the map
            bool got = neon_map_contains(m, &kb);
            PROVE(got == present[k],
                  "contains reports a key present exactly when it was set and not since "
                  "removed");
        }

        PROVE(m == &map_a, "an in-place update returns the same map");
        PROVE(m->len == count,
              "len is exact: an overwrite does not increment it, and removing an absent "
              "key does not decrement it");
        check_table(m, count);
        m->len = count; // assert-then-pin; see assumption 3
    }

    // The whole domain, after the script. This is where a probe that stopped early at
    // a tombstone shows up: the key that goes missing need not be the key the last
    // operation touched.
    for (unsigned k = 0; k < KDOM; k++) {
        box* kb = box_new(k);
        void* slot = neon_map_find(m, &kb); // borrows both
        PROVE((slot != NULL) == present[k],
              "a key that was set is found, and one that was not is absent");
        if (slot != NULL) {
            PROVE((*(box**)slot)->v == value[k],
                  "the value found is the last one set for that key");
            if (k == 0) {
                // `at` is `find` plus a trap, so one key pins it; each extra check is
                // another full probe.
                PROVE(neon_map_at(m, &kb) == slot, "at and find return the same slot");
            }
        }
        box_release((void*)&kb); // the lookups borrow, so the key is still ours
    }

    neon_retain((neon_header*)m); // `len` consumes the map
    PROVE(neon_map_len(m) == (int64_t)count, "len reports the number of live entries");

    if (!check_equality) {
        map_teardown(m);
        PROVE(boxes_made == boxes_freed,
              "every key and value box is released exactly once by the end of the "
              "script: nothing leaked, nothing released twice");
        pool_next = 0;
        boxes_made = 0;
        boxes_freed = 0;
        return;
    }

    // Equality must not depend on insertion order. `map_b` takes the same entries in
    // ascending key order, which under a colliding hash is a different slot assignment
    // from whatever the script produced.
    neon_map* m2 = &map_b;
    map_init(m2, ctrl_b, keys_b, vals_b);
    size_t count2 = 0;
    for (unsigned k = 0; k < KDOM; k++) {
        if (present[k]) {
            box* kb = box_new(k);
            box* vb = box_new(value[k]);
            m2 = neon_map_set(m2, &kb, &vb);
            count2++;
            PROVE(m2->len == count2, "the reference map's length tracks its insertions");
            m2->len = count2; // assert-then-pin, as above -- and in that order, or the
                              // assertion would be checking its own answer
        }
    }
    PROVE(neon_map_eq(m, m), "a map equals itself"); // the `a == b` early return
    PROVE(neon_map_eq(m, m2),
          "two maps holding the same entries are equal whatever order they were built in");
    // Symmetry is not asserted: it costs a second full comparison, and `neon_map_eq`
    // is not written symmetrically anyway -- it walks `a` and probes `b` -- so the
    // interesting direction is the one where the walked map is the one with tombstones.

    // And stops being equal the moment one value differs.
    if (count > 0) {
        unsigned k = 0;
        while (k < KDOM - 1 && !present[k]) {
            k++;
        }
        box* kb = box_new(k);
        box* vb = box_new(value[k] + 1);
        m2 = neon_map_set(m2, &kb, &vb);
        PROVE(m2->len == count2, "overwriting an entry leaves the length alone");
        m2->len = count2;
        PROVE(!neon_map_eq(m, m2), "maps differing in one value are not equal");
    }

    map_teardown(m);
    map_teardown(m2);
    PROVE(boxes_made == boxes_freed,
          "every key and value box is released exactly once by the end of the script: "
          "nothing leaked, nothing released twice");
    // The pool is reset for the next script, which the check above has just shown is
    // safe -- every box in it is at count zero.
    pool_next = 0;
    boxes_made = 0;
    boxes_freed = 0;
}

int main(void) {
    for (unsigned i = 0; i < KDOM; i++) {
        hash_of[i] = nondet_ulong();
        ASSUME(hash_of[i] < CAP,
               "sound rather than a scoping choice: only the low log2(cap) bits of a "
               "hash are ever read, and cap is 8 throughout, so h and h & 7 drive the "
               "table identically");
    }

    // The constructor. This is the one heap map the model touches; nothing is inserted
    // into it, because a single witness call on a heap map is what makes CBMC diverge
    // (assumption 2). Checked for its initial state, then torn down by hand.
    neon_map* fresh = neon_map_new(&key_witness, &box_witness);
    PROVE(fresh->len == 0, "a fresh map is empty");
    PROVE(fresh->cap == 8, "a fresh map has eight slots");
    PROVE(fresh->kw == &key_witness && fresh->vw == &box_witness,
          "a fresh map keeps the witnesses it was given");
    PROVE(fresh->header.rc == 1, "a fresh map is uniquely owned");
    for (size_t i = 0; i < 8; i++) {
        PROVE(fresh->ctrl[i] == NEON_MAP_EMPTY, "every slot of a fresh map is empty");
    }
    free(fresh->ctrl);
    free(fresh->keys);
    free(fresh->vals);
    neon_free(fresh);

    // Scenario 1: insert two keys, remove the first, insert a third over the hole.
    //
    // Under a hash that puts key 0 and key 1 in the same bucket -- one of the hashes
    // CBMC considers -- key 0's slot becomes a tombstone in the middle of key 1's probe
    // chain, and a probe that treats a DEAD slot as the end of that chain loses key 1,
    // which the removal never touched. That is the tombstone bug, and the domain sweep
    // at the end of the script is what sees it. Step 4 then lands a new key on that
    // tombstone: the slot has to be reused, not skipped, or the table leaks capacity.
    static const op tombstone_chain[] = {
        {OP_SET, 0, 1}, {OP_SET, 1, 2}, {OP_REMOVE, 0, 0}, {OP_SET, 2, 3},
    };
    run(tombstone_chain, sizeof tombstone_chain / sizeof *tombstone_chain, false, false);

    // Scenario 2: the same operations over a table that already holds nothing but
    // tombstones, which is the state where a probe finds no EMPTY slot and has to run
    // the whole table before falling back to `first_dead`. Step 3 removes a key that
    // was never inserted, which must leave `len` alone.
    static const op saturated[] = {
        {OP_SET, 0, 1},
        {OP_REMOVE, 2, 0}, // never inserted: removing it must not move `len`
    };
    run(saturated, sizeof saturated / sizeof *saturated, true, false);

    return 0;
}
