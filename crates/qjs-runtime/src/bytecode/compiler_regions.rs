//! Select abrupt jump routing from the final protected-region metadata.
//!
//! All break/continue sites start as AbruptJump. A region emitter only supplies
//! EnterTry/BeginFinally bounds; no separate compiler stack may omit a cleanup.
use super::ir::Op;

/// Rewrites each `AbruptJump` whose target stays inside every protected
/// region around it into a plain `Jump`. Regions are properly nested and each
/// starts right after its own opener, so one pass with a stack of the open
/// regions decides each jump by the innermost one: a target inside it is
/// inside all the enclosing ones too.
pub(super) fn route_jumps(code: &mut [Op]) {
    let mut open: Vec<std::ops::Range<usize>> = Vec::new();
    for pc in 0..code.len() {
        while open.last().is_some_and(|region| region.end <= pc) {
            open.pop();
        }
        match code[pc] {
            Op::AbruptJump(target) if open.last().is_none_or(|region| region.contains(&target)) => {
                code[pc] = Op::Jump(target);
            }
            Op::EnterTry { end, .. } | Op::BeginFinally(end) => {
                debug_assert!(end != usize::MAX && end <= code.len());
                debug_assert!(
                    open.last().is_none_or(|outer| end <= outer.end),
                    "protected regions must nest"
                );
                if end > pc + 1 {
                    open.push(pc + 1..end);
                }
            }
            _ => {}
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

    #[test]
    fn jumps_route_by_their_innermost_region() {
        let enter = |end| Op::EnterTry {
            catch: None,
            finally: None,
            end,
            catch_scope: None,
            cleanup_slots: vec![],
        };
        let mut code = vec![
            enter(9),
            Op::AbruptJump(8),
            enter(6),
            Op::AbruptJump(5),
            Op::AbruptJump(8),
            Op::ExitTry,
            Op::AbruptJump(1),
            Op::AbruptJump(9),
            Op::ExitTry,
            Op::AbruptJump(3),
        ];
        route_jumps(&mut code);
        // Inside the outer region only.
        assert!(matches!(code[1], Op::Jump(8)));
        // Inside the inner region: its own target stays within it, while one
        // in the outer region leaves the inner one.
        assert!(matches!(code[3], Op::Jump(5)));
        assert!(matches!(code[4], Op::AbruptJump(8)));
        // Back in the outer region once the inner one has ended.
        assert!(matches!(code[6], Op::Jump(1)));
        assert!(matches!(code[7], Op::AbruptJump(9)));
        // Outside every region.
        assert!(matches!(code[9], Op::Jump(3)));
    }
}
