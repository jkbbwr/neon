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
    /// A lambda's inferred signature, as an arrow.
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
    pub fn ty(&self, e: ExprId) -> Option<TyId> {
        self.expr_types.get(&e).copied()
    }

    /// Every expression's type. The IR walks these to assign a representation to each.
    pub fn types(&self) -> impl Iterator<Item = (ExprId, TyId)> + '_ {
        self.expr_types.iter().map(|(&e, &t)| (e, t))
    }

    pub fn call(&self, e: ExprId) -> Option<&Resolution> {
        self.resolved_calls.get(&e)
    }

    pub fn lambda(&self, e: ExprId) -> Option<TyId> {
        self.resolved_lambdas.get(&e).copied()
    }

    /// A generic call's solved type arguments.
    pub fn generics(&self, e: ExprId) -> Option<&[(String, TyId)]> {
        self.generic_args.get(&e).map(Vec::as_slice)
    }

    /// The declared type of the `let` whose initialiser this is.
    pub fn declared(&self, e: ExprId) -> Option<TyId> {
        self.declared_types.get(&e).copied()
    }

    pub fn set_declared(&mut self, e: ExprId, t: TyId) {
        self.declared_types.insert(e, t);
    }

    /// The error type a `try` expression's handler receives.
    pub fn caught(&self, e: ExprId) -> Option<TyId> {
        self.caught_types.get(&e).copied()
    }

    pub fn len(&self) -> usize {
        self.expr_types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.expr_types.is_empty()
    }

    pub(super) fn set_ty(&mut self, e: ExprId, t: TyId) {
        self.expr_types.insert(e, t);
    }

    pub(super) fn set_call(&mut self, e: ExprId, r: Resolution) {
        self.resolved_calls.insert(e, r);
    }

    pub(super) fn set_generics(&mut self, e: ExprId, args: Vec<(String, TyId)>) {
        self.generic_args.insert(e, args);
    }

    pub(super) fn set_caught(&mut self, e: ExprId, t: TyId) {
        self.caught_types.insert(e, t);
    }

    /// No caller yet: `Lambda` is one of the forms the checker does not infer.
    #[allow(dead_code)]
    pub(super) fn set_lambda(&mut self, e: ExprId, t: TyId) {
        self.resolved_lambdas.insert(e, t);
    }
}
