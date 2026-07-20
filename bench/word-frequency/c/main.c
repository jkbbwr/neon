// Word-frequency: string building, string hashing, and hash-map upserts. The map is a
// plain open-addressed table with FNV-1a, grown at 70% load — an ordinary hand-rolled
// C map, not a tuned library.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

typedef struct {
    char *key;
    int64_t count;
} Slot;

static Slot *slots;
static size_t cap, used;

static uint64_t hash(const char *s) {
    uint64_t h = 1469598103934665603ULL;
    for (; *s; s++) {
        h ^= (unsigned char)*s;
        h *= 1099511628211ULL;
    }
    return h;
}

static void grow(void);

static void bump(const char *word) {
    if (used * 10 >= cap * 7) grow();
    size_t i = hash(word) & (cap - 1);
    while (slots[i].key) {
        if (strcmp(slots[i].key, word) == 0) {
            slots[i].count++;
            return;
        }
        i = (i + 1) & (cap - 1);
    }
    size_t len = strlen(word) + 1;
    slots[i].key = malloc(len);
    memcpy(slots[i].key, word, len);
    slots[i].count = 1;
    used++;
}

static void grow(void) {
    size_t ocap = cap;
    Slot *old = slots;
    cap = cap ? cap * 2 : 16384;
    slots = calloc(cap, sizeof(Slot));
    used = 0;
    for (size_t i = 0; i < ocap; i++) {
        if (!old[i].key) continue;
        size_t j = hash(old[i].key) & (cap - 1);
        while (slots[j].key) j = (j + 1) & (cap - 1);
        slots[j] = old[i];
        used++;
    }
    free(old);
}

int main(void) {
    int64_t x = 42;
    const int64_t n = 10000000;
    char word[32];

    for (int64_t i = 0; i < n; i++) {
        x = (x * 48271) % 2147483647;
        snprintf(word, sizeof word, "w%lld", (long long)(x % 10000));
        bump(word);
    }

    int64_t max = 0;
    size_t distinct = 0;
    for (size_t i = 0; i < cap; i++) {
        if (!slots[i].key) continue;
        distinct++;
        if (slots[i].count > max) max = slots[i].count;
    }

    printf("Result: %zu %lld %lld\n", distinct, (long long)n, (long long)max);
    return 0;
}
