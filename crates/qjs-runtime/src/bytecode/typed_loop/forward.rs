//! Copy forwarding over a compiled program.
//!
//! The builder keeps every operand-stack entry in the register of its depth,
//! so `this[i].pos == obj.pos` copies `this` and the hoisted `obj.pos` into
//! stack registers only to read each once in the next operation -- a boxed
//! copy is a reference-count increment and, when the register is next
//! overwritten, a decrement. This pass lets the consumer read the source
//! register itself and deletes copies whose destination is then dead.
//!
//! A copy's destination is a stack register only (locals, globals,
//! constants and hoisted reads are `pinned`). What makes forwarding sound is
//! the deoptimization protocol: a stopped operation rebuilds the operand
//! stack from the registers its site names, so the consumer's site entries
//! are rewritten with its operands, and a copy is deleted only when a
//! liveness pass over the rewritten program -- counting every site entry as
//! a read at its operation -- proves nothing reads its destination.

use std::collections::BTreeSet;

use super::{Class, DeoptSite, TypedOp};

/// A register and its file: `true` for the boxed one.
type Register = (bool, u16);

/// Programs longer than this are not rewritten.
const MAX_OPS: usize = 512;

/// A set of registers, one bit each, scalar and boxed interleaved.
#[derive(Clone, PartialEq)]
struct Bits(Vec<u64>);

impl Bits {
    fn new(width: usize) -> Self {
        Self(vec![0; width.div_ceil(64)])
    }

    fn slot((boxed, index): Register) -> (usize, u64) {
        let bit = usize::from(index) * 2 + usize::from(boxed);
        (bit / 64, 1 << (bit % 64))
    }

    fn insert(&mut self, register: Register) {
        let (word, mask) = Self::slot(register);
        self.0[word] |= mask;
    }

    fn remove(&mut self, register: Register) {
        let (word, mask) = Self::slot(register);
        self.0[word] &= !mask;
    }

    fn contains(&self, register: Register) -> bool {
        let (word, mask) = Self::slot(register);
        self.0.get(word).is_some_and(|bits| bits & mask != 0)
    }

    fn clear(&mut self) {
        self.0.fill(0);
    }

    fn copy_from(&mut self, other: &Self) {
        self.0.copy_from_slice(&other.0);
    }

    fn union(&mut self, other: &Self) {
        for (bits, more) in self.0.iter_mut().zip(&other.0) {
            *bits |= more;
        }
    }
}

fn register(class: Class, index: u16) -> Register {
    (class == Class::Boxed, index)
}

/// Rewrites `ops` (and `sites`, parallel to it, and the `site_entries` they
/// name) to drop forwardable copies. Registers in `pinned` are never a
/// copy's destination here.
pub(super) fn forward_copies(
    ops: &mut Vec<TypedOp>,
    sites: &mut Vec<DeoptSite>,
    site_entries: &mut [(Class, u16)],
    pinned: &BTreeSet<Register>,
) {
    // Each round deletes one copy and recomputes liveness: programs are a
    // few dozen operations, compiled once per loop. A very large one is left
    // as built rather than paying for the rounds.
    if ops.len() > MAX_OPS {
        return;
    }
    // Diagnostic builds can compare a program with and without the pass.
    #[cfg(feature = "perf-counters")]
    if std::env::var_os("QJS_TL_NO_FORWARD").is_some() {
        return;
    }
    while let Some((index, rewrite)) = next_removable_copy(ops, sites, site_entries, pinned) {
        if let Some(rewrite) = rewrite {
            rewrite.apply(ops, site_entries);
        }
        remove(ops, sites, index);
    }
}

/// A consumer that reads `from` instead of `to`, and its site's entries.
struct Rewrite {
    consumer: usize,
    from: Register,
    to: u16,
    entries: std::ops::Range<usize>,
}

impl Rewrite {
    fn apply(&self, ops: &mut [TypedOp], site_entries: &mut [(Class, u16)]) {
        rewrite_uses(&mut ops[self.consumer], self.from, self.to);
        for entry in &mut site_entries[self.entries.clone()] {
            if register(entry.0, entry.1) == self.from {
                entry.1 = self.to;
            }
        }
    }
}

