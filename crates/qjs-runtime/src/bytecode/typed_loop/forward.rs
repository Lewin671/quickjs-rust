//! Copy forwarding over a compiled program.
//!
//! The builder keeps every operand-stack entry in the register of its depth,
//! so `this[i].pos == obj.pos` copies `this` and the hoisted `obj.pos` into
//! stack registers only to read each once in the next operation -- a boxed
//! copy is a reference-count increment and, when the register is next
//! overwritten, a decrement. This pass lets the consumer read the source
//! register itself and deletes copies whose destination is then dead.
//!
//! The same holds the other way round -- `s = s + i` computes into a stack
//! register and copies it into the local, so the producing operation writes
//! the local itself -- and for the `ToNumeric` that every `i++` puts ahead of
//! its `Update`, which converts its operand again.
//!
//! A forwarded copy's destination is a stack register only (locals,
//! globals, constants and hoisted reads are `pinned`). What makes forwarding sound is
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
    while let Some((index, edit)) = next_removable_copy(ops, sites, site_entries, pinned) {
        match edit {
            Edit::Delete => {}
            Edit::Forward(rewrite) => rewrite.apply(ops, site_entries),
            Edit::Retarget { producer, to } => rewrite_def(&mut ops[producer], to),
        }
        remove(ops, sites, index);
    }
}

/// What deleting a copy takes besides the deletion.
enum Edit {
    /// Nothing: its destination is dead.
    Delete,
    /// Its reader reads the source.
    Forward(Rewrite),
    /// The operation producing its source writes the destination instead.
    Retarget { producer: usize, to: u16 },
}

/// A consumer that reads `to` instead of `from`, and the site entries of
/// every instruction from the copy to the consumer's.
struct Rewrite {
    consumer: usize,
    from: Register,
    to: u16,
    entries: Vec<std::ops::Range<usize>>,
}

impl Rewrite {
    fn apply(&self, ops: &mut [TypedOp], site_entries: &mut [(Class, u16)]) {
        rewrite_uses(&mut ops[self.consumer], self.from, self.to);
        for range in &self.entries {
            for entry in &mut site_entries[range.clone()] {
                if register(entry.0, entry.1) == self.from {
                    entry.1 = self.to;
                }
            }
        }
    }
}

