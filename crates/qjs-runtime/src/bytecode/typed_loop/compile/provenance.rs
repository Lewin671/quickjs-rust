//! Bounded identity origins for representation inference only.
//!
//! A union keeps the sources of an identity value after a control-flow join.
//! It proves no receiver, callee or type identity. Computed values are opaque:
//! demanding a boxed arithmetic result never demands its operands. Canonical
//! union handles are stable within one compiler pass; no arena survives it.

use super::{Builder, Class, TypedOp};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Origin {
    Computed,
    Local(u32),
    ElementRead(u32),
    /// Exact named-read site for the existing scalar-read and callee proofs.
    NamedRead(u32),
    Identity(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    Local(u32),
    ElementRead(u32),
    Opaque,
}

// These limits bound compilation even for adversarial branch fan-in. Hitting
// either rejects the region, leaving its ordinary bytecode unchanged.
const MAX_SOURCES: usize = 64;
const MAX_UNIONS: usize = super::super::MAX_REGION_OPS;

#[derive(Default)]
pub(super) struct IdentityOrigins(Vec<Vec<Source>>);

impl IdentityOrigins {
    fn sources(&self, origin: Origin) -> Option<Vec<Source>> {
        Some(match origin {
            Origin::Local(slot) => vec![Source::Local(slot)],
            Origin::ElementRead(site) => vec![Source::ElementRead(site)],
            Origin::Computed | Origin::NamedRead(_) => vec![Source::Opaque],
            Origin::Identity(id) => self.0.get(usize::from(id))?.clone(),
        })
    }

    fn merge(&mut self, left: Origin, right: Origin) -> Option<Origin> {
        if left == right {
            return Some(left);
        }
        let mut sources = self.sources(left)?;
        sources.extend(self.sources(right)?);
        sources.sort_unstable();
        sources.dedup();
        if sources.len() > MAX_SOURCES {
            return None;
        }
        if sources == [Source::Opaque] {
            return Some(Origin::Computed);
        }
        if let Some(id) = self.0.iter().position(|existing| *existing == sources) {
            return Some(Origin::Identity(u16::try_from(id).ok()?));
        }
        if self.0.len() >= MAX_UNIONS {
            return None;
        }
        let id = u16::try_from(self.0.len()).ok()?;
        self.0.push(sources);
        Some(Origin::Identity(id))
    }
}

impl Builder<'_> {
    /// Records identity sources that must change representation next pass.
    /// The caller abandons this pass: predecessor code may already be emitted.
    pub(super) fn discover_identity_demand(&mut self, origin: Origin) -> Option<bool> {
        let mut identity = false;
        for source in self.identity_origins.sources(origin)? {
            match source {
                Source::Local(slot) => self.discovered_boxed.push(slot),
                Source::ElementRead(site) => self.discovered_boxed_element_reads.push(site),
                Source::Opaque => continue,
            }
            identity = true;
        }
        Some(identity)
    }

    pub(super) fn pop_boxed(&mut self) -> Option<(u16, Origin)> {
        let (register, class, origin) = self.stack.pop()?;
        match class {
            Class::Boxed => Some((register, origin)),
            Class::Scalar => {
                if self.discover_identity_demand(origin)? {
                    return None;
                }
                let dst = self.fresh_boxed()?;
                self.emit(TypedOp::Box { dst, src: register });
                Some((dst, origin))
            }
        }
    }

    pub(super) fn merge_identity_states(
        &mut self,
        left: &[(u16, Class, Origin)],
        right: &[(u16, Class, Origin)],
    ) -> Option<Vec<(u16, Class, Origin)>> {
        if left.len() != right.len() {
            return None;
        }
        left.iter()
            .zip(right)
            .map(|(left, right)| {
                if left.0 != right.0 {
                    return None;
                }
                if left.1 != right.1 {
                    let scalar = if left.1 == Class::Scalar {
                        left.2
                    } else {
                        right.2
                    };
                    self.discover_identity_demand(scalar)?;
                    return None;
                }
                Some((
                    left.0,
                    left.1,
                    self.identity_origins.merge(left.2, right.2)?,
                ))
            })
            .collect()
    }

    /// Backward edges cannot weaken origins already consumed by emitted code.
    /// A source-union change there is conservatively rejected, not revisited.
    pub(super) fn record_target(&mut self, target: usize, ip: usize) -> Option<()> {
        let offset = target.checked_sub(self.header)?;
        let state = self.stack.clone();
        let existing = self.states.get(offset)?.clone();
        if target <= ip {
            let existing = existing?;
            return (self.merge_identity_states(&existing, &state)? == existing).then_some(());
        }
        self.states[offset] = Some(match existing {
            Some(existing) => self.merge_identity_states(&existing, &state)?,
            None => state,
        });
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unions_are_canonical_bounded_and_keep_computed_opaque() {
        let mut arena = IdentityOrigins::default();
        let a = Origin::Local(1);
        let b = Origin::ElementRead(9);
        let union = arena.merge(a, b).unwrap();
        assert_eq!(arena.merge(b, a), Some(union));
        assert_eq!(arena.merge(union, a), Some(union));
        let mixed = arena.merge(union, Origin::Computed).unwrap();
        assert_eq!(
            arena.sources(mixed).unwrap(),
            [Source::Local(1), Source::ElementRead(9), Source::Opaque]
        );
        assert_eq!(
            arena.merge(Origin::NamedRead(1), Origin::NamedRead(2)),
            Some(Origin::Computed)
        );
        assert!(arena.sources(Origin::Identity(u16::MAX)).is_none());
        let mut many = a;
        for slot in 2..=MAX_SOURCES as u32 {
            many = arena.merge(many, Origin::Local(slot)).unwrap();
        }
        assert!(arena.merge(many, Origin::Local(1000)).is_none());
        let mut full = IdentityOrigins::default();
        for slot in 0..MAX_UNIONS as u32 {
            full.merge(Origin::Local(slot), Origin::Computed).unwrap();
        }
        assert!(
            full.merge(Origin::Local(u32::MAX), Origin::Computed)
                .is_none()
        );
    }

    #[test]
    fn mixed_copy_classes_make_monotone_progress_in_both_directions() {
        let code = super::super::Bytecode::new(vec![], vec![], vec![super::super::Op::Pop]);
        for (boxed, discovered) in [(1, 0), (0, 1)] {
            let mut builder = Builder::new(&code, 0, 0, vec![boxed], vec![], vec![]);
            assert!(
                builder
                    .compile_op(
                        &super::super::Op::CopyLocal {
                            from: 0,
                            to: 1,
                            skip: 0
                        },
                        0
                    )
                    .is_none()
            );
            assert_eq!(builder.discovered_boxed, [discovered]);
        }
    }

    #[test]
    fn backward_states_accept_identical_unions_and_reject_new_sources() {
        let code = super::super::Bytecode::new(vec![], vec![], vec![super::super::Op::Pop]);
        let mut builder = Builder::new(&code, 0, 0, vec![], vec![], vec![]);
        let union = builder
            .identity_origins
            .merge(Origin::Local(1), Origin::Local(2))
            .unwrap();
        builder.states[0] = Some(vec![(0, Class::Scalar, union)]);
        builder.stack = vec![(0, Class::Scalar, Origin::Local(1))];
        assert_eq!(builder.record_target(0, 0), Some(()));
        builder.stack = vec![(0, Class::Scalar, Origin::Local(3))];
        assert_eq!(builder.record_target(0, 0), None);
        assert!(builder.ops.is_empty());
        assert_eq!(builder.states[0], Some(vec![(0, Class::Scalar, union)]));
    }
}
