# Cross-library identity and module ownership

**Status: decided, not built.** `use` does not load dependencies — every declaration the
compiler can see today comes from the toolchain's own stdlib or the program being
compiled. There is no cross-library code path to fix. This document exists so the loader
is built against it rather than retrofitted, because the retrofit is the expensive one:
once programs depend on loose behavior, tightening it breaks them.

## The decision

**Identity is `(library, module path, name)`.** `identity.md` decides the last two; this
adds the first.

1. **Library identity must be un-spoofable.** Derived from content — a hash, a lockfile
   entry, a registry-assigned namespace — never from a name the library merely asserts
   about itself in a manifest. A library that can *claim* to be `std` is a library that
   can read the insides of every opaque type `std` declares.
2. **Every module path is owned by exactly one library, intermediate paths included.**
   Not only the leaves a library actually declares. Today `std::fs` is claimed but bare
   `std` is not, so `mod std { mod totally_new { .. } }` is accepted — it grants no
   opacity access (nothing is *owned* by `std` itself) but it is a namespace claim, and
   the same gap at a path that does own something would be an access grant.
3. **Ownership is derived from the dependency graph, not from registration order.**
   `claim_module` currently gives a path to whichever compilation unit registered it
   first. That is sound only because exactly one untrusted unit exists. With two, "first"
   is arbitrary and may be attacker-influenced through manifest ordering.
4. **A library may not declare a module inside another library's path.** The
   generalization of `claim_module`, which is doing considerably more security work than
   its name suggests.

## Why this is load-bearing

Opacity's visibility rule is *subtree*: a record's insides are visible to the module that
declares it and to modules nested inside that one (`opacity.md`). That rule is only as
strong as the impossibility of **declaring yourself** a descendant:

```neon
mod std { mod fs { mod thief { /* now inside File's owner */ } } }
```

Rejected today (`tests/lang/records/opaque_cannot_graft_into_owner_module.neon`), by
`claim_module`, on the strength of rule 3's weakest form. **The subtree rule and the
path-ownership rule are a pair; neither is sound alone.** Everything `opacity.md` builds
rests on a guard whose current implementation does not survive a second untrusted unit.

## What already exists

- `Env::module_unit` — module key to the entry that introduced it, and
  `TypeErrorKind::ModuleCollision`, whose message already states the principle: *"A module
  path is an identity: `opaque` decides who may reach inside a record by comparing module
  paths, so claiming this path would give this code the right to read the insides of every
  opaque type declared there."*
- `Env::PRELUDE` — the prelude at a path no source can write, and resolution that consults
  it last. The pattern generalizes: a unit that must be universally visible but must not
  own the shared namespace gets a reserved path.
- `Env::root_unit` and the unit filter in `push_scope_candidates` — imports and root
  declarations do not cross unit boundaries. Built 2026-07-20 after a program's root `use`
  was found resolving names *inside* the stdlib.

## Deferred, and worth knowing before someone assumes otherwise

**`pub use` does not exist.** A `use` binds a name in a scope and that binding is
inherited by nested scopes; it does *not* make the name a member of the importing module.
`a::List` does not resolve when `a` merely imported `List`. Nothing is broken — the
feature is simply absent — but a facade module (`mod api { pub use ... }`) will not work,
and the prelude's re-exports work through scope inheritance rather than through re-export.
When a facade is wanted, `pub use` is the feature to add, and it should be explicit:
plain `use` staying private is what keeps a module's imports from becoming its API by
accident.
