#include "libneon_rt.h"

// ---- i64 arithmetic ----
//
// `+`, `-`, `*`, and unary `-` wrap on overflow (two's complement, no trap); the
// unsigned round-trip is how C gives that defined behaviour rather than UB. Division and
// remainder trap on a zero divisor and on INT64_MIN / -1, whose true quotient is not
// representable.

int64_t neon_i64_add(int64_t a, int64_t b) {
    return (int64_t)((uint64_t)a + (uint64_t)b);
}

int64_t neon_i64_sub(int64_t a, int64_t b) {
    return (int64_t)((uint64_t)a - (uint64_t)b);
}

int64_t neon_i64_mul(int64_t a, int64_t b) {
    return (int64_t)((uint64_t)a * (uint64_t)b);
}

int64_t neon_i64_div(int64_t a, int64_t b) {
    if (b == 0) {
        neon_trap("division by zero");
    }
    if (a == INT64_MIN && b == -1) {
        neon_trap("integer overflow");
    }
    return a / b;
}

int64_t neon_i64_rem(int64_t a, int64_t b) {
    if (b == 0) {
        neon_trap("division by zero");
    }
    if (a == INT64_MIN && b == -1) {
        neon_trap("integer overflow");
    }
    return a % b;
}

int64_t neon_i64_neg(int64_t a) {
    return (int64_t)(-(uint64_t)a);
}
