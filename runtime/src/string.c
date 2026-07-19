#include "libneon_rt.h"

#include "internal.h"

#include <stdio.h>

// ---- str ----

neon_str neon_str_lit(const char* data, size_t len) {
    neon_str s = {(char*)data, len, NULL}; // static: never freed
    return s;
}

bool neon_str_eq(neon_str a, neon_str b) {
    return a.len == b.len && memcmp(a.data, b.data, a.len) == 0;
}

// Byte-lexicographic order: the shared prefix decides, and if one string is a prefix of
// the other the shorter sorts first. `memcmp`'s sign is only guaranteed meaningful over
// the common length, hence comparing lengths separately rather than over the longer one.
// This is bytes, not codepoints and not collation -- `byte_len`'s naming rule applies.
int neon_str_cmp(neon_str a, neon_str b) {
    size_t n = a.len < b.len ? a.len : b.len;
    int c = n ? memcmp(a.data, b.data, n) : 0;
    if (c != 0) {
        return c < 0 ? -1 : 1;
    }
    return a.len < b.len ? -1 : (a.len > b.len ? 1 : 0);
}

neon_str neon_str_concat(neon_str a, neon_str b) {
    neon_header* h = neon_alloc(a.len + b.len, neon_str_drop);
    char* data = (char*)(h + 1);
    memcpy(data, a.data, a.len);
    memcpy(data + a.len, b.data, b.len);
    neon_str s = {data, a.len + b.len, h};
    neon_release(a.owner);
    neon_release(b.owner);
    return s;
}

// The `+` operator. It borrows both operands -- the IR treats a `prim.add`'s inputs as
// borrowed and releases them itself at their last use -- so this must not release them.
neon_str neon_str_add(neon_str a, neon_str b) {
    neon_header* h = neon_alloc(a.len + b.len, neon_str_drop);
    char* data = (char*)(h + 1);
    memcpy(data, a.data, a.len);
    memcpy(data + a.len, b.data, b.len);
    neon_str s = {data, a.len + b.len, h};
    return s;
}

// ---- string natives (consume their str arguments) ----

// The byte offset of `needle` in `hay`, or -1. An empty needle is found at 0.
static int64_t str_index_of(neon_str hay, neon_str needle) {
    if (needle.len == 0) return 0;
    if (needle.len > hay.len) return -1;
    for (size_t i = 0; i + needle.len <= hay.len; i++) {
        if (memcmp(hay.data + i, needle.data, needle.len) == 0) return (int64_t)i;
    }
    return -1;
}

int64_t neon_str_byte_len(neon_str s) {
    int64_t r = (int64_t)s.len;
    neon_release(s.owner);
    return r;
}

bool neon_str_is_empty(neon_str s) {
    bool r = s.len == 0;
    neon_release(s.owner);
    return r;
}

neon_str neon_str_to_upper(neon_str s) {
    neon_str r = neon_str_new(s.data, s.len);
    for (size_t i = 0; i < r.len; i++) {
        char c = r.data[i];
        if (c >= 'a' && c <= 'z') r.data[i] = (char)(c - 32);
    }
    neon_release(s.owner);
    return r;
}

neon_str neon_str_to_lower(neon_str s) {
    neon_str r = neon_str_new(s.data, s.len);
    for (size_t i = 0; i < r.len; i++) {
        char c = r.data[i];
        if (c >= 'A' && c <= 'Z') r.data[i] = (char)(c + 32);
    }
    neon_release(s.owner);
    return r;
}

neon_str neon_str_repeat(neon_str s, int64_t n) {
    if (n <= 0) {
        neon_release(s.owner);
        return neon_str_lit("", 0);
    }
    size_t total = s.len * (size_t)n;
    neon_header* h = neon_alloc(total, neon_str_drop);
    char* data = (char*)(h + 1);
    for (int64_t i = 0; i < n; i++) memcpy(data + (size_t)i * s.len, s.data, s.len);
    neon_str r = {data, total, h};
    neon_release(s.owner);
    return r;
}

bool neon_str_contains(neon_str s, neon_str needle) {
    bool r = str_index_of(s, needle) >= 0;
    neon_release(s.owner);
    neon_release(needle.owner);
    return r;
}

