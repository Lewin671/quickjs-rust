//! Select abrupt jump routing from the final protected-region metadata.
//!
//! All break/continue sites start as AbruptJump. A region emitter only supplies
//! EnterTry/BeginFinally bounds; no separate compiler stack may omit a cleanup.
use super::ir::Op;

pub(super) fn route_jumps(code: &mut [Op]) {
    let regions: Vec<_> = code
        .iter()
        .enumerate()
        .filter_map(|(pc, op)| match op {
            Op::EnterTry { end, .. } | Op::BeginFinally(end) => {
                debug_assert!(*end != usize::MAX && *end <= code.len());
                Some(pc + 1..*end)
            }
            _ => None,
        })
        .collect();
    for (pc, op) in code.iter_mut().enumerate() {
        if let Op::AbruptJump(target) = *op
            && !regions
                .iter()
                .any(|region| region.contains(&pc) && !region.contains(&target))
        {
            *op = Op::Jump(target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_routing_depends_only_on_emitted_region_bounds() {
        let mut code = vec![
            Op::EnterTry {
                catch: None,
                finally: None,
                end: 4,
                catch_scope: None,
                cleanup_slots: vec![],
            },
            Op::AbruptJump(3),
            Op::AbruptJump(7),
            Op::ExitTry,
            Op::BeginFinally(7),
            Op::AbruptJump(7),
            Op::EndFinally,
            Op::AbruptJump(0),
        ];
        route_jumps(&mut code);
        assert!(matches!(code[1], Op::Jump(3)));
        assert!(matches!(code[2], Op::AbruptJump(7)));
        assert!(matches!(code[5], Op::AbruptJump(7)));
        assert!(matches!(code[7], Op::Jump(0)));
    }
}
