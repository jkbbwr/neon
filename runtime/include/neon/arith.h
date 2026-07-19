#ifndef NEON_ARITH_H
#define NEON_ARITH_H

// Trapping i64 arithmetic: overflow and division by zero are traps, not wraparound.
// The IEEE f64 side lives in `neon/math.h`, which does not trap.

#include <stdint.h>

int64_t neon_i64_add(int64_t a, int64_t b);
int64_t neon_i64_sub(int64_t a, int64_t b);
int64_t neon_i64_mul(int64_t a, int64_t b);
int64_t neon_i64_div(int64_t a, int64_t b);
int64_t neon_i64_rem(int64_t a, int64_t b);
int64_t neon_i64_neg(int64_t a);

#endif
