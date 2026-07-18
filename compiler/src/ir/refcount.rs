//! Reference-count insertion, Perceus-style: last-use-driven. See `docs/design/ir.md`.
//!
//! Every counted (pointer-backed) value owns one reference when it is produced. A use is
//! either **consuming** (it takes ownership -- a call argument, a field stored into an
//! aggregate, a returned or branched value) or **borrowing** (it only reads -- a field
//! access, a tag test). The pass walks each block backwards over a cross-block liveness
//! result and inserts:
//!
//! - a `Retain` before a consuming use of a value that is still live afterwards (it needs
//!   its own owned reference), and
//! - a `Release` after the last use of a value that is not moved out (a borrow with no
//!   later use, or a dead result).
//!
//! Because the language is immutable, values are acyclic and this is complete: the last
//! release always runs, and nothing leaks. Moves at last use and `rc == 1` reuse are the
//! refinements the optimiser adds on top; this establishes the balanced baseline.

use super::ssa::{Func, Inst, Op, Program, Term, Value};
use std::collections::{HashMap, HashSet};

pub fn insert(program: &mut Program) {
    for f in &mut program.funcs {
        insert_fn(f);
    }
}

/// Where a value was read out of: a `Field`/`Elem` result *aliases* what its aggregate
/// owns rather than holding a reference of its own. The base must therefore outlive every
/// use of what was read from it — releasing the base the moment the base itself is dead
/// frees the thing the reader is still holding.
fn base_of(f: &Func, ptr: &HashSet<Value>) -> HashMap<Value, Value> {
    let mut out = HashMap::new();
    for b in &f.blocks {
        for inst in &b.insts {
            // Every projection: a field or element read, a cast between a union and one of
            // its variants, and the two tagged-result unwraps. All of them hand back a
            // view into their operand. `Index` is not one — `emit_index` retains what it
            // reads, so that result owns itself.
            let projected = match (inst.result, &inst.op) {
                (Some(v), Op::Field { base, .. } | Op::Elem { base, .. }) => Some((v, *base)),
                (
                    Some(v),
                    Op::Cast(base) | Op::UnwrapOk(base) | Op::UnwrapErr(base),
                ) => Some((v, *base)),
                _ => None,
            };
            if let Some((v, base)) = projected {
                if ptr.contains(&v) && ptr.contains(&base) {
                    out.insert(v, base);
                }
            }
        }
    }
    out
}

/// Extend a live set with the bases every live value was read out of.
fn with_bases(set: &mut HashSet<Value>, base_of: &HashMap<Value, Value>) {
    let mut queue: Vec<Value> = set.iter().copied().collect();
    while let Some(v) = queue.pop() {
        if let Some(&b) = base_of.get(&v) {
            if set.insert(b) {
                queue.push(b);
            }
        }
    }
}

/// Mark a value live, and with it every base it was read out of.
fn mark_live(live: &mut HashSet<Value>, v: Value, base_of: &HashMap<Value, Value>) {
    let mut cur = Some(v);
    while let Some(x) = cur {
        if !live.insert(x) {
            break;
        }
        cur = base_of.get(&x).copied();
    }
}

