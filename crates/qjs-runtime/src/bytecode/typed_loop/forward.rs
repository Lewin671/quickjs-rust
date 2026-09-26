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
//! globals, constants and hoisted reads are `pinned`). What makes forwarding
//! sound is the deoptimization protocol: a stopped operation rebuilds the
//! operand stack from the registers its site names, so the site entries
//! between a copy and its reader are rewritten with the reader, and a copy
//! is deleted only when liveness -- counting a site's entries as read where
//! an operation can stop -- proves nothing reads its destination.
//!
//! The pass runs at compile time inside the measured run, so it is linear
//! per round: one analysis, then every edit whose operations no other edit
//! touches. Edits only shorten live ranges, except that a forwarded source
//! lives on through its window, which no other edit's window can overlap.

use std::collections::{BTreeMap, BTreeSet};

use super::{Class, DeoptSite, TypedOp};

/// A register and its file: `true` for the boxed one.
type Register = (bool, u16);

/// Programs longer than this are not rewritten.
const MAX_OPS: usize = 512;

/// Rounds of edits; each round also enables the next (a forwarded copy
/// leaves a dead one behind), and a handful reach the fixed point.
const MAX_ROUNDS: usize = 16;

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

/// The registers an operation reads and the one it writes, if any.
#[derive(Clone, Copy)]
struct Operands {
    uses: [Register; 6],
    len: usize,
    def: Option<Register>,
}

impl Operands {
    fn new(uses: &[Register], def: Option<Register>) -> Self {
        let mut all = [(false, 0); 6];
        all[..uses.len()].copy_from_slice(uses);
        Self {
            uses: all,
            len: uses.len(),
            def,
        }
    }

    fn uses(&self) -> &[Register] {
        &self.uses[..self.len]
    }

    fn reads(&self, register: Register) -> bool {
        self.uses().contains(&register)
    }

    fn writes(&self, register: Register) -> bool {
        self.def == Some(register)
    }
}

