//! The checker's output, and the contract between typechecking and lowering.
//!
//! Everything here is written once by `check.rs` and read many times by `ir/`. The rule
//! the module exists to enforce is one-directional: **lowering asks, it never derives.**
//! Each map records a decision the checker had the information to make and lowering does
//! not — a solved generic argument, the union a `try` can catch, the declared type of a
//! binding whose initialiser is narrower. Where such a map is missing, lowering has
//! historically guessed, and guessing about a type ends in erasure.
//!
//! Every map is keyed by `ExprId`, which is unique across the whole compilation (see
//! `ast::ids`), so one `TypecheckResult` covers every module and nothing has to be
//! namespaced by file.

use super::dispatch::Resolution;
use super::types::TyId;
use crate::ast::ExprId;
use std::collections::HashMap;

/// What the checker learned, keyed by expression.
///
/// `expr_types` is the keystone. The previous implementation kept only the
/// resolutions and **threw every expression type away**, so IR lowering had to
/// re-derive them — which is why `infer.rs` existed. It could not always succeed,
/// so it fell back to `Erased`; that leaked into `NeonValue` boxing, which invented
/// vtables, which produced `*_Any` collections with 24-byte slots that `push` read
/// as 8 — an ASan stack-buffer-overflow on every `list::new()`.
///
/// One discarded hashmap, four subsystems of consequences. Nothing downstream
/// re-derives or re-resolves anything here.
#[derive(Debug, Default)]
pub struct TypecheckResult {
    expr_types: HashMap<ExprId, TyId>,
    resolved_calls: HashMap<ExprId, Resolution>,
    /// A lambda's inferred signature, as an arrow. Currently redundant: `check.rs`
    /// records the same arrow into `expr_types` for the lambda expression, and lowering
    /// reads it from there. Nothing reads this map.
    resolved_lambdas: HashMap<ExprId, TyId>,
    /// The error type a `try` can catch — the union of what its body throws. Recorded so
    /// lowering gives the handler a *concrete* error parameter; without it the error
    /// channel falls back to `any`, which is erasure leaking in by the back door.
    caught_types: HashMap<ExprId, TyId>,
    /// A generic call's solved type arguments, by parameter name. Recorded because
    /// lowering otherwise re-derives them from the turbofish *syntax*, which carries only
    /// a type's head — enough to mangle a name, not enough to lay one out.
    generic_args: HashMap<ExprId, Vec<(String, TyId)>>,
    /// A `let`'s declared type, keyed on its initialiser. The annotation is the binding's
    /// type -- `let x: i64 | str = 1` binds the union -- but lowering sees only the
    /// initialiser, whose type is the narrow one. Without this the binding was laid out at
    /// the *variant's* repr: `let n: P | :none = :none` became a bare tag, and any later
    /// use expecting the union read the wrong layout.
    declared_types: HashMap<ExprId, TyId>,
}

impl TypecheckResult {
    /// `None` means the checker never visited this expression — which, after a clean
    /// check, means it is not an expression whose value anything can observe. Lowering
    /// treating `None` as "infer it myself" is exactly the failure this module exists to
    /// prevent; treat it as a checker bug instead.
    pub fn ty(&self, e: ExprId) -> Option<TyId> {
        self.expr_types.get(&e).copied()
    }

    /// Every expression's type. The IR walks these to assign a representation to each,
    /// which is why the whole map has to survive rather than just the entries a lowering
    /// walk happens to ask for. Iteration order is a `HashMap`'s, so nothing built from
    /// this may depend on order.
    pub fn types(&self) -> impl Iterator<Item = (ExprId, TyId)> + '_ {
        self.expr_types.iter().map(|(&e, &t)| (e, t))
    }

    /// Which function or impl method a call selected. Keyed on the *call* expression, so
    /// the same callee name resolved differently at two sites stays distinct — that is
    /// the point of recording it, since lowering cannot redo protocol dispatch without
    /// the argument types the checker had.
    pub fn call(&self, e: ExprId) -> Option<&Resolution> {
        self.resolved_calls.get(&e)
    }

    /// See `resolved_lambdas`: nothing calls this, and the same arrow is available from
    /// `ty` for the lambda's own expression id.
    pub fn lambda(&self, e: ExprId) -> Option<TyId> {
        self.resolved_lambdas.get(&e).copied()
    }

    /// A generic call's solved type arguments.
    pub fn generics(&self, e: ExprId) -> Option<&[(String, TyId)]> {
        self.generic_args.get(&e).map(Vec::as_slice)
    }

    /// The declared type of the `let` whose initialiser this is. Note the key is the
    /// *initialiser* expression, not the binding — the binding has no `ExprId` — so
    /// lowering asks this of the value it is about to store.
    pub fn declared(&self, e: ExprId) -> Option<TyId> {
        self.declared_types.get(&e).copied()
    }

    /// Records the *annotation*, not the initialiser's own type — recording the latter
    /// would make this map a no-op. `pub` rather than `pub(super)` only by inconsistency;
    /// the sole caller is `check.rs`.
    pub fn set_declared(&mut self, e: ExprId, t: TyId) {
        self.declared_types.insert(e, t);
    }

    /// The error type a `try` expression's handler receives.
    pub fn caught(&self, e: ExprId) -> Option<TyId> {
        self.caught_types.get(&e).copied()
    }

    /// How many expressions were typed. Only the tests use this, as a coarse assertion
    /// that the checker recorded types at all rather than silently recording none — the
    /// regression the module doc describes was invisible precisely because an empty
    /// result still compiled.
    pub fn len(&self) -> usize {
        self.expr_types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.expr_types.is_empty()
    }

    /// The blanket write is at the end of `check.rs`'s `expr`, so every expression that
    /// goes through `expr` is recorded whatever path produced its type. The other call
    /// sites exist for the few expressions the checker types *without* going through
    /// `expr` — a record-literal field value, which is inferred directly so a mismatch can
    /// name the field, and a call's callee, which is looked up rather than evaluated. Each
    /// is a subexpression lowering will still ask about, so skipping them would leave a
    /// hole in `expr_types`.
    pub(super) fn set_ty(&mut self, e: ExprId, t: TyId) {
        self.expr_types.insert(e, t);
    }

    /// Keyed on the call expression, never on the callee: the same callee resolves
    /// differently at different sites, and the call is what lowering is standing on when
    /// it needs the answer.
    pub(super) fn set_call(&mut self, e: ExprId, r: Resolution) {
        self.resolved_calls.insert(e, r);
    }

    /// Only the parameters the solver pinned down. A generic left unsolved is absent
    /// rather than recorded as `any`, so a missing entry is a real gap and not a lie
    /// lowering would go on to lay out.
    pub(super) fn set_generics(&mut self, e: ExprId, args: Vec<(String, TyId)>) {
        self.generic_args.insert(e, args);
    }

    pub(super) fn set_caught(&mut self, e: ExprId, t: TyId) {
        self.caught_types.insert(e, t);
    }

    /// Called for every lambda, but nothing reads the map back — see `resolved_lambdas`.
    #[allow(dead_code)]
    pub(super) fn set_lambda(&mut self, e: ExprId, t: TyId) {
        self.resolved_lambdas.insert(e, t);
    }
}