fn insert_fn(f: &mut Func) {
    let ptr: HashSet<Value> = f.values().filter(|&v| f.value_repr(v).is_counted()).collect();
    if ptr.is_empty() {
        return;
    }
    let bases = base_of(f, &ptr);
    let (live_in, live_out) = liveness(f, &ptr, &bases);
    let (pred_map, moved_in) = predecessors(f);

    for b in &mut f.blocks {
        let mut live: HashSet<Value> = live_out[&b.id].clone();
        // Terminator operands: consuming uses (a returned/branched value is handed on).
        // They are already in `live_out` for values used by successors; a returned value
        // is consumed here, so mark it live so nothing releases it before the return.
        let mut term_uses = Vec::new();
        term_consuming(&b.term, &mut |v| {
            if ptr.contains(v) {
                term_uses.push(*v);
            }
        });
        for v in &term_uses {
            live.insert(*v);
        }
        // A view handed on by the terminator — a block argument, a return, a throw — is
        // consumed just as it would be by a call, so it must materialise a reference of
        // its own. This is where `unwrap_err`'s result escapes into a handler block.
        let term_views: Vec<Value> =
            term_uses.iter().copied().filter(|v| bases.contains_key(v)).collect();

        let mut rev: Vec<Inst> = Vec::new();
        for inst in b.insts.iter().rev() {
            let mut releases_after: Vec<Value> = Vec::new();
            let mut retains_before: Vec<Value> = Vec::new();

            // A dead pointer result is dropped immediately.
            if let Some(v) = inst.result {
                // A view owns nothing, so it is never released on its own account —
                // releasing one destroys what its base still holds. `elem` reading a
                // captured closure out of an environment then released the environment's
                // own copy, and the next call read freed memory.
                if ptr.contains(&v) && !live.contains(&v) && !bases.contains_key(&v) {
                    releases_after.push(v);
                }
                live.remove(&v);
            }

            let (consuming, borrowing) = operand_uses(&inst.op, &ptr);
            for w in borrowing {
                let was_live = live.contains(&w);
                // Mark first and unconditionally: borrowing a view has to keep the base it
                // looks into alive, whether or not the view itself gets released here.
                mark_live(&mut live, w, &bases);
                if !was_live && !bases.contains_key(&w) {
                    // Dead after this borrow: release it once the borrow has read it.
                    releases_after.push(w);
                }
            }
            for w in consuming {
                if live.contains(&w) {
                    // Used again later, so this consume needs its own owned reference.
                    retains_before.push(w);
                } else if bases.contains_key(&w) {
                    // Consuming a *view*. A projection holds no reference of its own — it
                    // looks into what its base owns — so handing it on has to materialise
                    // one. Without this, `unwrap_err` passed as a block argument made the
                    // receiving parameter release a payload the tagged union it was read
                    // from then released again.
                    retains_before.push(w);
                    mark_live(&mut live, w, &bases);
                } else {
                    mark_live(&mut live, w, &bases);
                }
            }

            // Emit in reverse-of-forward order; reversed below to `retains, inst, releases`.
            for v in releases_after {
                rev.push(Inst { result: None, op: Op::Release(v) });
            }
            rev.push(inst.clone());
            for v in retains_before {
                rev.push(Inst { result: None, op: Op::Retain(v) });
            }
        }
        rev.reverse();
        // A block parameter (a function parameter, or a value received on a jump) is an
        // owned reference. If it was never used, `live` no longer holds it: release it at
        // the top so it does not leak.
        let mut head: Vec<Inst> = b
            .params
            .iter()
            .filter(|p| ptr.contains(p) && !live.contains(p))
            .map(|&p| Inst { result: None, op: Op::Release(p) })
            .collect();
        let preds = &pred_map[&b.id];
        if !preds.is_empty() {
            let mut dying: Vec<Value> = live_out[&preds[0]]
                .iter()
                .copied()
                .filter(|v| {
                    !live_in[&b.id].contains(v)
                        && !bases.contains_key(v)
                        && !b.params.contains(v)
                        && !moved_in[&b.id].contains(v)
                        && preds.iter().all(|p| live_out[p].contains(v))
                })
                .collect();
            dying.sort();
            head.extend(dying.into_iter().map(|v| Inst { result: None, op: Op::Release(v) }));
        }
        head.extend(rev);
        for v in term_views {
            head.push(Inst { result: None, op: Op::Retain(v) });
        }
        b.insts = head;
    }
}