/// Rewrites `ops` (and `sites`, parallel to it, and the `site_entries` they
/// name) to drop forwardable copies. Registers in `pinned` are never a
/// forwarded copy's destination.
pub(super) fn forward_copies(
    ops: &mut Vec<TypedOp>,
    sites: &mut Vec<DeoptSite>,
    site_entries: &mut [(Class, u16)],
    pinned: &BTreeSet<Register>,
) {
    if ops.len() > MAX_OPS {
        return;
    }
    // Diagnostic builds can compare a program with and without the pass.
    #[cfg(feature = "perf-counters")]
    if std::env::var_os("QJS_TL_NO_FORWARD").is_some() {
        return;
    }
    for _ in 0..MAX_ROUNDS {
        let analysis = Analysis::new(ops, sites, site_entries);
        let edits = collect_edits(ops, sites, site_entries, pinned, &analysis);
        if edits.is_empty() {
            break;
        }
        // Last first: an edit only moves the operations after it.
        for (_, index, edit) in edits.into_iter().rev() {
            match edit {
                Edit::Delete => {}
                Edit::Forward(rewrite) => rewrite.apply(ops, site_entries),
                Edit::Retarget { producer, to } => rewrite_def(&mut ops[producer], to),
            }
            remove(ops, sites, index);
        }
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

/// What one round knows about the program before editing it.
struct Analysis {
    operands: Vec<Operands>,
    live_out: Vec<Bits>,
    targets: Vec<bool>,
    /// First and last operation of each site.
    site_span: BTreeMap<(u32, u32, u8), (usize, usize)>,
    /// First and last operation whose non-empty site starts at an entry.
    start_span: BTreeMap<u32, (usize, usize)>,
}

impl Analysis {
    fn new(ops: &[TypedOp], sites: &[DeoptSite], site_entries: &[(Class, u16)]) -> Self {
        let operands: Vec<Operands> = ops.iter().map(operands).collect();
        let mut targets = vec![false; ops.len() + 1];
        for op in ops {
            if let TypedOp::Jump { target } | TypedOp::JumpIfFalsy { target, .. } = *op
                && let Some(slot) = targets.get_mut(target as usize)
            {
                *slot = true;
            }
        }
        let mut site_span = BTreeMap::new();
        let mut start_span = BTreeMap::new();
        for (at, site) in sites.iter().enumerate() {
            widen(site_span.entry(site_key(*site)).or_insert((at, at)), at);
            if site.len > 0 {
                widen(start_span.entry(site.start).or_insert((at, at)), at);
            }
        }
        let live_out = liveness(ops, sites, site_entries, &operands);
        Self {
            operands,
            live_out,
            targets,
            site_span,
            start_span,
        }
    }

    fn is_target(&self, at: usize) -> bool {
        self.targets.get(at).copied().unwrap_or(false)
    }

    /// Whether the operations sharing `at`'s site are `at` alone.
    fn site_is_alone(&self, sites: &[DeoptSite], at: usize) -> bool {
        self.site_span.get(&site_key(sites[at])) == Some(&(at, at))
    }

    /// Whether every operation naming `site`'s entries lies in `window`.
    fn entries_within(&self, site: DeoptSite, window: &std::ops::RangeInclusive<usize>) -> bool {
        self.start_span
            .get(&site.start)
            .is_none_or(|(first, last)| window.contains(first) && window.contains(last))
    }
}

fn widen(span: &mut (usize, usize), at: usize) {
    span.0 = span.0.min(at);
    span.1 = span.1.max(at);
}

fn site_key(site: DeoptSite) -> (u32, u32, u8) {
    (site.ip, site.start, site.len)
}

fn same_site(left: DeoptSite, right: DeoptSite) -> bool {
    site_key(left) == site_key(right)
}

/// Every copy this round removes, with its edit: in program order, no two
/// touching the same operation.
fn collect_edits(
    ops: &[TypedOp],
    sites: &[DeoptSite],
    site_entries: &[(Class, u16)],
    pinned: &BTreeSet<Register>,
    analysis: &Analysis,
) -> Vec<(usize, usize, Edit)> {
    let mut edits: Vec<(usize, usize, Edit)> = Vec::new();
    let mut claimed_to = None;
    for (index, op) in ops.iter().enumerate() {
        let Some((first, edit, last)) =
            copy_edit(ops, sites, site_entries, pinned, analysis, index, op)
        else {
            continue;
        };
        if claimed_to.is_some_and(|end| first <= end) {
            continue;
        }
        claimed_to = Some(last);
        edits.push((first, index, edit));
    }
    edits
}

/// The edit removing the copy at `index`, if there is one, with the first
/// and last operation it touches.
fn copy_edit(
    ops: &[TypedOp],
    sites: &[DeoptSite],
    site_entries: &[(Class, u16)],
    pinned: &BTreeSet<Register>,
    analysis: &Analysis,
    index: usize,
    op: &TypedOp,
) -> Option<(usize, Edit, usize)> {
    let (destination, source, numeric) = match *op {
        TypedOp::Move { dst, src } => ((false, dst), src, false),
        TypedOp::MoveBoxed { dst, src } => ((true, dst), src, false),
        // `to_numeric` of a register value has no effect but its result,
        // and an `Update` applies it again itself: the `ToNumeric; Update`
        // pair every `i++` compiles to needs only the second.
        TypedOp::ToNumeric { dst, src } => ((false, dst), src, true),
        _ => return None,
    };
    if destination.1 == source {
        return None;
    }
    if !numeric
        && let Some(producer) = retarget(ops, sites, pinned, analysis, index, destination, source)
    {
        let edit = Edit::Retarget {
            producer,
            to: destination.1,
        };
        return Some((producer, edit, index));
    }
    if pinned.contains(&destination) {
        return None;
    }
    if !analysis.live_out[index].contains(destination) {
        return Some((index, Edit::Delete, index));
    }
    let (rewrite, group_end) = forwarding(
        ops,
        sites,
        site_entries,
        analysis,
        index,
        destination,
        source,
    )?;
    // A converted value differs from its source, so only a consumer that
    // converts it again, and no site entry that can be materialized, may
    // read the source.
    if numeric
        && (!matches!(ops[rewrite.consumer], TypedOp::Update { src, .. } if src == destination.1)
            || (ops[index + 1..=group_end].iter().any(may_stop)
                && rewrite.entries.iter().any(|range| {
                    site_entries[range.clone()]
                        .iter()
                        .any(|&(class, index)| register(class, index) == destination)
                })))
    {
        return None;
    }
    dead_after_rewrite(ops, analysis, &rewrite, group_end).then_some((
        index,
        Edit::Forward(rewrite),
        group_end,
    ))
}

/// The operation right before the copy at `index` whose result is the
/// copy's source, when it can write the copy's destination itself: the
/// source is a stack register nothing reads after the copy, the copy is its
/// instruction's only operation (`x = expr` in a loop: the store), and
/// control reaches the copy only from the producer. The producer writes its
/// destination only once it has succeeded, so a deoptimizing producer still
/// leaves the old value to be written back.
fn retarget(
    ops: &[TypedOp],
    sites: &[DeoptSite],
    pinned: &BTreeSet<Register>,
    analysis: &Analysis,
    index: usize,
    destination: Register,
    source: u16,
) -> Option<usize> {
    let source = (destination.0, source);
    let producer = index.checked_sub(1)?;
    if pinned.contains(&source)
        || analysis.is_target(index)
        || analysis.live_out[index].contains(source)
        || same_site(sites[producer], sites[index])
        || !analysis.site_is_alone(sites, index)
    {
        return None;
    }
    (analysis.operands[producer].def == Some(source) && writes_last(&ops[producer]))
        .then_some(producer)
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
    analysis: &Analysis,
    rewrite: &Rewrite,
    group_end: usize,
) -> bool {
    let destination = rewrite.from;
    for (at, op) in ops
        .iter()
        .enumerate()
        .take(group_end + 1)
        .skip(rewrite.consumer)
    {
        let operands = &analysis.operands[at];
        if at != rewrite.consumer && operands.reads(destination) {
            return false;
        }
        if operands.writes(destination) {
            return true;
        }
        // A branch out of the middle of the instruction reaches code whose
        // liveness the original program already describes; judged from it
        // (which still counts the unrewritten entries after the branch, so
        // it only ever refuses).
        if at != group_end
            && matches!(op, TypedOp::Jump { .. } | TypedOp::JumpIfFalsy { .. })
            && analysis.live_out[at].contains(destination)
        {
            return false;
        }
    }
    !analysis.live_out[group_end].contains(destination)
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
    analysis: &Analysis,
    index: usize,
    destination: Register,
    source: u16,
) -> Option<(Rewrite, usize)> {
    let source = (destination.0, source);
    let count = analysis.operands.len();
    let mut consumer = index + 1;
    loop {
        if consumer >= count || analysis.is_target(consumer) {
            return None;
        }
        let operands = &analysis.operands[consumer];
        if operands.reads(destination) {
            break;
        }
        if operands.writes(destination)
            || operands.writes(source)
            || matches!(
                ops[consumer],
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
    let group_end = (consumer..count)
        .take_while(|&at| same_site(sites[at], sites[consumer]))
        .last()?;
    for at in consumer..=group_end {
        // The consumer reads before it writes; an operation after it that
        // stopped would materialize the rewritten entries after the write.
        if analysis.operands[at].writes(source) && (at != consumer || group_end != consumer) {
            return None;
        }
        // A jump into the instruction's middle would reach its site entries
        // on a path where the destination need not hold the copy. A later
        // read of the destination itself keeps it live, which the caller's
        // liveness check then refuses.
        if at > consumer && analysis.is_target(at) {
            return None;
        }
    }
    // Every site from the copy to the consumer's instruction, each of whose
    // entries must belong to operations in that stretch alone.
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
        if !analysis.entries_within(site, &window) {
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

/// The registers live after each operation. A site's entries are read by
/// an operation that can stop there (a deoptimization or an exit
/// materializes them) and by a backward edge at its target's site (the
/// residency bound), falling off the end continues at the first operation,
/// and leaving reads nothing else: the frame's slots are written back from
/// pinned registers only.
fn liveness(
    ops: &[TypedOp],
    sites: &[DeoptSite],
    site_entries: &[(Class, u16)],
    operands: &[Operands],
) -> Vec<Bits> {
    let count = ops.len();
    let entries_of = |site: DeoptSite| {
        let start = site.start as usize;
        let end = (start + usize::from(site.len)).min(site_entries.len());
        &site_entries[start.min(end)..end]
    };
    // Extra reads per operation: the site entries it can materialize.
    let site_reads = |at: usize| -> [&[(Class, u16)]; 2] {
        let op = &ops[at];
        let own: &[(Class, u16)] = if may_stop(op) {
            entries_of(sites[at])
        } else {
            &[]
        };
        // A backward jump, and falling off the end, count toward the
        // residency bound and stop at the target's site.
        let edge: &[(Class, u16)] = match *op {
            TypedOp::Jump { target } if (target as usize) <= at => {
                entries_of(sites[target as usize % count])
            }
            _ if at + 1 == count && !matches!(op, TypedOp::Jump { .. } | TypedOp::Leave { .. }) => {
                entries_of(sites[0])
            }
            _ => &[],
        };
        [own, edge]
    };
    let width = operands
        .iter()
        .flat_map(|operands| operands.uses().iter().chain(&operands.def))
        .map(|&(_, index)| usize::from(index))
        .chain(site_entries.iter().map(|&(_, index)| usize::from(index)))
        .max()
        .map_or(0, |index| index * 2 + 2);
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
            input.copy_from(&out);
            if let Some(def) = operands[at].def {
                input.remove(def);
            }
            for &register in operands[at].uses() {
                input.insert(register);
            }
            for entries in site_reads(at) {
                for &(class, index) in entries {
                    input.insert(register(class, index));
                }
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
fn operands(op: &TypedOp) -> Operands {
    let scalar = |index: u16| (false, index);
    let boxed = |index: u16| (true, index);
    match *op {
        TypedOp::Move { dst, src }
        | TypedOp::ToNumeric { dst, src }
        | TypedOp::Unary { dst, src, .. }
        | TypedOp::Update { dst, src, .. } => Operands::new(&[scalar(src)], Some(scalar(dst))),
        TypedOp::Binary {
            dst, left, right, ..
        } => Operands::new(&[scalar(left), scalar(right)], Some(scalar(dst))),
        TypedOp::DenseRead { dst, index, .. } => Operands::new(&[scalar(index)], Some(scalar(dst))),
        TypedOp::DenseWrite { index, value, .. } => {
            Operands::new(&[scalar(index), scalar(value)], None)
        }
        TypedOp::DenseWriteBoxed { index, value, .. } => {
            Operands::new(&[scalar(index), boxed(value)], None)
        }
        TypedOp::StoreSloppyGlobal { value, .. } => Operands::new(&[scalar(value)], None),
        TypedOp::JumpIfFalsy { cond, .. } | TypedOp::Exit { cond, .. } => {
            Operands::new(&[scalar(cond)], None)
        }
        TypedOp::Jump { .. } | TypedOp::Leave { .. } => Operands::new(&[], None),
        TypedOp::MoveBoxed { dst, src } => Operands::new(&[boxed(src)], Some(boxed(dst))),
        TypedOp::Unbox { dst, src } | TypedOp::Truthy { dst, src } => {
            Operands::new(&[boxed(src)], Some(scalar(dst)))
        }
        TypedOp::Box { dst, src } => Operands::new(&[scalar(src)], Some(boxed(dst))),
        TypedOp::GetNamed { dst, object, .. } => Operands::new(&[boxed(object)], Some(boxed(dst))),
        TypedOp::GetNamedTyped { dst, object, .. } => {
            Operands::new(&[boxed(object)], Some(scalar(dst)))
        }
        TypedOp::SetNamed { object, value, .. } => {
            Operands::new(&[boxed(object), boxed(value)], None)
        }
        TypedOp::SetNamedTyped { object, value, .. } => {
            Operands::new(&[boxed(object), scalar(value)], None)
        }
        TypedOp::ElementRead {
            dst,
            receiver,
            index,
        } => Operands::new(&[boxed(receiver), scalar(index)], Some(boxed(dst))),
        TypedOp::ComputedRead { dst, receiver, key } => {
            Operands::new(&[boxed(receiver), boxed(key)], Some(boxed(dst)))
        }
        TypedOp::ComputedWrite {
            receiver,
            key,
            value,
        } => Operands::new(&[boxed(receiver), boxed(key), boxed(value)], None),
        TypedOp::CallNumericNative {
            dst,
            callee,
            first,
            second,
            arity,
        } => {
            let arguments = [boxed(callee), scalar(first), scalar(second)];
            Operands::new(
                &arguments[..1 + usize::from(arity).min(2)],
                Some(scalar(dst)),
            )
        }
        TypedOp::CallClosedFormLeaf {
            dst,
            receiver,
            callee,
            args,
            arity,
        } => {
            let boxed_arguments = arity & super::BOXED_ARGUMENTS != 0;
            let count = usize::from(arity & !super::BOXED_ARGUMENTS).min(args.len());
            let mut uses = [(false, 0); 6];
            uses[0] = boxed(receiver);
            uses[1] = boxed(callee);
            for (slot, &argument) in uses[2..].iter_mut().zip(&args[..count]) {
                *slot = (boxed_arguments, argument);
            }
            Operands::new(&uses[..2 + count], Some(boxed(dst)))
        }
        TypedOp::Guard {
            src,
            boxed: is_boxed,
            ..
        } => Operands::new(&[(is_boxed, src)], None),
        TypedOp::ArrayPush {
            dst,
            receiver,
            callee,
            value,
        } => Operands::new(
            &[boxed(receiver), boxed(callee), boxed(value)],
            Some(scalar(dst)),
        ),
        TypedOp::BoxedEquality {
            dst, left, right, ..
        } => Operands::new(&[boxed(left), boxed(right)], Some(scalar(dst))),
    }
}
