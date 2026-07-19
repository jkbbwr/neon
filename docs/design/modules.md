# Modules and visibility

Module paths come from file paths (`stdlib/std/io.neon` is `std::io`) and from inline
`mod` blocks, which nest inside the module that contains them.

A module path is not just a namespace. It is the **identity** two other rules are decided
against: whether an `internal mod`'s contents are reachable, and whether an `opaque`
record's contents are. Both compare the accessing module's path with the declaring one.

## `internal mod`

One visibility mechanism, module-granular. There is no per-item modifier.

    // std::collections::list
    internal mod raw {
        @native("neon_list_set") fn set_unchecked[T](xs: List[T], i: i64, v: T) -> List[T]
    }

    fn set[T](xs: List[T], i: i64, v: T) throws IndexError -> List[T] {
        if i < 0 or i >= len(xs) { throw IndexError { message: "..." }; }
        raw::set_unchecked(xs, i, v)
    }

Names inside an `internal mod` resolve **only from the subtree rooted at the module that
declared it**. `std::collections::list::raw` resolves from `std::collections::list` and
anything beneath it, and nowhere else. A path segment literally named `internal` marks a
directory the same way, with the same rule.

Subtree rather than parent-only, so two siblings can share an internal helper — a hashing
or witness routine wanted by both `list` and `map` goes in `std::collections::internal` and
is reachable from both. Parent-only would force it up into the parent and out of reach of
the modules that need it.

**Enforced while resolving a name**, in `Env::candidates` — the choke point every lookup
passes through — not while checking a `use`. `candidates` calls `visible_from` and *drops*
what the caller may not reach, so the filter is the enforcement and no lookup built on it
can return an internal name by accident. Import-time enforcement would be bypassable by
writing the path out in full:

    outer::raw::secret()      // no `use` in sight

A failed lookup distinguishes the two cases, because "nothing named `secret` is in scope"
is actively misleading when the name exists. `candidates_unfiltered` re-runs the lookup
without the filter, and `hidden_by_internal` reports the owner:

    `outer::raw::secret` is internal to `outer::raw` and cannot be used from here

An `internal mod` at the root of a program is vacuous: its parent is the root, so its
subtree is everything. Harmless — there is nothing outside it to keep out.

One narrow gap in the `internal` *directory* rule: only the **first** such segment is
judged, so `a::internal::b::internal::c` is checked against `a` and not also against
`a::internal::b`. Unreachable from source today — `internal` is a keyword, so no
`mod internal` parses — and it arises only from a library tree with two nested `internal/`
directories, which the stdlib does not have. It becomes a hole the day one appears.

## Opacity: modules decide who may reach inside a record

`opaque record` hides a record's *contents*, not its type. Three routes in are closed —
reading a field, building a literal, destructuring a pattern
(`tests/lang/records/opaque_hides_its_contents.neon`) — while the value itself travels
anywhere: held in a local, passed, returned, stored in another module's record, put in a
list (`tests/lang/records/opaque_values_still_travel.neon`).

The rule is `check.rs::opacity_permits(module, owner)`, and it admits three cases:

- the declaring module itself;
- **anything nested inside it** — an `internal mod` is the implementation of the module
  around it and cannot be barred from the type it exists to implement;
- the **immediate parent**, for the mirror arrangement: `std::fs`'s inner module declares
  the handle and the module above implements the public API over it.

Siblings and grandparents cannot. An empty owner path is the prelude, which has no name to
print. There is an exception for the root, because the prelude and the program share it;
`TODO.md` item 17 notes that moving `List`/`Map` out of the prelude would remove it.

This is what lets a module hold an invariant its callers cannot break — a descriptor that
must not be forged from an integer, a cleanup guard that must not be disarmed behind the
module's back. `std::fs` depends on it; see `docs/design/resources.md`.

*Limitation:* a nested opaque type cannot be **named** from outside, because
`vault::inner::Secret` does not resolve at the root — a module-path resolution limit, not an
opacity one. `tests/lang/records/opaque_values_still_travel.neon` works around it by having
`vault` hand out its own single-level type.

## Sealing: one owner per module path

A module path is a claim the source makes, and until 2026-07-19 it was exploitable. A
program could write

    mod std { mod fs { fn steal(f: std::fs::File) -> ... { f.r } } }

and, as far as the checker could tell, *be inside* `std::fs` — reading the guard out of the
stdlib's opaque `File`. Verified: the same `f.r` at the program's root was refused, and
inside the forged module it compiled and ran.

`Env::claim_module` now refuses the claim. Each module passed to `Env::build_with` is a
**unit**; the first unit to introduce a path owns it, and a second unit declaring a `mod` at
that path is an error:

    `std::fs` is already a module of another library, so this `mod` may not claim that
    path. A module path is an identity: `opaque` decides who may reach inside a record by
    comparing module paths, so claiming `std::fs` would give this code the right to read
    the insides of every opaque type declared there. Give the module a name of your own

Pinned by `tests/lang/types/a_module_path_may_not_be_forged.neon`.

Refusing the *claim* is the containable fix. The alternative is giving every declaration a
provenance and teaching `opacity_permits` to compare that instead.

Three things this does and does not cover, stated because the boundaries matter:

- **Paths within one unit are free.** `std::fs` declaring `internal mod raw` claims
  `std::fs::raw` for its own unit, which is not a collision. The consequence is that the
  same-file hijack still works:

        mod outer { internal mod raw { fn secret() -> i64 { 42 } } }
        mod outer { mod raw { fn hijack() -> i64 { secret() } } }   // still accepted

  Self-harm, not a breach: one unit reaching into its own internals.
- **Only claimed paths collide, and only explicit `mod` claims them.** Intermediate segments
  established by the file mapping are not registered, so a user `mod std { ... }` is fine on
  its own — the collision fires on `std::fs`, the path a stdlib unit actually declared. This
  is exactly the wrinkle an earlier draft of this file predicted would break the stdlib
  against itself; not claiming implied ancestors is what avoids it.
- **The root path is exempt**, because the prelude and the program share it by design.

## Known-broken

- **Nominal identity is a bare name, so `opaque` is decoration in the general case.**
  `typecheck/env.rs::record_body` interns the bare identifier, so two modules declaring
  `record Secret` declare the **same type** — no cast, no `any`, no module-path forgery
  needed. Every opacity guarantee above rests on this. Recorded as
  `tests/lang/types/a_nominal_name_is_not_a_module_identity.neon`, deliberately unlisted in
  `expected-pass.txt`: unlisted-and-failing is how the ratchet records an open bug. The fix
  is not local — the name is read back by `dispatch::nominal_head` and matched against
  `ast_head`'s `path.last()`, and `ordered.rs` matches it against literal `"List"`/`"Map"`.
  `TODO.md` item 1.

## Not yet

- **Per-item visibility.** A module with mostly-public contents and one private function has
  to split that function into an `internal mod`. Fine for the stdlib; may not stay fine.
  `std::fs::fail` and `std::fs::collect` are the current casualties — helpers that read as
  private and are not.
- **Packages.** Sealing is enforced per unit, and "unit" today means "one module handed to
  `Env::build_with`". Defining it properly, and defining what a dependency is, belongs with
  the package work. `Env::check_coherence` already carries a related gap: one orphan-impl
  rule from `docs/decisions.md` cannot be written because ownership is a property of the
  library a declaration came from, and `use` does not load a dependency.
