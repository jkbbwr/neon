# Modules and visibility

Module paths come from file paths (`stdlib/std/io.neon` is `std::io`) and from inline
`mod` blocks, which nest inside the module that contains them.

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
passes through — not while checking a `use`. Import-time enforcement is bypassable by
writing the path out in full:

    outer::raw::secret()      // no `use` in sight

A failed lookup distinguishes the two cases, because "nothing named `secret` is in scope"
is actively misleading when the name exists:

    `outer::raw::secret` is internal to `outer::raw` and cannot be used from here

An `internal mod` at the root of a program is vacuous: its parent is the root, so its
subtree is everything. Harmless — there is nothing outside it to keep out.

## Not yet

- **Sealing: one owner per module path.** The module namespace is open — anyone may declare
  a module at any path, and same-path declarations merge. So the fence above is defeated by
  declaring yourself inside it:

        mod outer { internal mod raw { fn secret() -> i64 { 42 } } }
        mod outer { mod raw { fn hijack() -> i64 { secret() } } }   // accepted

  and user code can declare itself into the stdlib's namespace outright:

        mod std { mod collections { mod list { fn intruder() -> i64 { 7 } } } }  // accepted

  Note this is *not* a consequence of the subtree rule — parent-only is equally hijackable,
  since you would simply declare yourself at the parent path instead. The cause is that
  module paths are open strings that merge.

  The fix is to seal them: an explicit `mod X` whose path another unit already declared is
  an error. That closes hijacking, accidental clobbering, and the surprise of two files
  silently sharing a namespace. One wrinkle — intermediate segments are established
  implicitly by the file mapping (`std/io.neon` and `std/collections/list.neon` both imply
  `std`), so sealing must apply to *explicit* `mod` declarations and not to implied
  ancestors, or the stdlib conflicts with itself.

  **Deferred deliberately.** With one program plus the stdlib, reaching into internals is
  self-harm, and the fence still does its real job: keeping `list::set_unchecked` from
  becoming an API people reach for by accident. It becomes a genuine breach only when
  third-party packages exist — someone else's code declaring itself into your namespace —
  so it belongs with the package work, where "unit" also has to be defined.

- **Per-item visibility.** A module with mostly-public contents and one private function has
  to split that function into an `internal mod`. Fine for the stdlib; may not stay fine.
