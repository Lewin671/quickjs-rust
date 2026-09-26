//! Copy forwarding (`forward`): the rewrite on hand-built programs, and
//! compiled loops that deoptimize at or right after a rewritten operation.

use std::collections::BTreeSet;

use super::forward::forward_copies;
use super::{Class, DeoptSite, TypedOp};
use crate::{Value, eval};

fn site(ip: u32, start: u32, len: u8) -> DeoptSite {
    DeoptSite { ip, start, len }
}

/// `MoveBoxed` then a read of the copy: the read takes the source, its
/// site entry names the source, and the copy is gone.
#[test]
fn a_copy_read_once_is_forwarded_into_its_consumer_and_its_site() {
    let mut ops = vec![
        TypedOp::MoveBoxed { dst: 0, src: 5 },
        TypedOp::GetNamed {
            dst: 0,
            object: 0,
            name: 0,
            cache: 0,
        },
        TypedOp::Exit {
            cond: 0,
            exit_ip: 9,
        },
    ];
    let mut sites = vec![site(1, 0, 0), site(2, 0, 1), site(3, 1, 0)];
    let mut entries = vec![(Class::Boxed, 0)];
    let pinned = BTreeSet::from([(true, 5)]);
    forward_copies(&mut ops, &mut sites, &mut entries, &pinned);
    assert!(matches!(
        ops[0],
        TypedOp::GetNamed {
            dst: 0,
            object: 5,
            ..
        }
    ));
    assert_eq!(ops.len(), 2);
    assert_eq!(sites.len(), 2);
    assert_eq!(entries, vec![(Class::Boxed, 5)]);
}

/// A copy read again later, a copy into a pinned register and a consumer
/// something jumps to all keep their copy.
#[test]
fn copies_that_are_read_again_pinned_or_jumped_past_stay() {
    let pinned = BTreeSet::from([(true, 5), (true, 6)]);
    let reread = vec![
        TypedOp::MoveBoxed { dst: 0, src: 5 },
        TypedOp::GetNamed {
            dst: 1,
            object: 0,
            name: 0,
            cache: 0,
        },
        TypedOp::SetNamed {
            object: 0,
            name: 0,
            value: 1,
            cache: 1,
        },
        TypedOp::Exit {
            cond: 0,
            exit_ip: 9,
        },
    ];
    let into_pinned = vec![
        TypedOp::MoveBoxed { dst: 6, src: 5 },
        TypedOp::GetNamed {
            dst: 1,
            object: 6,
            name: 0,
            cache: 0,
        },
        TypedOp::Exit {
            cond: 0,
            exit_ip: 9,
        },
    ];
    let jumped_to = vec![
        TypedOp::JumpIfFalsy { cond: 0, target: 2 },
        TypedOp::MoveBoxed { dst: 0, src: 5 },
        TypedOp::GetNamed {
            dst: 1,
            object: 0,
            name: 0,
            cache: 0,
        },
        TypedOp::SetNamed {
            object: 6,
            name: 0,
            value: 1,
            cache: 1,
        },
        TypedOp::Exit {
            cond: 0,
            exit_ip: 9,
        },
    ];
    for program in [reread, into_pinned, jumped_to] {
        let mut ops = program.clone();
        let mut sites: Vec<DeoptSite> = (0..ops.len()).map(|at| site(at as u32, 0, 0)).collect();
        forward_copies(&mut ops, &mut sites, &mut [], &pinned);
        assert_eq!(format!("{ops:?}"), format!("{program:?}"));
    }
}

/// A copy nothing reads is deleted and jumps past it are retargeted.
#[test]
fn a_dead_copy_is_deleted_and_jumps_retargeted() {
    let mut ops = vec![
        TypedOp::Move { dst: 0, src: 7 },
        TypedOp::Binary {
            dst: 0,
            op: qjs_ast::BinaryOp::Lt,
            left: 7,
            right: 8,
        },
        TypedOp::JumpIfFalsy { cond: 0, target: 4 },
        TypedOp::Jump { target: 0 },
        TypedOp::Exit {
            cond: 0,
            exit_ip: 9,
        },
    ];
    let mut sites: Vec<DeoptSite> = (0..ops.len()).map(|at| site(at as u32, 0, 0)).collect();
    let pinned = BTreeSet::from([(false, 7), (false, 8)]);
    forward_copies(&mut ops, &mut sites, &mut [], &pinned);
    assert_eq!(ops.len(), 4);
    assert!(matches!(ops[1], TypedOp::JumpIfFalsy { target: 3, .. }));
    assert!(matches!(ops[2], TypedOp::Jump { target: 0 }));
}

/// Loops whose element, field and equality operands are forwarded, run
/// through deoptimizations at the rewritten operation (an equality that
/// must call `valueOf`) and after it (a hole, an accessor, a string field
/// in a numeric sum). Expected values from V8.
#[test]
fn forwarded_loops_deoptimize_to_the_same_results() {
    let source = r#"function run() {
    var out = [];
    function find(list, obj) { for (var i = 0; i < list.length; i++) { if (list[i].pos == obj.pos) return i; } return -1; }
    var p = {}, list = [];
    for (var i = 0; i < 20; i++) list.push({ pos: i == 13 ? p : {} });
    out.push(find(list, { pos: p }));
    // a hole: the forwarded element read deopts mid-loop
    var holey = list.slice(); delete holey[5]; 
    try { out.push(find(holey, { pos: p })); } catch (e) { out.push(e instanceof TypeError); }
    // an accessor on one element: the forwarded receiver's read deopts
    var acc = list.slice(); acc[7] = { get pos() { out.push('get'); return p; } };
    out.push(find(acc, { pos: p }));
    // the hoisted operand read through a getter on entry
    out.push(find(list, { get pos() { return list[3].pos; } }));
    // strings and numbers compared by value
    var s = []; for (var i = 0; i < 9; i++) s.push({ pos: 'k' + i });
    out.push(find(s, { pos: 'k' + 6 }));
    // a sum whose operand copy feeds a deopting add
    function sum(xs) { var t = 0; for (var i = 0; i < xs.length; i++) t = t + xs[i].v; return t; }
    var vs = []; for (var i = 0; i < 10; i++) vs.push({ v: i });
    out.push(sum(vs)); vs[4] = { v: 'x' }; out.push(sum(vs));
    // an object compared with a string: the equality over the forwarded
    // hoisted operand calls valueOf, so it stops the program
    function findV(list, obj) { for (var i = 0; i < list.length; i++) { if (list[i].pos == obj.pos) return i; } return -1; }
    var calls = 0, objs = [];
    for (var i = 0; i < 30; i++) objs.push({ pos: 'k' + i });
    objs[20] = { pos: { valueOf: function () { calls++; return 'x'; } } };
    objs[25] = { pos: 'k' };
    out.push(findV(objs, { pos: 'k' }), calls);
    return out.join();
}"#;
    assert_eq!(
        eval(&format!("{source} run();")),
        Ok(Value::String(
            "13,true,get,7,3,6,45,6x56789,25,1".to_owned().into()
        ))
    );
}
