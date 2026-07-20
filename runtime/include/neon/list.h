#ifndef NEON_LIST_H
#define NEON_LIST_H

// Lists: elements are moved in and out by codegen through the void* slot pointer.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "neon/core.h"

// A list stores its elements inline in `data` (len used of cap slots, each `w->size`
// bytes). The header is first, so a `neon_list*` is also its `neon_header*`.
typedef struct neon_list {
    neon_header header;
    const neon_witness* w;
    size_t len;
    size_t cap;
    char* data;
} neon_list;

neon_list* neon_list_new(const neon_witness* w);
neon_list* neon_list_new_with_capacity(const neon_witness* w, int64_t cap);
int64_t neon_list_len(neon_list* l);                        // consumes l
void* neon_list_at(neon_list* l, int64_t i); // borrows l; slot pointer, traps OOB
neon_list* neon_list_push(neon_list* l, const void* elem);  // consumes l, moves *elem in
neon_list* neon_list_set(neon_list* l, int64_t i, const void* elem); // consumes l, traps OOB
// As `neon_list_set`, for an element the caller knows is NOT refcounted, with `sz`
// constant at the call site. Skips the witness entirely: no release of the overwritten
// slot, and the copy folds to a store. Calling it for a refcounted element leaks.
neon_list* neon_list_set_scalar(neon_list* l, int64_t i, const void* elem, size_t sz);
neon_list* neon_list_concat(neon_list* a, neon_list* b);    // consumes both
int neon_list_cmp(const neon_list* a, const neon_list* b);  // borrows both; -1/0/1
bool neon_list_eq(const neon_list* a, const neon_list* b);  // borrows both

#endif