bool neon_str_starts_with(neon_str s, neon_str prefix) {
    bool r = prefix.len <= s.len && memcmp(s.data, prefix.data, prefix.len) == 0;
    neon_release(s.owner);
    neon_release(prefix.owner);
    return r;
}

bool neon_str_ends_with(neon_str s, neon_str suffix) {
    bool r = suffix.len <= s.len &&
             memcmp(s.data + s.len - suffix.len, suffix.data, suffix.len) == 0;
    neon_release(s.owner);
    neon_release(suffix.owner);
    return r;
}

// A byte slice: `str` is byte-indexed throughout (`byte_len`, `find`), so this cuts at
// byte offsets and may split a UTF-8 sequence — the caller asked for bytes.
neon_str neon_str_slice_unchecked(neon_str s, int64_t from, int64_t to) {
    neon_str r = neon_str_new(s.data + from, (size_t)(to - from));
    neon_release(s.owner);
    return r;
}

// The single byte at `i`. `str` is byte-indexed throughout, so this indexes bytes and may
// land inside a UTF-8 sequence — the same contract as `slice` and `find`.
neon_str neon_str_char_at_unchecked(neon_str s, int64_t i) {
    neon_str r = neon_str_new(s.data + i, 1);
    neon_release(s.owner);
    return r;
}

int64_t neon_str_index_of(neon_str s, neon_str needle) {
    int64_t r = str_index_of(s, needle);
    neon_release(s.owner);
    neon_release(needle.owner);
    return r;
}

// Whether the whole string is a decimal integer, optionally signed. Kept separate from
// parsing so the Neon wrapper decides what to throw.
bool neon_str_is_int(neon_str s) {
    size_t i = 0;
    if (s.len > 0 && (s.data[0] == '-' || s.data[0] == '+')) i = 1;
    bool any = false;
    for (; i < s.len; i++) {
        if (s.data[i] < '0' || s.data[i] > '9') {
            neon_release(s.owner);
            return false;
        }
        any = true;
    }
    neon_release(s.owner);
    return any;
}

int64_t neon_str_parse_int(neon_str s) {
    int64_t sign = 1, v = 0;
    size_t i = 0;
    if (s.len > 0 && (s.data[0] == '-' || s.data[0] == '+')) {
        sign = s.data[0] == '-' ? -1 : 1;
        i = 1;
    }
    for (; i < s.len; i++) {
        v = (int64_t)((uint64_t)v * 10 + (uint64_t)(s.data[i] - '0'));
    }
    neon_release(s.owner);
    return (int64_t)((uint64_t)v * (uint64_t)sign);
}

// ---- to-string natives ----

neon_str neon_i64_to_string(int64_t n) {
    char buf[24];
    int len = snprintf(buf, sizeof buf, "%lld", (long long)n);
    return neon_str_new(buf, (size_t)len);
}

neon_str neon_f64_to_string(double x) {
    char buf[32];
    int len = snprintf(buf, sizeof buf, "%g", x);
    return neon_str_new(buf, (size_t)len);
}

neon_str neon_bool_to_string(bool b) {
    return neon_str_lit(b ? "true" : "false", b ? 4 : 5);
}

neon_str neon_str_to_string(neon_str s) {
    return s; // identity; ownership passes through
}

// `join` builds a string out of a `List[str]`, so it lives with the other string
// constructors rather than with the list natives -- it is the only list-taking function
// that allocates a `neon_str`.
neon_str neon_str_join(neon_list* parts, neon_str sep) {
    size_t total = 0;
    for (size_t i = 0; i < parts->len; i++) {
        total += ((neon_str*)parts->data)[i].len;
    }
    if (parts->len > 1) total += sep.len * (parts->len - 1);

    neon_header* h = neon_alloc(total, neon_str_drop);
    char* data = (char*)(h + 1);
    size_t off = 0;
    for (size_t i = 0; i < parts->len; i++) {
        if (i > 0) {
            memcpy(data + off, sep.data, sep.len);
            off += sep.len;
        }
        neon_str e = ((neon_str*)parts->data)[i];
        memcpy(data + off, e.data, e.len);
        off += e.len;
    }
    neon_str s = {data, total, h};
    neon_release((neon_header*)parts); // consumes parts (drops its str elements)
    neon_release(sep.owner);
    return s;
}
