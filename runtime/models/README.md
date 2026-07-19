# CBMC models

Each model is a directory:

    <name>/main.c        the harness
    <name>/sources.txt   the runtime .c files it verifies, one per line, relative to runtime/

CMake globs `*/main.c`, so adding a model touches no shared file.

    cmake -B build -S .
    cmake --build build --target verify-<name>
    cmake --build build --target verify-all

Include `../support/cbmc_support.h` **first**, before the runtime header. It gives you
`PROVE`, `ASSUME`, `NONDET_UPTO`, the `nondet_*` declarations, and the `_exit`/`abort`
stubs without which CBMC walks off the end of a trap.

---

## The rules

These are not style. Every one of them is here because breaking it cost hours.

### 1. Models verify the shipping source. Never a copy.

`sources.txt` names the real `.c` files and CBMC compiles them with the harness. Do not
paste a simplified `neon_resource_take` into the model.

The predecessor project's models inlined copies of the runtime functions, so they proved
properties about source that no longer existed and kept passing after the real code
changed. The runtime is one translation unit per area precisely so a model can take the two
files it cares about and leave the rest.

### 2. One model, one invariant.

Prefer many small models over one large one. `verify-map-probe-does-not-stop-at-a-tombstone`
beats a single `verify-map` that checks everything the map does.

Three reasons, in order of how much they matter:

- **A small model solves fast.** Solve time is superlinear in what the harness reaches, so
  splitting is not a linear trade — two models are much cheaper than one twice as big.
- **A failure names the property.** `verify-map` failing tells you the map is broken
  somewhere. A named model failing tells you which contract broke.
- **A small model is honest about its bounds.** When one harness covers ten behaviours, the
  bounds needed for the worst one silently narrow the other nine.

### 3. Avoid loops. Where you cannot, bound them with an assumption.

CBMC unrolls every loop to `--unwind` and builds all of it into the formula **before** any
assumption prunes it. So `for (i = 0; i < k; i++)` with a nondeterministic `k` costs the
full bound no matter what you later assume about `k`.

Write a constant guard with an inner break, and constrain the bound:

    ASSUME(n <= 3, "why three is enough to reach every distinct state");
    for (size_t i = 0; i < 3; i++) {
        if (i >= n) break;
        ...
    }

Keep every bound well under `--unwind`, and leave `--unwinding-assertions` on so guessing
too low is a failure rather than a proof that quietly covers less than it claims.

### 4. Stub libc. Verify our code, not the C library.

Anything crossing into libc gets a shadow in the harness that emulates the behaviour the
model needs. Otherwise CBMC spends its budget proving things about `fprintf`.

`cbmc_support.h` already stubs `_exit` and `abort`. Add your own for whatever else your
model reaches — `neon_trap`'s `fprintf`/`fflush` pull a `FILE` into every trap site, and
traps are reachable from every allocation check, which alone exhausted the default
`--object-bits 8` budget in the list model.

CBMC's own `memcpy` is also imprecise with a symbolic byte count: it leaves the copied
bytes unconstrained and every downstream property fails spuriously. That is not
Neon-specific — it reproduces in twenty lines of plain C. Enter such scenarios with
concrete lengths.

### 5. Every `ASSUME` is a hole in the proof. Say why.

`ASSUME`'s second argument is not passed to CBMC; it exists to make you write the reason.
An assumption silently narrows what was verified, so a model that assumes away the case
containing the bug still reports success **and looks like evidence**.

Prefer a literal bound in the harness's control flow over an assumption where you can: a
hardcoded `3` is visible in the code, while an assumption is something CBMC is told.

### 6. A passing model is not a result until you have seen it fail.

Break the code deliberately and confirm the model catches it. `resource/main.c` was
validated this way — three mutations, including the exact use-after-free that had shipped
that morning, each confirmed to fail the model and then reverted.

This matters most when the model finds nothing. "No bug found" and "no bug findable" look
identical in CBMC's output.

### 7. Exercise the case that makes the bug visible, not the case the code happens to use.

The resource model uses a **counted** payload, with a real retain/release witness. Every
`Resource` in the tree holds a plain integer, whose witness has no `release` — and with
that payload all three mutations above pass silently.

The shipped use-after-free survived for exactly this reason. A model built around the
common case would have reproduced the blind spot rather than closing it.

---

## Performance, and what it costs you

Sequence *depth* is the expensive dimension, not path count.

When a model releases something CBMC cannot constant-fold, the object becomes symbolically
freed: every later dereference carries that disjunction, and the drop recursion behind it
re-expands to the full unwind depth at each one. In the resource model, adding one
operation ahead of the final one took a run from **0.45s to over 300s**, and cutting the
choices at that position from seven to two changed nothing.

Making a count nondeterministic instead of enumerating it concretely has the same shape:
armed and disarmed merge into one symbolic flag ahead of a six-way switch, each branch with
an inlined drop — **under 1s to over 5 minutes**, covering identical executions.

This is the main argument for rule 2. If a model is slow, splitting it is usually the fix,
and enumerating a choice concretely usually beats letting CBMC explore it.