fn next_removable_copy(
    ops: &[TypedOp],
    sites: &[DeoptSite],
    site_entries: &[(Class, u16)],
    pinned: &BTreeSet<Register>,
) -> Option<(usize, Option<Rewrite>)> {
    let live_out = liveness(ops, sites, site_entries);
    let targets = jump_targets(ops);
    for (index, op) in ops.iter().enumerate() {
        let (destination, source) = match *op {
            TypedOp::Move { dst, src } => ((false, dst), src),
            TypedOp::MoveBoxed { dst, src } => ((true, dst), src),
            _ => continue,
        };
        if pinned.contains(&destination) || destination.1 == source {
            continue;
        }
        if !live_out[index].contains(destination) {
            return Some((index, None));
        }
        let Some((rewrite, group_end)) = forwarding(
            ops,
            sites,
            site_entries,
            &targets,
            index,
            destination,
            source,
        ) else {
            continue;
        };
        if dead_after_rewrite(ops, &rewrite, group_end, &live_out) {
            return Some((index, Some(rewrite)));
        }
    }
    None
}

/// Whether the copy's destination is dead once `rewrite` is applied,
/// judged from the liveness of the program before it. The rewrite changes
/// reads only inside the consumer's instruction, which control enters only
/// through the copy -- which writes the destination -- so liveness outside
/// that instruction is unchanged.
fn dead_after_rewrite(
    ops: &[TypedOp],
    rewrite: &Rewrite,
    group_end: usize,
    live_out: &[Bits],
) -> bool {
    let destination = rewrite.from;
    for at in rewrite.consumer..=group_end {
        let (uses, defs) = operands(&ops[at]);
        if at != rewrite.consumer && uses.contains(&destination) {
            return false;
        }
        if defs.contains(&destination) {
            return true;
        }
        // A branch out of the middle of the instruction reaches code whose
        // liveness the original program already describes; judged from it
        // (which still counts the unrewritten entries after the branch, so
        // it only ever refuses).
        if at != group_end
            && matches!(ops[at], TypedOp::Jump { .. } | TypedOp::JumpIfFalsy { .. })
            && live_out[at].contains(destination)
        {
            return false;
        }
    }
    !live_out[group_end].contains(destination)
}

/// The rewrite that lets the operation after the copy at `index` read
/// `source` itself, when control reaches it only from the copy and nothing
/// between the copy and each rewritten read changes `source`.
fn forwarding(
    ops: &[TypedOp],
    sites: &[DeoptSite],
    site_entries: &[(Class, u16)],
    targets: &BTreeSet<usize>,
    index: usize,
    destination: Register,
    source: u16,
) -> Option<(Rewrite, usize)> {
    let consumer = index + 1;
    if consumer >= ops.len() || targets.contains(&consumer) {
        return None;
    }
    let site = sites[consumer];
    if same_site(site, sites[index]) {
        return None;
    }
    // The consumer's instruction: the operations sharing its site, which
    // must be contiguous and reachable only through the consumer.
    let group_end = (consumer..ops.len())
        .take_while(|&at| same_site(sites[at], site))
        .last()?;
    let entries = site.start as usize..site.start as usize + usize::from(site.len);
    // Sites take consecutive entry ranges, so two share entries only when
    // they start at the same one and neither is empty.
    let entries_shared = sites.iter().enumerate().any(|(at, other)| {
        !(consumer..=group_end).contains(&at) && other.start == site.start && other.len > 0
    });
    if !entries.is_empty() && entries_shared {
        return None;
    }
    let source = (destination.0, source);
    for (at, op) in ops.iter().enumerate().take(group_end + 1).skip(consumer) {
        let (_, defs) = operands(op);
        if defs.contains(&source) {
            return None;
        }
        // A jump into the instruction's middle would reach its site entries
        // on a path where the destination need not hold the copy. A later
        // read of the destination itself keeps it live, which the caller's
        // liveness check then refuses.
        if at > consumer && targets.contains(&at) {
            return None;
        }
    }
    site_entries.get(entries.clone())?;
    Some((
        Rewrite {
            consumer,
            from: destination,
            to: source.1,
            entries,
        },
        group_end,
    ))
}

fn same_site(left: DeoptSite, right: DeoptSite) -> bool {
    left.ip == right.ip && left.start == right.start && left.len == right.len
}

fn jump_targets(ops: &[TypedOp]) -> BTreeSet<usize> {
    ops.iter()
        .filter_map(|op| match *op {
            TypedOp::Jump { target } | TypedOp::JumpIfFalsy { target, .. } => {
                usize::try_from(target).ok()
            }
            _ => None,
        })
        .collect()
}

