//! The optimiser: a pass pipeline over SSA, run to a fixpoint. See `docs/design/ir.md`.
//!
//! The always-on set is small and correctness-preserving: constant folding, dead-code
//! elimination (guided by the effect analysis, so an effectful instruction is never
//! dropped), and CFG cleanup (removing unreachable blocks). It is written so a further
//! pass is an addition, not a redesign.

use super::effects;
use super::ssa::{BlockId, Func, Op, PrimOp, Program, Term, Value};
use std::collections::{HashMap, HashSet};

/// Optimise every function in the program to a fixpoint.
pub fn optimize(program: &mut Program) {
    let pure = effects::analyze(program);
    for f in &mut program.funcs {
        loop {
            let a = const_fold(f);
            let b = dead_code(f, &pure);
            let c = drop_unreachable_blocks(f);
            if !(a || b || c) {
                break;
            }
        }
    }
}

// ---- constant folding ----

/// Fold a primitive op on constant operands into a constant. Returns whether anything
/// changed. Overflow and divide-by-zero are left to the runtime, unfolded.
fn const_fold(f: &mut Func) -> bool {
    let mut ints: HashMap<Value, i64> = HashMap::new();
    let mut bools: HashMap<Value, bool> = HashMap::new();
    for b in &f.blocks {
        for inst in &b.insts {
            match (inst.result, &inst.op) {
                (Some(v), Op::ConstI64(n)) => {
                    ints.insert(v, *n);
                }
                (Some(v), Op::ConstBool(x)) => {
                    bools.insert(v, *x);
                }
                _ => {}
            }
        }
    }

    let mut changed = false;
    for b in &mut f.blocks {
        for inst in &mut b.insts {
            if let Op::Prim(op, args) = &inst.op {
                if let Some(folded) = fold_prim(*op, args, &ints, &bools) {
                    inst.op = folded;
                    changed = true;
                }
            }
        }
    }
    changed
}

fn fold_prim(
    op: PrimOp,
    args: &[Value],
    ints: &HashMap<Value, i64>,
    bools: &HashMap<Value, bool>,
) -> Option<Op> {
    match (op, args) {
        (PrimOp::Neg, [a]) => ints.get(a).map(|n| Op::ConstI64(n.wrapping_neg())),
        (PrimOp::Not, [a]) => bools.get(a).map(|x| Op::ConstBool(!x)),
        (_, [a, b]) => {
            if let (Some(&x), Some(&y)) = (ints.get(a), ints.get(b)) {
                return fold_int(op, x, y);
            }
            if let (Some(&x), Some(&y)) = (bools.get(a), bools.get(b)) {
                return fold_bool(op, x, y);
            }
            None
        }
        _ => None,
    }
}

fn fold_int(op: PrimOp, x: i64, y: i64) -> Option<Op> {
    Some(match op {
        PrimOp::Add => Op::ConstI64(x.checked_add(y)?),
        PrimOp::Sub => Op::ConstI64(x.checked_sub(y)?),
        PrimOp::Mul => Op::ConstI64(x.checked_mul(y)?),
        PrimOp::Div => Op::ConstI64(x.checked_div(y)?),
        PrimOp::Rem => Op::ConstI64(x.checked_rem(y)?),
        PrimOp::Band => Op::ConstI64(x & y),
        PrimOp::Bor => Op::ConstI64(x | y),
        PrimOp::Bxor => Op::ConstI64(x ^ y),
        PrimOp::Eq => Op::ConstBool(x == y),
        PrimOp::Ne => Op::ConstBool(x != y),
        PrimOp::Lt => Op::ConstBool(x < y),
        PrimOp::Le => Op::ConstBool(x <= y),
        PrimOp::Gt => Op::ConstBool(x > y),
        PrimOp::Ge => Op::ConstBool(x >= y),
        _ => return None,
    })
}

fn fold_bool(op: PrimOp, x: bool, y: bool) -> Option<Op> {
    Some(match op {
        PrimOp::And => Op::ConstBool(x && y),
        PrimOp::Or => Op::ConstBool(x || y),
        PrimOp::Eq => Op::ConstBool(x == y),
        PrimOp::Ne => Op::ConstBool(x != y),
        _ => return None,
    })
}

