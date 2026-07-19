#ifndef NEON_TRAP_H
#define NEON_TRAP_H

// Traps: print + _exit. No unwind, no teardown.

#include "neon/core.h"

_Noreturn void neon_trap(const char* msg);
_Noreturn void neon_panic(neon_str msg);
_Noreturn void neon_unreachable(void);

#endif