/// The registers live after each operation. A site's entries are read by
/// the operation (a deoptimization or an exit materializes them), falling
/// off the end continues at the first operation, and leaving reads nothing
/// else: the frame's slots are written back from pinned registers only.
fn liveness(ops: &[TypedOp], sites: &[DeoptSite], site_entries: &[(Class, u16)]) -> Vec<Bits> {
    let count = ops.len();
    let reads: Vec<(Vec<Register>, Vec<Register>)> = ops
        .iter()
        .zip(sites)
        .map(|(op, site)| {
            let (mut uses, defs) = operands(op);
            let start = site.start as usize;
            let end = (start + usize::from(site.len)).min(site_entries.len());
            uses.extend(
                site_entries[start.min(end)..end]
                    .iter()
                    .map(|&(class, index)| register(class, index)),
            );
            (uses, defs)
        })
        .collect();
    let width = reads
        .iter()
        .flat_map(|(uses, defs)| uses.iter().chain(defs))
        .map(|&(_, index)| usize::from(index) * 2 + 2)
        .max()
        .unwrap_or(0);
    let successors = |at: usize| -> ([usize; 2], usize) {
        let next = if at + 1 == count { 0 } else { at + 1 };
        let wrap = |target: u32| target as usize % count;
        match ops[at] {
            TypedOp::Jump { target } => ([wrap(target), 0], 1),
            TypedOp::JumpIfFalsy { target, .. } => ([next, wrap(target)], 2),
            TypedOp::Leave { .. } => ([0, 0], 0),
            _ => ([next, 0], 1),
        }
    };
    let mut live_in = vec![Bits::new(width); count];
    let mut live_out = vec![Bits::new(width); count];
    let (mut out, mut input) = (Bits::new(width), Bits::new(width));
    let mut changed = true;
    while changed {
        changed = false;
        for at in (0..count).rev() {
            let (targets, targets_len) = successors(at);
            out.clear();
            for &successor in &targets[..targets_len] {
                out.union(&live_in[successor]);
            }
            let (uses, defs) = &reads[at];
            input.copy_from(&out);
            for &register in defs {
                input.remove(register);
            }
            for &register in uses {
                input.insert(register);
            }
            if input != live_in[at] {
                live_in[at].copy_from(&input);
                changed = true;
            }
            if out != live_out[at] {
                live_out[at].copy_from(&out);
            }
        }
    }
    live_out
}

/// Deletes the operation at `index`, retargeting jumps past it.
fn remove(ops: &mut Vec<TypedOp>, sites: &mut Vec<DeoptSite>, index: usize) {
    ops.remove(index);
    sites.remove(index);
    for op in ops.iter_mut() {
        if let TypedOp::Jump { target } | TypedOp::JumpIfFalsy { target, .. } = op
            && *target as usize > index
        {
            *target -= 1;
        }
    }
}

fn rewrite_uses(op: &mut TypedOp, from: Register, to: u16) {
    let visit = |boxed: bool, index: &mut u16| {
        if (boxed, *index) == from {
            *index = to;
        }
    };
    match op {
        TypedOp::Move { src, .. }
        | TypedOp::ToNumeric { src, .. }
        | TypedOp::Unary { src, .. }
        | TypedOp::Update { src, .. } => visit(false, src),
        TypedOp::Binary { left, right, .. } => {
            visit(false, left);
            visit(false, right);
        }
        TypedOp::DenseRead { index, .. } => visit(false, index),
        TypedOp::DenseWrite { index, value, .. } => {
            visit(false, index);
            visit(false, value);
        }
        TypedOp::DenseWriteBoxed { index, value, .. } => {
            visit(false, index);
            visit(true, value);
        }
        TypedOp::StoreSloppyGlobal { value, .. } => visit(false, value),
        TypedOp::JumpIfFalsy { cond, .. } | TypedOp::Exit { cond, .. } => visit(false, cond),
        TypedOp::MoveBoxed { src, .. }
        | TypedOp::Unbox { src, .. }
        | TypedOp::Truthy { src, .. } => visit(true, src),
        TypedOp::Box { src, .. } => visit(false, src),
        TypedOp::GetNamed { object, .. } | TypedOp::GetNamedTyped { object, .. } => {
            visit(true, object)
        }
        TypedOp::SetNamed { object, value, .. } => {
            visit(true, object);
            visit(true, value);
        }
        TypedOp::SetNamedTyped { object, value, .. } => {
            visit(true, object);
            visit(false, value);
        }
        TypedOp::ElementRead {
            receiver, index, ..
        } => {
            visit(true, receiver);
            visit(false, index);
        }
        TypedOp::ComputedRead { receiver, key, .. } => {
            visit(true, receiver);
            visit(true, key);
        }
        TypedOp::ComputedWrite {
            receiver,
            key,
            value,
        } => {
            visit(true, receiver);
            visit(true, key);
            visit(true, value);
        }
        TypedOp::CallNumericNative {
            callee,
            first,
            second,
            ..
        } => {
            visit(true, callee);
            visit(false, first);
            visit(false, second);
        }
        TypedOp::CallClosedFormLeaf {
            receiver,
            callee,
            args,
            arity,
            ..
        } => {
            let boxed_arguments = *arity & super::BOXED_ARGUMENTS != 0;
            visit(true, receiver);
            visit(true, callee);
            for argument in args {
                visit(boxed_arguments, argument);
            }
        }
        TypedOp::Guard { src, boxed, .. } => visit(*boxed, src),
        TypedOp::ArrayPush {
            receiver,
            callee,
            value,
            ..
        } => {
            visit(true, receiver);
            visit(true, callee);
            visit(true, value);
        }
        TypedOp::BoxedEquality { left, right, .. } => {
            visit(true, left);
            visit(true, right);
        }
        TypedOp::Jump { .. } | TypedOp::Leave { .. } => {}
    }
}

