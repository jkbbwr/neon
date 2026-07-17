# Design: the type checker

**Status:** proposed. Nothing built yet.

## The bet

Types are **sets of values**. Union, intersection and negation mean what they mean in set
theory, and subtyping is containment:

    s <: t   ⟺   s ∧ ¬t  is empty

So the whole checker rests on one question — **is this type empty?** — and everything else
is bookkeeping. This is the Frisch–Castagna–Benzaken semantic-subtyping approach (CDuce,
and Castagna's later work on union/intersection/negation types).

It is a real bet. It buys `T | null` with no Option, `{name: str}` structural parameters
that nominal records satisfy, exhaustiveness that falls out of `s ∧ ¬covered = ∅`, and
`:ok | :err` unions. It costs a decision procedure that is genuinely subtle.

## v1 scope

All four of these, because each one is load-bearing and adding them later means
rewriting the decision procedure:

- **Arrows** (`(A) -> B`), with contravariant parameters. Closures and protocol methods
  need them on day one.
- **Full negation** (`!T`). Negation is what makes emptiness the whole game; without it
  subtyping could be a structural walk.
- **μ-types**, which need coinduction.
- **Atom singletons** (`:ok` as a type), which interact with exhaustiveness.

## What the previous implementation got wrong, structurally

Worth stating because it dictates the design.

It put **every kind of atom in one BDD** — primitives, records, arrows, tuples, type
variables. The consequence is `is_path_satisfiable`: the algorithm carries a *path* of
atom assumptions down the tree and, at every `Any` leaf, re-buckets that whole path by
kind to decide satisfiability. Two things follow:

1. **It cannot memoise.** Its own comment says so: *"We intentionally do NOT memoize
   results here. The is_any() terminal's result is path-sensitive."* That is true of that
   algorithm, and it is why the checker is exponential.
2. **It cannot do recursion.** There is no coinduction in `is_empty` at all. Deciding
   `mu type A = :ok | List[A]` requires assuming a recursive query and checking for
   contradiction; there is nowhere to put the assumption.

Both are the same root cause: mixing kinds forces path-sensitivity, and path-sensitivity
forbids memoisation, and without memoisation there is no fixpoint to be coinductive about.

## The design

### 1. Separate by kind

A type is not one BDD. It is a **descriptor** with one field per kind, each a BDD over
only that kind's atoms:

    struct Descriptor {
        base:    BaseSet,        // i64, str, bool, f64, null, ... — a bitmask
        atoms:   AtomSet,        // :ok, :err — a finite-or-cofinite set of names
        records: Bdd<RecordAtom>,
        tuples:  Bdd<TupleAtom>,
        arrows:  Bdd<ArrowAtom>,
    }

Union, intersection and negation are **field-wise**. Emptiness is **every field empty**,
decided independently. An `i64` is never a record, so the kinds never interact, and no
path has to be carried anywhere.

This is the whole fix. Each kind's emptiness depends only on that kind's atoms, so it
memoises on the node.

`base` is a bitmask because the primitives are a fixed finite set. `atoms` is
finite-or-cofinite (`{:ok, :err}` or "everything except `{:ok}`"), because atom names are
countably infinite but any one type mentions finitely many.

### 2. Emptiness, per kind

- **base**: mask == 0.
- **atoms**: the set is empty (or, if cofinite, never — its complement is finite).
- **records / tuples**: collect the BDD path's positive and negative atoms, then decompose
  field-wise. `⋀ᵢ{ℓ: tᵢ} ∧ ⋀ⱼ¬{ℓ: sⱼ}` is empty iff for every way of assigning each
  negative to a field it must differ on, some field's intersection is empty.
- **arrows**: the hard one.

      ⋀_{i∈P}(sᵢ→tᵢ)  ≤  (s→t)
      iff  ∀ P' ⊆ P:  s ≤ ⋁_{i∈P'} sᵢ   or   ⋀_{i∈P∖P'} tᵢ ≤ t

  Exponential in the number of positive arrows in one intersection, which in real programs
  is one or two. Cite: Frisch, Castagna, Benzaken, *Semantic subtyping* (JACM 2008), §4.

### 3. Recursion is coinductive

`mu type A = :ok | List[A]` becomes a `TypeRef` atom plus a side table — never inlined, so
the atom's identity does not depend on the declaration being resolved yet. This is
**equi-recursive**: `A` and its unfolding are the same type. No fold/unfold, no tag, no
allocation.

Emptiness carries an **assumption set**. On re-entering a query already in progress,
return "empty" and continue. If the derivation completes without contradiction, the
assumption was consistent and the type really is empty. That is the standard treatment:
assume the goal, look for a contradiction.

**Contractivity is what makes this terminate**, and it is checked at the declaration, not
here — every recursive occurrence must sit beneath a constructor, so unfolding always
makes progress. The rules (covariant positions only, no recursion beneath negation, no
mutual recursion in v1) are in `decisions.md`.

### 4. Hash-consing

Descriptors are interned; equality is an id comparison. This is what makes the memo
tables work at all, and it is why the emptiness cache can key on `(node, node)` rather
than on a structural hash.

### 5. Nominal records satisfy structural types

Decided. A `NominalRecord` atom carries its name and generic args; the side table holds
its fields. When a nominal is checked against a structural type, it **expands** to its
field shape. Nominal-vs-nominal stays a name comparison — `Red {}` and `Green {}` are
distinct despite sharing a shape.

`opaque` is module-scoped, so expansion is only permitted where the fields are visible.
The same query can legitimately answer differently in two modules.

### 6. There is no `Erased`, and no way to write one

`any` is **⊤** — the type inhabited by every value. It is not an erasure marker, and the
type language has no way to say "I do not know".

The previous implementation conflated the two: `any` parsed to `TypeSpecKind::Erased`,
which became `Type::Erased`. Once "the top type" and "I could not work it out" are the
same value, every unknown silently becomes `any` — and ~70 of its ~108 `Erased`
constructions were exactly that, a fallback rather than a decision.

Structurally, here:

- `Descriptor` has **no `Erased` variant**. `any` is every kind full. There is nothing to
  fall back *to*, so no fallback can be written.
- When the checker cannot determine a type, it **emits a diagnostic**. It does not return
  a type, because there is no type meaning "unknown" to return.
- Erasure is a **lowering** concern, not a typing one. A value of type ⊤ needs a uniform
  runtime representation; that is a consequence of ⊤, not its meaning, and it is decided
  in codegen. The checker never mentions it.

There is one poison, and it is not erasure:

    Descriptor::Error   // recovery only

It exists so one bad expression does not produce a cascade of downstream complaints, it is
produced **only** where a diagnostic has already been emitted, and it **cannot reach
lowering** — a failed typecheck does not lower. It is a diagnostics device, not a type.
`Error` is not ⊤ and not ⊥: it satisfies nothing and is satisfied by nothing, so it cannot
silently make a constraint pass. (Typing an error expression `never` would be worse than
useless: `never <: T` for every `T`, so every downstream check would vacuously succeed.)

A test asserts that no `Descriptor::Error` survives a successful check, and that the only
route to ⊤ is a user writing `any`.

## Module layout

    typecheck/
      types.rs     Descriptor, atoms, hash-consing, union/intersect/negate
      empty.rs     the emptiness decision procedure, per kind, with the assume-set
      subtype.rs   s <: t  ==  is_empty(s ∧ ¬t)   (thin)
      env.rs       records, aliases, protocols, impls; the mu side table
      resolve.rs   ast::TypeSpec -> Descriptor. Contractivity and covariance live here.
      check/       the checker: walks the AST, computes a type for every expression
      result.rs    TypecheckResult

## The keystone: TypecheckResult carries per-expression types

    pub struct TypecheckResult {
        expr_types: HashMap<ExprId, Descriptor>,   // <- this
        resolved_calls: HashMap<ExprId, ProtocolSelection>,
        resolved_lambdas: HashMap<ExprId, Signature>,
    }

The previous implementation kept only the last two and **threw every expression type
away**. IR lowering then had to re-derive them, which is why `infer.rs` existed; it could
not always succeed, so it fell back to `Erased`; that leaked into `NeonValue` boxing,
which invented vtables, which produced `*_Any` collections with 24-byte slots that `push`
read as 8-byte — an ASan stack-buffer-overflow on every `list::new()`.

One discarded hashmap, four subsystems of consequences. It is the single most important
line in this document.

Keying: nodes need stable identity. The previous implementation keyed on `span.start`,
which is fragile. Give AST nodes an `ExprId` at parse time instead.

## The checker layer is where the soundness holes were

The solver answers subtyping questions correctly; the layer above asked the wrong ones.
All of these were **accepted** by the previous implementation:

    let x: u8 = 999          // literal out of range
    let y: i64 = 1 + 2.5     // no implicit numeric promotion
    -"hi"                    // operator typing
    p.field                  // field access on a partial union

None of these are solver bugs. They are the checker not checking. The rewrite is mostly
this layer, and it needs `expected` threaded downwards — a literal's type depends on what
it is checked *against*, which is also what makes `[[1.0], ["a"]] : Json` work at all.

## Non-goals for v1

- Mutual recursion between μ-aliases (a clear "not yet supported" error).
- Type inference beyond local propagation of an expected type. Signatures are explicit.
- Polymorphic set-theoretic types in their full generality (Castagna's later work). v1
  generics are parametric and monomorphised at the call site.

## Risks

- **Arrows are where this gets hard.** The decomposition is exponential in the number of
  positive arrows in one intersection. Fine in practice, but it is the first place to look
  when something hangs.
- **Coinduction is easy to get subtly wrong.** Assuming the wrong polarity gives an
  unsound "yes". Every recursive test must be checked in *both* directions.
- **The expansion of nominal-to-structural interacts with `opaque` and with contractivity**
  — the same nominal is a data constructor in its own module and an atom outside. Emptiness
  queries are therefore module-relative, which is unusual and easy to forget.