/// Live-out per block: the counted values a block's successors still need. Standard
/// backward dataflow; a block's parameters are definitions, not live-in.
#[allow(clippy::type_complexity)]
fn liveness(
    f: &Func,
    ptr: &HashSet<Value>,
    base_of: &HashMap<Value, Value>,
) -> (
    HashMap<super::ssa::BlockId, HashSet<Value>>,
    HashMap<super::ssa::BlockId, HashSet<Value>>,
) {
    let mut live_in: HashMap<_, HashSet<Value>> = f.blocks.iter().map(|b| (b.id, HashSet::new())).collect();
    let mut live_out: HashMap<_, HashSet<Value>> = live_in.clone();

    loop {
        let mut changed = false;
        for b in f.blocks.iter().rev() {
            // live_out = union of successors' live_in, plus the args passed on jumps.
            let mut out = HashSet::new();
            for (succ, args) in successor_edges(&b.term) {
                out.extend(live_in[&succ].iter().copied());
                out.extend(args.into_iter().filter(|v| ptr.contains(v)));
            }
            term_consuming(&b.term, &mut |v| {
                if ptr.contains(v) {
                    out.insert(*v);
                }
            });

            // live_in = (out \ defs) ∪ uses.
            let mut defs: HashSet<Value> = b.params.iter().copied().collect();
            for inst in &b.insts {
                if let Some(v) = inst.result {
                    defs.insert(v);
                }
            }
            #[allow(unused_mut)]
            let mut ins: HashSet<Value> = out.iter().copied().filter(|v| !defs.contains(v)).collect();
            for inst in &b.insts {
                let (c, br) = operand_uses(&inst.op, ptr);
                for w in c.into_iter().chain(br) {
                    if !defs.contains(&w) {
                        ins.insert(w);
                    }
                }
            }

            with_bases(&mut out, base_of);
            with_bases(&mut ins, base_of);
            if out != live_out[&b.id] {
                live_out.insert(b.id, out);
                changed = true;
            }
            if ins != live_in[&b.id] {
                live_in.insert(b.id, ins);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    (live_in, live_out)
}

/// Each block's predecessors, and the values handed to it as block arguments.
#[allow(clippy::type_complexity)]
fn predecessors(
    f: &Func,
) -> (
    HashMap<super::ssa::BlockId, Vec<super::ssa::BlockId>>,
    HashMap<super::ssa::BlockId, HashSet<Value>>,
) {
    let mut preds: HashMap<_, Vec<_>> = f.blocks.iter().map(|b| (b.id, Vec::new())).collect();
    let mut moved: HashMap<_, HashSet<Value>> =
        f.blocks.iter().map(|b| (b.id, HashSet::new())).collect();
    for b in &f.blocks {
        for (succ, args) in successor_edges(&b.term) {
            preds.entry(succ).or_default().push(b.id);
            moved.entry(succ).or_default().extend(args);
        }
    }
    (preds, moved)
}

/// A pointer op's operands split into consuming and borrowing uses.
fn operand_uses(op: &Op, ptr: &HashSet<Value>) -> (Vec<Value>, Vec<Value>) {
    let mut consuming = Vec::new();
    let mut borrowing = Vec::new();
    match op {
        Op::Call { args, .. } | Op::Native { args, .. } | Op::MakeTuple(args) | Op::MakeList(args) => {
            consuming.extend(args.iter().copied())
        }
        Op::CallClosure { callee, args } => {
            // Calling a closure reads it; it does not destroy it, and you may call it
            // again. Marking the callee consumed meant its reference was handed to a call
            // that never released it, so every closure value leaked its environment.
            borrowing.push(*callee);
            consuming.extend(args.iter().copied());
        }
        Op::MakeClosure { captures, .. } => consuming.extend(captures.iter().copied()),
        Op::MakeRecord { fields, .. } => consuming.extend(fields.iter().map(|(_, v)| *v)),
        // Borrows: they read but do not take ownership.
        Op::Field { base, .. } | Op::Elem { base, .. } => borrowing.push(*base),
        Op::Index { base, index } => {
            borrowing.push(*base);
            borrowing.push(*index);
        }
        Op::Cast(v)
        | Op::IsNull(v)
        | Op::IsErr(v)
        | Op::UnwrapOk(v)
        | Op::UnwrapErr(v)
        | Op::IsVariant { value: v, .. } => borrowing.push(*v),
        Op::Prim(_, vs) => borrowing.extend(vs.iter().copied()),
        Op::Retain(_) | Op::Release(_) => {}
        _ => {}
    }
    consuming.retain(|v| ptr.contains(v));
    borrowing.retain(|v| ptr.contains(v));
    (consuming, borrowing)
}

/// A terminator's consuming operands (a returned, thrown, or branched value).
fn term_consuming(term: &Term, f: &mut impl FnMut(&Value)) {
    match term {
        Term::Ret(Some(v)) | Term::Throw(v) => f(v),
        Term::Jump(t) => t.args.iter().for_each(f),
        Term::Branch { then, els, .. } => {
            then.args.iter().for_each(&mut *f);
            els.args.iter().for_each(f);
        }
        Term::Switch { arms, default, .. } => {
            for (_, t) in arms {
                t.args.iter().for_each(&mut *f);
            }
            default.args.iter().for_each(f);
        }
        Term::Ret(None) | Term::Unreachable => {}
    }
}

fn successor_edges(term: &Term) -> Vec<(super::ssa::BlockId, Vec<Value>)> {
    match term {
        Term::Jump(t) => vec![(t.to, t.args.clone())],
        Term::Branch { then, els, .. } => {
            vec![(then.to, then.args.clone()), (els.to, els.args.clone())]
        }
        Term::Switch { arms, default, .. } => arms
            .iter()
            .map(|(_, t)| (t.to, t.args.clone()))
            .chain(std::iter::once((default.to, default.args.clone())))
            .collect(),
        Term::Ret(_) | Term::Throw(_) | Term::Unreachable => vec![],
    }
}

#[cfg(test)]
mod tests;