fn next_removable_copy(
    ops: &[TypedOp],
    sites: &[DeoptSite],
    site_entries: &[(Class, u16)],
    pinned: &BTreeSet<Register>,
) -> Option<(usize, Edit)> {
    let live_out = liveness(ops, sites, site_entries);
    let targets = jump_targets(ops);
    for (index, op) in ops.iter().enumerate() {
        let (destination, source, numeric) = match *op {
            TypedOp::Move { dst, src } => ((false, dst), src, false),
            TypedOp::MoveBoxed { dst, src } => ((true, dst), src, false),
            // `to_numeric` of a register value has no effect but its
            // result, and an `Update` applies it again itself: the
            // `ToNumeric; Update` pair every `i++` compiles to needs only
            // the second.
            TypedOp::ToNumeric { dst, src } => ((false, dst), src, true),
            _ => continue,
        };
        if destination.1 == source {
            continue;
        }
        if !numeric
            && let Some(producer) = retarget(
                ops,
                sites,
                &targets,
                &live_out,
                pinned,
                index,
                destination,
                source,
            )
        {
            return Some((
                index,
                Edit::Retarget {
                    producer,
                    to: destination.1,
                },
            ));
        }
        if pinned.contains(&destination) {
            continue;
        }
        if !live_out[index].contains(destination) {
            return Some((index, Edit::Delete));
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
        // A converted value differs from its source, so only a consumer
        // that converts it again, and no site entry, may read the source.
        if numeric
            && (!matches!(ops[rewrite.consumer], TypedOp::Update { src, .. } if src == destination.1)
                || (ops[index + 1..=group_end].iter().any(may_stop)
                    && rewrite.entries.iter().any(|range| {
                        site_entries[range.clone()]
                            .iter()
                            .any(|&(class, index)| register(class, index) == destination)
                    })))
        {
            continue;
        }
        if dead_after_rewrite(ops, &rewrite, group_end, &live_out) {
            return Some((index, Edit::Forward(rewrite)));
        }
    }
    None
}

/// The operation right before the copy at `index` whose result is the
/// copy's source, when it can write the copy's destination itself: the
/// source is a stack register nothing reads after the copy, the copy is its
/// instruction's only operation (`x = expr` in a loop: the store), and
/// control reaches the copy only from the producer. The producer writes its
/// destination only once it has succeeded, so a deoptimizing producer still
/// leaves the old value to be written back.
#[allow(clippy::too_many_arguments)]
fn retarget(
    ops: &[TypedOp],
    sites: &[DeoptSite],
    targets: &BTreeSet<usize>,
    live_out: &[Bits],
    pinned: &BTreeSet<Register>,
    index: usize,
    destination: Register,
    source: u16,
) -> Option<usize> {
    let source = (destination.0, source);
    let producer = index.checked_sub(1)?;
    if pinned.contains(&source)
        || targets.contains(&index)
        || live_out[index].contains(source)
        || same_site(sites[producer], sites[index])
        || sites
            .iter()
            .enumerate()
            .any(|(at, site)| at != index && same_site(*site, sites[index]))
    {
        return None;
    }
    let (_, defs) = operands(&ops[producer]);
    (defs == [source] && writes_last(&ops[producer])).then_some(producer)
}

/// Whether `op` writes its destination only after everything else it does
/// has succeeded -- true of every operation with a destination except the
/// copies themselves, whose retargeting would be pointless.
fn writes_last(op: &TypedOp) -> bool {
    !matches!(op, TypedOp::Move { .. } | TypedOp::MoveBoxed { .. })
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

/// The rewrite that lets the first operation after the copy at `index`
/// that reads its destination read `source` instead, when control runs
/// straight from the copy to that operation's instruction and nothing on
/// the way changes `source` or the destination. The instructions passed on
/// the way hold the copy on their operand stack as well, so their site
/// entries are rewritten with the consumer's.
fn forwarding(
    ops: &[TypedOp],
    sites: &[DeoptSite],
    site_entries: &[(Class, u16)],
    targets: &BTreeSet<usize>,
    index: usize,
    destination: Register,
    source: u16,
) -> Option<(Rewrite, usize)> {
    let source = (destination.0, source);
    let mut consumer = index + 1;
    loop {
        let op = ops.get(consumer)?;
        if targets.contains(&consumer) {
            return None;
        }
        let (uses, defs) = operands(op);
        if uses.contains(&destination) {
            break;
        }
        if defs.contains(&destination)
            || defs.contains(&source)
            || matches!(
                op,
                TypedOp::Jump { .. }
                    | TypedOp::JumpIfFalsy { .. }
                    | TypedOp::Exit { .. }
                    | TypedOp::Leave { .. }
            )
        {
            return None;
        }
        consumer += 1;
    }
    // The consumer's instruction: the operations sharing its site, which
    // must be contiguous and reachable only through the consumer.
    let group_end = (consumer..ops.len())
        .take_while(|&at| same_site(sites[at], sites[consumer]))
        .last()?;
    for (at, op) in ops.iter().enumerate().take(group_end + 1).skip(consumer) {
        let (_, defs) = operands(op);
        // The consumer reads before it writes; an operation after it that
        // stopped would materialize the rewritten entries after the write.
        if defs.contains(&source) && (at != consumer || group_end != consumer) {
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
    // Every site from the copy to the consumer's instruction, each of whose
    // entries must belong to operations in that stretch alone. Sites take
    // consecutive entry ranges, so two share entries only when they start at
    // the same one and neither is empty.
    let window = index + 1..=group_end;
    let mut entries: Vec<std::ops::Range<usize>> = Vec::new();
    for at in window.clone() {
        let site = sites[at];
        if site.len == 0 {
            continue;
        }
        let range = site.start as usize..site.start as usize + usize::from(site.len);
        let site_registers = site_entries.get(range.clone())?;
        // The copy's own instruction started before the copy: its entries
        // describe the stack without it, so they are left alone -- which
        // is only right if they do not name the destination at all.
        if same_site(site, sites[index]) {
            if site_registers
                .iter()
                .any(|&(class, register_index)| register(class, register_index) == destination)
            {
                return None;
            }
            continue;
        }
        if entries.contains(&range) {
            continue;
        }
        let shared = sites.iter().enumerate().any(|(other, candidate)| {
            !window.contains(&other) && candidate.start == site.start && candidate.len > 0
        });
        if shared {
            return None;
        }
        entries.push(range);
    }
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
/// an operation that can stop there (a deoptimization or an exit
/// materializes them) and by a backward edge at its target's site (the
/// residency bound), falling off the end continues at the first operation,
/// and leaving reads nothing else: the frame's slots are written back from
/// pinned registers only.
fn liveness(ops: &[TypedOp], sites: &[DeoptSite], site_entries: &[(Class, u16)]) -> Vec<Bits> {
    let count = ops.len();
    let entries_of = |site: DeoptSite| {
        let start = site.start as usize;
        let end = (start + usize::from(site.len)).min(site_entries.len());
        site_entries[start.min(end)..end]
            .iter()
            .map(|&(class, index)| register(class, index))
    };
    let reads: Vec<(Vec<Register>, Vec<Register>)> = ops
        .iter()
        .enumerate()
        .map(|(at, op)| {
            let (mut uses, defs) = operands(op);
            if may_stop(op) {
                uses.extend(entries_of(sites[at]));
            }
            // A backward jump, and falling off the end, count toward the
            // residency bound and stop at the target's site.
            match *op {
                TypedOp::Jump { target } if (target as usize) <= at => {
                    uses.extend(entries_of(sites[target as usize % count]));
                }
                _ if at + 1 == count
                    && !matches!(op, TypedOp::Jump { .. } | TypedOp::Leave { .. }) =>
                {
                    uses.extend(entries_of(sites[0]));
                }
                _ => {}
            }
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

/// Whether `op` can deoptimize or leave the program, materializing its
/// site's operand stack. A copy, a conversion of a register value and a
/// branch cannot; nor can `Update`, whose operand converts to a number
/// whatever it holds.
fn may_stop(op: &TypedOp) -> bool {
    !matches!(
        op,
        TypedOp::Move { .. }
            | TypedOp::MoveBoxed { .. }
            | TypedOp::ToNumeric { .. }
            | TypedOp::Update { .. }
            | TypedOp::Jump { .. }
            | TypedOp::JumpIfFalsy { .. }
    )
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

/// Points the destination of `op` (which has exactly one) at `to`.
fn rewrite_def(op: &mut TypedOp, to: u16) {
    match op {
        TypedOp::Move { dst, .. }
        | TypedOp::ToNumeric { dst, .. }
        | TypedOp::Binary { dst, .. }
        | TypedOp::Unary { dst, .. }
        | TypedOp::Update { dst, .. }
        | TypedOp::DenseRead { dst, .. }
        | TypedOp::MoveBoxed { dst, .. }
        | TypedOp::Unbox { dst, .. }
        | TypedOp::Truthy { dst, .. }
        | TypedOp::Box { dst, .. }
        | TypedOp::GetNamed { dst, .. }
        | TypedOp::GetNamedTyped { dst, .. }
        | TypedOp::ElementRead { dst, .. }
        | TypedOp::ComputedRead { dst, .. }
        | TypedOp::CallNumericNative { dst, .. }
        | TypedOp::CallClosedFormLeaf { dst, .. }
        | TypedOp::ArrayPush { dst, .. }
        | TypedOp::BoxedEquality { dst, .. } => *dst = to,
        TypedOp::DenseWrite { .. }
        | TypedOp::DenseWriteBoxed { .. }
        | TypedOp::StoreSloppyGlobal { .. }
        | TypedOp::JumpIfFalsy { .. }
        | TypedOp::Jump { .. }
        | TypedOp::SetNamed { .. }
        | TypedOp::SetNamedTyped { .. }
        | TypedOp::ComputedWrite { .. }
        | TypedOp::Guard { .. }
        | TypedOp::Exit { .. }
        | TypedOp::Leave { .. } => {}
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
            arity,
        } => {
            let mut uses = vec![boxed(callee)];
            uses.extend(
                [first, second]
                    .iter()
                    .take(usize::from(arity))
                    .map(|&arg| scalar(arg)),
            );
            (uses, vec![scalar(dst)])
        }
        TypedOp::CallClosedFormLeaf {
            dst,
            receiver,
            callee,
            args,
            arity,
        } => {
            let boxed_arguments = arity & super::BOXED_ARGUMENTS != 0;
            let count = usize::from(arity & !super::BOXED_ARGUMENTS);
            let mut uses = vec![boxed(receiver), boxed(callee)];
            uses.extend(
                args.iter()
                    .take(count)
                    .map(|&argument| (boxed_arguments, argument)),
            );
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
