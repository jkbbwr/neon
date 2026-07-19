#ifndef NEON_MAP_H
#define NEON_MAP_H

// An open-addressed hash map.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "neon/core.h"
#include "neon/list.h" // neon_map_keys / neon_map_values return lists

// `ctrl` marks each slot empty/tombstone/full, and keys and values live in parallel arrays
// sized by their witnesses. The header is first, so a `neon_map*` is also its
// `neon_header*`.
#define NEON_MAP_EMPTY 0u
#define NEON_MAP_DEAD 1u
#define NEON_MAP_FULL 2u

typedef struct neon_map {
    neon_header header;
    const neon_key_witness* kw;
    const neon_witness* vw;
    size_t len;
    size_t cap;
    unsigned char* ctrl;
    char* keys;
    char* vals;
} neon_map;

neon_map* neon_map_new(const neon_key_witness* kw, const neon_witness* vw);
int64_t neon_map_len(neon_map* m);                                  // consumes m
// `contains` and `set` *consume* their key, like any other native: `set` moves it into the
// table, or drops it when the table already holds that key. `at` and `find` borrow it --
// they are reached through `Op::Index`, whose operands the refcount pass releases itself,
// so releasing here too would double-free.
bool neon_map_contains(neon_map* m, const void* key);               // consumes m and key
neon_map* neon_map_set(neon_map* m, const void* key, const void* val); // consumes m and key
void* neon_map_at(neon_map* m, const void* key);   // borrows both; traps if absent
void* neon_map_find(neon_map* m, const void* key); // borrows both; NULL when absent
bool neon_map_eq(neon_map* a, neon_map* b);        // borrows both; same keys, equal values
neon_map* neon_map_remove(neon_map* m, const void* key); // consumes m and key
neon_list* neon_map_keys(neon_map* m, const neon_witness* w);   // consumes m
neon_list* neon_map_values(neon_map* m, const neon_witness* w); // consumes m

#endif