/// The registers `op` reads and writes. Exhaustive on purpose: a new
/// operation must say what it touches before this pass can move around it.
/// Unused argument slots of a call are counted as reads, which only ever
/// keeps a copy.
fn operands(op: &TypedOp) -> (Vec<Register>, Vec<Register>) {
    let scalar = |index: u16| (false, index);
    let boxed = |index: u16| (true, index);
    match *op {
        TypedOp::Move { dst, src }
        | TypedOp::ToNumeric { dst, src }
        | TypedOp::Unary { dst, src, .. }
        | TypedOp::Update { dst, src, .. } => (vec![scalar(src)], vec![scalar(dst)]),
        TypedOp::Binary {
            dst, left, right, ..
        } => (vec![scalar(left), scalar(right)], vec![scalar(dst)]),
        TypedOp::DenseRead { dst, index, .. } => (vec![scalar(index)], vec![scalar(dst)]),
        TypedOp::DenseWrite { index, value, .. } => (vec![scalar(index), scalar(value)], vec![]),
        TypedOp::DenseWriteBoxed { index, value, .. } => {
            (vec![scalar(index), boxed(value)], vec![])
        }
        TypedOp::StoreSloppyGlobal { value, .. } => (vec![scalar(value)], vec![]),
        TypedOp::JumpIfFalsy { cond, .. } | TypedOp::Exit { cond, .. } => {
            (vec![scalar(cond)], vec![])
        }
        TypedOp::Jump { .. } | TypedOp::Leave { .. } => (vec![], vec![]),
        TypedOp::MoveBoxed { dst, src } => (vec![boxed(src)], vec![boxed(dst)]),
        TypedOp::Unbox { dst, src } | TypedOp::Truthy { dst, src } => {
            (vec![boxed(src)], vec![scalar(dst)])
        }
        TypedOp::Box { dst, src } => (vec![scalar(src)], vec![boxed(dst)]),
        TypedOp::GetNamed { dst, object, .. } => (vec![boxed(object)], vec![boxed(dst)]),
        TypedOp::GetNamedTyped { dst, object, .. } => (vec![boxed(object)], vec![scalar(dst)]),
        TypedOp::SetNamed { object, value, .. } => (vec![boxed(object), boxed(value)], vec![]),
        TypedOp::SetNamedTyped { object, value, .. } => {
            (vec![boxed(object), scalar(value)], vec![])
        }
        TypedOp::ElementRead {
            dst,
            receiver,
            index,
        } => (vec![boxed(receiver), scalar(index)], vec![boxed(dst)]),
        TypedOp::ComputedRead { dst, receiver, key } => {
            (vec![boxed(receiver), boxed(key)], vec![boxed(dst)])
        }
        TypedOp::ComputedWrite {
            receiver,
            key,
            value,
        } => (vec![boxed(receiver), boxed(key), boxed(value)], vec![]),
        TypedOp::CallNumericNative {
            dst,
            callee,
            first,
            second,
            ..
        } => (
            vec![boxed(callee), scalar(first), scalar(second)],
            vec![scalar(dst)],
        ),
        TypedOp::CallClosedFormLeaf {
            dst,
            receiver,
            callee,
            args,
            arity,
        } => {
            let boxed_arguments = arity & super::BOXED_ARGUMENTS != 0;
            let mut uses = vec![boxed(receiver), boxed(callee)];
            uses.extend(args.iter().map(|&argument| (boxed_arguments, argument)));
            (uses, vec![boxed(dst)])
        }
        TypedOp::Guard {
            src,
            boxed: is_boxed,
            ..
        } => (vec![(is_boxed, src)], vec![]),
        TypedOp::ArrayPush {
            dst,
            receiver,
            callee,
            value,
        } => (
            vec![boxed(receiver), boxed(callee), boxed(value)],
            vec![scalar(dst)],
        ),
        TypedOp::BoxedEquality {
            dst, left, right, ..
        } => (vec![boxed(left), boxed(right)], vec![scalar(dst)]),
    }
}