// ---- dead-code elimination ----

/// Remove instructions whose result is unused and whose op is pure. Effectful
/// instructions stay even when their result is dead. Returns whether anything changed.
fn dead_code(f: &mut Func, pure: &HashMap<String, bool>) -> bool {
    let used = used_values(f);
    let mut changed = false;
    for b in &mut f.blocks {
        let before = b.insts.len();
        b.insts.retain(|inst| {
            let dead = inst.result.is_some_and(|v| !used.contains(&v));
            let removable = dead && !effects::op_is_effectful(&inst.op, pure);
            !removable
        });
        changed |= b.insts.len() != before;
    }
    changed
}

/// Every value read by an instruction operand, a terminator, or a branch's block args.
fn used_values(f: &Func) -> HashSet<Value> {
    let mut used = HashSet::new();
    let mut note = |v: &Value| {
        used.insert(*v);
    };
    for b in &f.blocks {
        for inst in &b.insts {
            op_operands(&inst.op, &mut note);
        }
        term_operands(&b.term, &mut note);
    }
    used
}

fn op_operands(op: &Op, f: &mut impl FnMut(&Value)) {
    match op {
        Op::Prim(_, vs) | Op::MakeTuple(vs) | Op::MakeList(vs) => vs.iter().for_each(f),
        Op::Call { args, .. } | Op::Native { args, .. } => args.iter().for_each(f),
        Op::CallClosure { callee, args } => {
            f(callee);
            args.iter().for_each(f);
        }
        Op::MakeClosure { captures, .. } => captures.iter().for_each(f),
        Op::MakeRecord { fields, .. } => fields.iter().for_each(|(_, v)| f(v)),
        Op::Field { base, .. } | Op::Elem { base, .. } => f(base),
        Op::Index { base, index } => {
            f(base);
            f(index);
        }
        Op::Cast(v)
        | Op::IsNull(v)
        | Op::IsErr(v)
        | Op::UnwrapOk(v)
        | Op::UnwrapErr(v)
        | Op::IsVariant { value: v, .. }
        | Op::Retain(v)
        | Op::Release(v) => f(v),
        Op::ConstI64(_)
        | Op::ConstF64(_)
        | Op::ConstBool(_)
        | Op::ConstStr(_)
        | Op::ConstNull
        | Op::ConstUnit
        | Op::ConstAtom(_) => {}
    }
}

fn term_operands(term: &Term, f: &mut impl FnMut(&Value)) {
    match term {
        Term::Ret(Some(v)) | Term::Throw(v) => f(v),
        Term::Ret(None) | Term::Unreachable => {}
        Term::Jump(t) => t.args.iter().for_each(f),
        Term::Branch { cond, then, els } => {
            f(cond);
            then.args.iter().for_each(&mut *f);
            els.args.iter().for_each(f);
        }
        Term::Switch { on, arms, default } => {
            f(on);
            for (_, t) in arms {
                t.args.iter().for_each(&mut *f);
            }
            default.args.iter().for_each(f);
        }
    }
}

// ---- CFG cleanup ----

/// Drop blocks unreachable from the entry. Returns whether anything changed.
fn drop_unreachable_blocks(f: &mut Func) -> bool {
    let mut reachable = HashSet::new();
    let mut stack = vec![f.entry];
    while let Some(id) = stack.pop() {
        if !reachable.insert(id) {
            continue;
        }
        for succ in successors(&f.block(id).term) {
            stack.push(succ);
        }
    }
    if reachable.len() == f.blocks.len() {
        return false;
    }
    f.blocks.retain(|b| reachable.contains(&b.id));
    true
}

fn successors(term: &Term) -> Vec<BlockId> {
    match term {
        Term::Jump(t) => vec![t.to],
        Term::Branch { then, els, .. } => vec![then.to, els.to],
        Term::Switch { arms, default, .. } => {
            arms.iter().map(|(_, t)| t.to).chain(std::iter::once(default.to)).collect()
        }
        Term::Ret(_) | Term::Throw(_) | Term::Unreachable => vec![],
    }
}

#[cfg(test)]
mod tests;
