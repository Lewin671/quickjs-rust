//! Behavioural tests for the typed-loop tier: admission, deoptimization,
//! property access, calls, and the corpus shapes that motivated each.

use crate::{Value, eval};

fn nested_function(source: &str) -> crate::bytecode::Bytecode {
    let script = qjs_parser::parse_script(source).expect("source should parse");
    let bytecode = crate::bytecode::compile_script(&script).expect("source should compile");
    bytecode
        .code
        .iter()
        .find_map(|op| match op {
            super::super::ir::Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
            _ => None,
        })
        .expect("function bytecode should be nested in the script")
}

#[test]
fn typed_loop_scratch_pool_reuses_only_cleared_storage() {
    let bytecode = nested_function(
        "function run(n) { var total = 0; for (var i = 0; i < n; i++) { total += i; } return total; }",
    );
    let programs = super::compile_all(&bytecode);
    let program = programs.first().expect("loop should be admitted");
    assert!(program.scratch_pool.get().is_none());

    let mut first = program.take_scratch();
    first.registers.push(super::Typed::Number(1.0));
    first.boxed.push(Value::Number(2.0));
    let register_capacity = first.registers.capacity();
    let boxed_capacity = first.boxed.capacity();
    program.recycle_scratch(first);

    let reused = program.take_scratch();
    assert!(reused.registers.is_empty());
    assert!(reused.receivers.is_empty());
    assert!(reused.boxed.is_empty());
    assert!(reused.sloppy_global_writes.is_empty());
    assert!(reused.registers.capacity() >= register_capacity);
    assert!(reused.boxed.capacity() >= boxed_capacity);
    program.recycle_scratch(reused);

    // A nested call receives fresh storage when the one sequential-entry
    // slot is occupied, and its later return cannot grow the pool.
    let mut active = program.take_scratch();
    active.registers.push(super::Typed::Number(4.0));
    let nested = program.take_scratch();
    assert!(nested.registers.is_empty());
    program.recycle_scratch(nested);
    assert_eq!(active.registers, vec![super::Typed::Number(4.0)]);
    program.recycle_scratch(active);
    assert_eq!(
        program
            .scratch_pool
            .get()
            .expect("taking scratch should initialize the pool")
            .borrow()
            .len(),
        1
    );
}

#[test]
fn typed_loop_compacts_register_files_to_used_stack_depth() {
    let scalar_bytecode = nested_function(
        "function run(n) { var total = 0; for (var i = 0; i < n; i++) { total += i; } return total; }",
    );
    let scalar_program = super::compile_all(&scalar_bytecode)
        .into_iter()
        .next()
        .expect("scalar loop should be admitted");
    assert!(scalar_program.register_count < super::MAX_STACK_DEPTH);
    assert_eq!(scalar_program.boxed_count, 0);

    let dense_bytecode = nested_function(
        "function run(n) { var values = [1, 2, 4]; var total = 0; for (var i = 0; i < n; i++) { total += values[i % 3]; } return total; }",
    );
    let dense_program = super::compile_all(&dense_bytecode)
        .into_iter()
        .next()
        .expect("dense loop should be admitted");
    assert!(dense_program.register_count < super::MAX_STACK_DEPTH);
    assert!(dense_program.boxed_count > 0);
    assert!(dense_program.boxed_count < super::MAX_STACK_DEPTH);
    assert_eq!(
        eval(
            "function run(n) { var values = [1, 2, 4]; var total = 0; for (var i = 0; i < n; i++) { total += values[i % 3]; } return total; } run(6);"
        ),
        Ok(Value::Number(14.0))
    );
}

/// Every case here is a loop the typed tier accepts. The expected values are
/// what the interpreter produces for the same source, so a divergence in the
/// register program shows up as a failing assertion rather than a silent
/// wrong answer.
#[test]
fn typed_loops_match_interpreted_results() {
    // Arithmetic, an if/else body, and a shift — the shape the specialized
    // tiers decline because of the branch.
    assert_eq!(
        eval(
            "function run(n) { var a = 0, b = 1, c = 0;\
               for (var i = 0; i < n; i++) {\
                 if (a > b) { c = a - (b >> i); b = (a >> i) + b; a = c; }\
                 else { c = a + (b >> i); b = -(a >> i) + b; a = c; }\
               }\
               return a + ':' + b + ':' + c; }\
             run(40);"
        ),
        Ok(Value::String("2:1:2".to_owned().into()))
    );
    // An element read from a dense array in a local slot.
    assert_eq!(
        eval(
            "function run(n) { var table = [1, 2, 4, 8, 16], total = 0;\
               for (var i = 0; i < n; i++) { total = total + table[i % 5]; }\
               return total; }\
             run(20);"
        ),
        Ok(Value::Number(124.0))
    );
    // Every admitted operator, so the register program's semantics are
    // pinned against the interpreter's.
    assert_eq!(
        eval(
            "function run() { var s = 0;\
               for (var i = 1; i < 12; i++) {\
                 s += i * 3 - 1;\
                 s += i / 2;\
                 s += i % 4;\
                 s += i ** 2;\
                 s += (i << 2) | (i >> 1);\
                 s += (i & 6) ^ (i >>> 1);\
                 s += -i;\
                 s += ~i;\
                 if (i < 5) s += 1;\
                 if (i <= 5) s += 1;\
                 if (i > 5) s += 1;\
                 if (i >= 5) s += 1;\
                 if (i === 5) s += 1;\
                 if (i !== 5) s += 1;\
                 if (!(i === 5)) s += 1;\
               }\
               return s; }\
             run();"
        ),
        Ok(Value::Number(980.0))
    );
    // A zero-iteration loop leaves everything untouched.
    assert_eq!(
        eval(
            "function run() { var s = 7; for (var i = 0; i < 0; i++) { s = s + 1; } return s + ':' + i; } run();"
        ),
        Ok(Value::String("7:0".to_owned().into()))
    );
}

#[test]
fn typed_loops_write_dense_elements() {
    // A branchy read-modify-write over a dense array: the shape the
    // specialized tiers decline, and the reason the element-assignment
    // idiom is recognized as a unit.
    assert_eq!(
        eval(
            "function run() { var a = [0, 1, 2, 3, 4];                   for (var r = 0; r < 3; r++) {                     for (var i = 0; i < 5; i++) {                       if (a[i] > 1) { a[i] = a[i] - 1; } else { a[i] = a[i] + 1; }                     }                   }                   return a.join(','); }                 run();"
        ),
        Ok(Value::String("1,2,1,2,1".to_owned().into()))
    );
    // The assignment's value is the expression's value.
    assert_eq!(
        eval(
            "function run() { var a = [1, 2, 3], last = 0;                   for (var i = 0; i < 3; i++) { last = (a[i] = a[i] * 2); }                   return a.join(',') + ':' + last; }                 run();"
        ),
        Ok(Value::String("2,4,6:6".to_owned().into()))
    );
    // Growth, a frozen array, a hole, and an own indexed descriptor all stay
    // on the observable path.
    assert_eq!(
        eval(
            "function run() { var a = [1]; for (var i = 0; i < 4; i++) { a[i] = i; } return a.join(','); } run();"
        ),
        Ok(Value::String("0,1,2,3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function run() { var a = Object.freeze([1, 2]);                   for (var i = 0; i < 2; i++) { a[i] = 9; }                   return a.join(','); }                 run();"
        ),
        Ok(Value::String("1,2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function run() { var a = [1, 2];                   Object.defineProperty(a, '1', { value: 5, writable: false, configurable: true, enumerable: true });                   for (var i = 0; i < 2; i++) { a[i] = i + 10; }                   return a.join(','); }                 run();"
        ),
        Ok(Value::String("10,5".to_owned().into()))
    );
}

#[test]
fn typed_loops_write_dense_elements_with_computed_scalar_indices() {
    let source = "function run(n) { var values = [0, 1, 2, 3, 4, 5, 6, 7]; for (var i = 0; i < n; i++) { values[(i + 1) & 7] = values[(i + 3) & 7] + i; } return values.join(','); }";
    let bytecode = nested_function(source);
    assert_eq!(
        super::compile_all(&bytecode).len(),
        1,
        "{:#?}",
        bytecode.code
    );
    assert_eq!(
        eval(&format!("{source} run(8);")),
        Ok(Value::String("12,3,5,7,9,11,5,9".to_owned().into()))
    );
    // A computed index that grows the Array declines at the write and
    // replays the first uncommitted assignment through ordinary semantics.
    assert_eq!(
        eval(
            "function run() { var values = [1]; for (var i = 0; i < 3; i++) { values[(i + 1) & 3] = i; } return values.join(','); } run();"
        ),
        Ok(Value::String("1,0,1,2".to_owned().into()))
    );
    // The key is evaluated before the value. The value mutates `i`, so
    // using its register directly for the dense write would store at a
    // different element than ordinary JavaScript.
    let order_source = "function run(n) { var values = [0, 0, 0, 0]; for (var i = 0; i < n; i++) { values[(i + 1) & 3] = (i = i + 1); } return values.join(',') + ':' + i; }";
    assert_eq!(super::compile_all(&nested_function(order_source)).len(), 1);
    assert_eq!(
        eval(&format!("{order_source} run(4);")),
        Ok(Value::String("0,1,0,3:4".to_owned().into()))
    );
}

/// A call whose callee is a global rather than a frame-local is answered
/// through the fast native table with boxed arguments -- `parseInt` over a
/// substring is every hex decoder -- and a global that turns out to be an
/// ordinary function deoptimizes to the interpreter's call.
#[test]
fn typed_loops_call_global_natives_with_boxed_arguments() {
    let source = "function hex(str) { var out = []; for (var i = 0; i < str.length; i += 8) out.push(parseInt(str.substr(i, 8), 16) ^ 0); return out.join(); }";
    let programs = super::compile_all(&nested_function(source));
    assert_eq!(programs.len(), 1, "{source}");
    assert!(
        programs[0].ops.iter().any(|op| matches!(
            op,
            super::TypedOp::CallClosedFormLeaf { arity, .. } if *arity == 2 | super::BOXED_ARGUMENTS
        )),
        "{:#?}",
        programs[0].ops
    );
    assert_eq!(
        eval(&format!("{source} hex('ffb7317e00000001');")),
        Ok(Value::String("-4771458,1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var F = function (x) { return x + 1; };\
             function g(n) { var s = 0; for (var i = 0; i < n; i++) { s += F(i); } return s; }\
             g(4);"
        ),
        Ok(Value::Number(10.0))
    );
    assert_eq!(
        eval(
            "function bad(n) { var k = 0; for (var i = 0; i < n; i++) { try { decodeURIComponent('%'); } catch (error) { k += error instanceof URIError ? 1 : 100; } } return k; }\
             bad(3);"
        ),
        Ok(Value::Number(3.0))
    );
}

/// A dense read that finds no element -- a hole below the length or an
/// index past it -- answers `undefined` when no prototype on the chain can
/// supply an indexed property, and deoptimizes to the interpreter when one
/// can. `bin[i >> 5] |= word` over an array that starts empty is every
/// hash's input conversion.
#[test]
fn typed_loops_read_missing_elements_as_undefined_unless_the_chain_answers() {
    assert_eq!(
        eval(
            "function conv(str) { var bin = Array(); for (var i = 0; i < str.length * 8; i += 8) bin[i >> 5] |= (str.charCodeAt(i / 8) & 255) << (i % 32); return bin.join(); }\
             conv('abcdefgh');"
        ),
        Ok(Value::String("1684234849,1751606885".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var holes = [1, , 3]; function read(n) { var s = ''; for (var i = 0; i < n; i++) { s += holes[i] + ','; } return s; }\
             read(4);"
        ),
        Ok(Value::String("1,undefined,3,undefined,".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Array.prototype[3] = 'proto'; function read(n) { var a = [1]; var s = ''; for (var i = 0; i < n; i++) { s += a[i] + ','; } return s; }\
             read(5);"
        ),
        Ok(Value::String(
            "1,undefined,undefined,proto,undefined,".to_owned().into()
        ))
    );
    // The `undefined` such a read produces coerces through the scalar
    // operators exactly as ToNumber does: NaN for arithmetic and comparison,
    // 0 for the bitwise operators, and never through an equality.
    assert_eq!(
        eval(
            "function f(n) { var a = [], s = 0, t = '';\
               for (var i = 0; i < n; i++) { a[i >> 1] |= i; s += a[9] + 1; t += (a[9] < 1) + ',' + (true + 1) + ',' + (a[9] == 0) + ',' + (-a[9]) + ',' + (~a[9]) + ';'; }\
               return a.join() + '|' + s + '|' + t; }\
             f(4);"
        ),
        Ok(Value::String(
            "1,3|NaN|false,2,false,NaN,-1;false,2,false,NaN,-1;false,2,false,NaN,-1;false,2,false,NaN,-1;"
                .to_owned()
                .into()
        ))
    );
}

/// A helper call may take up to four arguments -- sha1's round function
/// `ft(t, b, c, d)` -- whether it is flattened into the region's helper
/// graph or answered as a closed-form leaf; a fifth argument keeps the
/// interpreter.
#[test]
fn typed_loops_flatten_helpers_of_up_to_four_arguments() {
    // `nested_function` takes the first function in the script, so the loop
    // body comes first and its helper after.
    let source = "function run(n) { var a = 0x67452301, b = 0xEFCDAB89, c = 0x98BADCFE, d = 0x10325476, acc = 0;\
          for (var t = 0; t < n; t++) { acc = (acc + ft(t, b, c, d) + t) | 0; } return acc; }\
        function ft(t, b, c, d) { if (t < 20) return (b & c) | ((~b) & d); if (t < 40) return b ^ c ^ d; return (b & c) | (b & d) | (c & d); }";
    let programs = super::compile_all(&nested_function(source));
    assert_eq!(programs.len(), 1, "{source}");
    assert!(
        programs[0]
            .ops
            .iter()
            .any(|op| matches!(op, super::TypedOp::CallClosedFormLeaf { arity: 4, .. })),
        "{:#?}",
        programs[0].ops
    );
    assert_eq!(
        eval(&format!("{source} run(60);")),
        Ok(Value::Number(-291_943_762.0))
    );
    let wide = "function run(n) { var s = 0; for (var i = 0; i < n; i++) { s += five(i, 1, 2, 3, 4); } return s; } function five(a, b, c, d, e) { return a + b + c + d + e; }";
    assert!(super::compile_all(&nested_function(wide)).is_empty());
    assert_eq!(eval(&format!("{wide} run(10);")), Ok(Value::Number(145.0)));
}

/// A compound element assignment carries `RequireObjectCoercible` and
/// `ToPropertyKeyForAccess` in its key range and discards the checked
/// receiver with `Pop`; the region compiles through the ordinary path with
/// runtime guards, and a receiver the guard cannot pass deoptimizes to the
/// interpreter's TypeError.
#[test]
fn typed_loops_compile_compound_element_assignments() {
    let source = "function xor(temp, table, i, nk) { for (var t = 0; t < 4; t++) temp[t] ^= table[i / nk][t]; return temp.join(); }";
    let programs = super::compile_all(&nested_function(source));
    assert_eq!(programs.len(), 1);
    assert!(
        programs[0]
            .ops
            .iter()
            .any(|op| matches!(op, super::TypedOp::Guard { .. })),
        "{:#?}",
        programs[0].ops
    );
    assert_eq!(
        eval(&format!(
            "{source} xor([1, 2, 3, 4], [[8, 8, 8, 8], [16, 16, 16, 16]], 2, 2);"
        )),
        Ok(Value::String("17,18,19,20".to_owned().into()))
    );
    assert_eq!(
        eval(&format!(
            "{source} var caught = 0; try {{ xor(null, [[1]], 0, 1); }} catch (error) {{ caught = error instanceof TypeError ? 1 : 2; }} caught;"
        )),
        Ok(Value::Number(1.0))
    );
}

/// A key expression that writes a local cannot be lowered as a dense write
/// with its temporaries elided -- that would reorder the write -- so the
/// element-assignment lowering declines by inspection and the ordinary
/// per-instruction path, which keeps the interpreter's order, compiles the
/// region instead.
#[test]
fn typed_loop_computed_index_with_assignment_takes_the_ordinary_path() {
    let source = "function run() { var values = [0, 0, 0, 0], key = 0; for (var i = 0; i < 4; i++) { values[key = (i + 1) & 3] = i; } return values.join(',') + ':' + key; }";
    let bytecode = nested_function(source);
    let programs = super::compile_all(&bytecode);
    assert_eq!(programs.len(), 1, "{:#?}", bytecode.code);
    assert!(
        !programs[0]
            .ops
            .iter()
            .any(|op| matches!(op, super::TypedOp::DenseWrite { .. })),
        "{:#?}",
        programs[0].ops
    );
    assert_eq!(
        eval(&format!("{source} run();")),
        Ok(Value::String("3,0,1,2:0".to_owned().into()))
    );
}

#[test]
fn typed_loops_read_globals_only_when_that_is_equivalent() {
    // A global read is hoisted to loop entry, which is equivalent because
    // the region provably writes no global and cannot call user code.
    assert_eq!(
        eval(
            "var k = 3; function run(n) { var s = 0;                   for (var i = 0; i < n; i++) { if (i % 7 > k) { s += k; } else { s -= k; } }                   return s; }                 run(14);"
        ),
        Ok(Value::Number(-6.0))
    );
    // Each entry re-reads, so a value changed between loops is picked up.
    assert_eq!(
        eval(
            "var k = 1; function run() { var s = 0;                   for (var i = 0; i < 3; i++) { if (i > 0) { s += k; } else { s -= k; } }                   return s; }                 var first = run(); k = 10; first + ':' + run();"
        ),
        Ok(Value::String("1:10".to_owned().into()))
    );
    // A region that writes a global keeps the observable path, so every
    // iteration sees the previous one's write.
    assert_eq!(
        eval(
            "var g = 5; function run() { var s = 0; for (var i = 0; i < 4; i++) { s += g; g = g + 1; } return s + ':' + g; } run();"
        ),
        Ok(Value::String("26:9".to_owned().into()))
    );
    // An accessor on the global object is called once per read, so the
    // region declines.
    assert_eq!(
        eval(
            "var calls = 0;                 Object.defineProperty(globalThis, 'probe', { get: function () { calls++; return 2; }, configurable: true });                 function run() { var s = 0; for (var i = 0; i < 5; i++) { if (i > 1) { s += probe; } else { s -= probe; } } return s; }                 var result = run(); result + ':' + calls;"
        ),
        Ok(Value::String("2:5".to_owned().into()))
    );
}

#[test]
fn typed_loops_sync_existing_sloppy_numeric_globals() {
    let source = "var typedLoopSloppyProbe = 0; function run(n) { var total = typedLoopSloppyProbe = 0; for (var i = 0; i < n; i++) { typedLoopSloppyProbe = typedLoopSloppyProbe + Math.pow(i, 0); total = total + typedLoopSloppyProbe; } return total + ':' + typedLoopSloppyProbe; }";
    let bytecode = nested_function(source);
    assert_eq!(super::compile_all(&bytecode).len(), 1, "{bytecode:#?}");
    assert_eq!(
        eval(&format!("{source} run(5);")),
        Ok(Value::String("15:5".to_owned().into()))
    );

    // A native-call guard can fail after a completed sloppy-global write.
    // Resume at the exact call site so the generic callee and subsequent
    // string additions run once, without repeating the completed store.
    let deopt_source = "function run(n) { var total = typedLoopDeoptProbe = 0, box = { f: function () { return 'x'; } }; for (var i = 0; i < n; i++) { typedLoopDeoptProbe = typedLoopDeoptProbe + 1; total = total + box.f(i); total = total + typedLoopDeoptProbe; } return total + ':' + typedLoopDeoptProbe; }";
    assert_eq!(
        super::compile_all(&nested_function(deopt_source)).len(),
        1,
        "{deopt_source}"
    );
    assert_eq!(
        eval(&format!("{deopt_source} run(2);")),
        Ok(Value::String("0x1x2:2".to_owned().into()))
    );

    // A read-only descriptor must retain sloppy assignment's silent
    // failure rather than entering the prevalidated sink path.
    assert_eq!(
        eval(
            "Object.defineProperty(globalThis, 'typedLoopReadOnlyProbe', { value: 2, writable: false, configurable: true });\
             function run(n) { var total = typedLoopReadOnlyProbe = 0;\
               for (var i = 0; i < n; i++) {\
                 total = total + typedLoopReadOnlyProbe;\
                 typedLoopReadOnlyProbe = typedLoopReadOnlyProbe + 1;\
               }\
               return total + ':' + typedLoopReadOnlyProbe; }\
             run(3);"
        ),
        Ok(Value::String("6:2".to_owned().into()))
    );
}

#[test]
fn typed_loops_deoptimize_instead_of_guessing() {
    // A non-numeric operand mid-loop must fall back to the interpreter and
    // produce the interpreter's answer, including string concatenation.
    assert_eq!(
        eval(
            "function run() { var s = 0, flip = 3;\
               for (var i = 0; i < 6; i++) { if (i === 3) flip = 'x'; s = s + flip; }\
               return String(s); }\
             run();"
        ),
        Ok(Value::String("9xxx".to_owned().into()))
    );
    // A hole and an out-of-range index both leave the fast path.
    assert_eq!(
        eval(
            "function run() { var table = [1, , 3], total = 0;\
               for (var i = 0; i < 4; i++) { total = total + table[i]; }\
               return String(total); }\
             run();"
        ),
        Ok(Value::String("NaN".to_owned().into()))
    );
    // A receiver that stops being an array declines on the next entry.
    assert_eq!(
        eval(
            "function run() { var table = [1, 2, 3], total = 0;\
               for (var i = 0; i < 3; i++) { total = total + table[i]; }\
               table = { 0: 10, 1: 20, 2: 30 };\
               for (var j = 0; j < 3; j++) { total = total + table[j]; }\
               return total; }\
             run();"
        ),
        Ok(Value::Number(66.0))
    );
    // A captured counter keeps the observable path, so the closure sees
    // every value the interpreter would produce.
    assert_eq!(
        eval(
            "function run() { var seen = [], s = 0;\
               for (var i = 0; i < 3; i++) { s = s + i; seen.push(function () { return i; }); }\
               return s + ':' + seen[0]() + ':' + seen.length; }\
             run();"
        ),
        Ok(Value::String("3:3:3".to_owned().into()))
    );
}
/// A region whose body already changed something observable cannot be
/// replayed from the loop header, so a guard that fails halfway has to
/// resume at the exact instruction it stopped on.
#[test]
fn typed_loops_resume_where_they_stopped() {
    // The element write happens before the operand that leaves the fast
    // path, so replaying the iteration would double it.
    assert_eq!(
        eval(
            "function run() { var table = [0, 0, 0, 0], step = 2;\
               for (var i = 0; i < 4; i++) {\
                 table[i] = table[i] + 1;\
                 if (i === 2) step = 'x';\
                 table[i] = table[i] + step;\
               }\
               return table.join(','); }\
             run();"
        ),
        Ok(Value::String("3,3,1x,1x".to_owned().into()))
    );
    // Same for a named property write followed by a non-numeric operand.
    assert_eq!(
        eval(
            "function run() { var point = { total: 0, step: 1 }, log = 0;\
               for (var i = 0; i < 5; i++) {\
                 point.total = point.total + 1;\
                 if (i === 3) point.step = 'x';\
                 log = log + 1;\
                 point.total = point.total + point.step;\
               }\
               return String(point.total) + ':' + log; }\
             run();"
        ),
        Ok(Value::String("7x1x:5".to_owned().into()))
    );
}

/// The shapes the bytecode compiler fuses into single instructions — the
/// counted loop's comparison and its increment, an assignment that also
/// feeds the completion temporaries — are what ordinary loops are made of.
#[test]
fn typed_loops_accept_fused_loop_shapes() {
    assert_eq!(
        eval(
            "function run(n) { var s = 0; for (var i = 0; i < n; i++) { s += 2; } return s; }\
             run(40);"
        ),
        Ok(Value::Number(80.0))
    );
    // `continue` jumps backwards into the region and the body leaves the
    // completion bookkeeping behind on the operand stack.
    assert_eq!(
        eval(
            "function run(n) { var s = 0, i = 0;\
               while (i < n) { i++; if (i % 3 === 0) { continue; } s += i; }\
               return s; }\
             run(30);"
        ),
        Ok(Value::Number(300.0))
    );
    // A counted loop that runs backwards, so the fused increment is absent
    // and the update is the ordinary shape.
    assert_eq!(
        eval(
            "function run(n) { var s = 1; for (var i = n; i > 0; i--) { s = s * 1.5 - 0.5; }\
               return s.toFixed(4); }\
             run(20);"
        ),
        Ok(Value::String("1.0000".to_owned().into()))
    );
}

/// Control flow inside a region: both arms of a branch and a nested loop's
/// own backedge have to agree with the interpreter.
#[test]
fn typed_loops_handle_branches_and_nesting() {
    // The two arms of the conditional leave a different value on the stack
    // at the join.
    assert_eq!(
        eval(
            "function run(n) { var s = 0;\
               for (var i = 0; i < n; i++) { s += (i % 3 === 0 ? i * 2 : i - 1); }\
               return s; }\
             run(30);"
        ),
        Ok(Value::Number(550.0))
    );
    // A nested loop is one region with a backward jump inside it.
    assert_eq!(
        eval(
            "function run(n) { var s = 0;\
               for (var i = 0; i < n; i++) { for (var j = 0; j < i; j++) { s += j; } }\
               return s; }\
             run(20);"
        ),
        Ok(Value::Number(1140.0))
    );
    // A `break` out of the inner loop leaves the region at a point the
    // interpreter has to be able to continue from.
    assert_eq!(
        eval(
            "function run(n) { var s = 0;\
               for (var i = 0; i < n; i++) {\
                 for (var j = 0; j < 8; j++) { if (j > i) break; s += 1; }\
               }\
               return s; }\
             run(12);"
        ),
        Ok(Value::Number(68.0))
    );
}

/// An inner loop's array receiver that the outer loop reassigns must be
/// read through the slot's current value, not an array resolved once at
/// region entry. SunSpider's 3d-cube verification loop had this shape
/// and summed the first vector nine times.
#[test]
fn typed_loops_follow_a_reassigned_element_receiver() {
    let read_source = "function run(rows) { var sum = 0;\
         for (var i = 0; i < rows.length; ++i) {\
           var row = rows[i];\
           for (var j = 0; j < row.length; ++j) sum += row[j];\
         }\
         return sum; }";
    // Both the inner and the enclosing region compile; the enclosing one
    // is the program that must not resolve `row` once.
    assert_eq!(
        super::compile_all(&nested_function(read_source)).len(),
        2,
        "{read_source}"
    );
    assert_eq!(
        eval(&format!(
            "{read_source} run([[10, 5, 7], [20, 5, 7], [30, 5, 7]]);"
        )),
        Ok(Value::Number(96.0))
    );
    assert_eq!(
        eval(
            "function run(rows, factor) {\
               for (var i = 0; i < rows.length; ++i) {\
                 var row = rows[i];\
                 for (var j = 0; j < row.length; ++j) row[j] = row[j] * factor;\
               }\
               return rows.join(';'); }\
             run([[1, 2], [3, 4], [5, 6]], 10);"
        ),
        Ok(Value::String("10,20;30,40;50,60".to_owned().into()))
    );
}

/// A field read the region consumes as a number lands in the scalar file
/// and a scalar written to a field never passes through a boxed register.
/// The site caches persist across entries, and a field that stops being a
/// number deoptimizes to the interpreter's answer.
#[test]
fn typed_loops_read_and_write_numeric_fields_as_scalars() {
    let source = "function run(bodies, n) { for (var k = 0; k < n; k++) { for (var i = 0; i < bodies.length; i++) { var body = bodies[i]; body.v = body.v + body.m * 0.5; } } return bodies[0].v + ':' + bodies[1].v; }";
    let programs = super::compile_all(&nested_function(source));
    let inner = programs
        .iter()
        .find(|program| program.header > programs[0].header.min(programs[1].header))
        .expect("inner region should compile");
    assert!(
        inner
            .ops
            .iter()
            .any(|op| matches!(op, super::TypedOp::GetNamedTyped { .. })),
        "{:#?}",
        inner.ops
    );
    assert!(
        inner
            .ops
            .iter()
            .any(|op| matches!(op, super::TypedOp::SetNamedTyped { .. })),
        "{:#?}",
        inner.ops
    );
    assert!(
        !inner.ops.iter().any(|op| matches!(
            op,
            super::TypedOp::Unbox { .. } | super::TypedOp::Box { .. }
        )),
        "{:#?}",
        inner.ops
    );
    assert_eq!(
        eval(&format!(
            "function Body(v, m) {{ this.v = v; this.m = m; }} {source} run([new Body(1, 2), new Body(3, 4)], 10);"
        )),
        Ok(Value::String("11:23".into()))
    );
    // The second body's field is a string: the scalar read deoptimizes
    // and the interpreter concatenates exactly as it would have.
    assert_eq!(
        eval(&format!(
            "function Body(v, m) {{ this.v = v; this.m = m; }} {source} run([new Body(1, 2), new Body('s', 4)], 2);"
        )),
        Ok(Value::String("3:s22".into()))
    );
    // A read-only field deoptimizes the write instead of skipping it.
    assert_eq!(
        eval(&format!(
            "function Body(v, m) {{ this.v = v; this.m = m; }} {source} var frozen = new Body(5, 1); Object.freeze(frozen); run([new Body(1, 2), frozen], 3);"
        )),
        Ok(Value::String("4:5".into()))
    );
}

/// A builtin receiver such as `Math` keeps its properties in dynamic
/// storage with no slot to cache, so the site remembers the value against
/// the exact receiver and its revision; a write to the receiver is seen
/// on the next read. A literal whose field is rewritten every iteration
/// stays on the shared-shape path rather than resolving by name.
#[test]
fn typed_loops_cache_dynamic_receivers_by_revision_and_rewritten_literals_by_shape() {
    assert_eq!(
        eval(
            "function run(n) { var s = 0;\
               for (var i = -n; i < n; i++) { s += Math.abs(i) + Math.max(i, 0); }\
               return s; }\
             run(10);"
        ),
        Ok(Value::Number(145.0))
    );
    assert_eq!(
        eval(
            "var table = { k: 1 };\
             function run(n) { var s = 0;\
               for (var i = 0; i < n; i++) { s += table.k; if (i === 4) table.k = 100; }\
               return s; }\
             run(10);"
        ),
        Ok(Value::Number(505.0))
    );
    assert_eq!(
        eval(
            "function run(n) { var node = { f: 0, g: 1 }, s = 0;\
               for (var i = 0; i < n; i++) { node.f = node.f + node.g; s += node.f; }\
               return s; }\
             run(10);"
        ),
        Ok(Value::Number(55.0))
    );
}

/// Element writes through an array an element read produced, `push` on an
/// ordinary array, and string `length`/`charCodeAt` all stay native; a
/// frozen array, a custom `push`, and an own-prototype array take the
/// interpreter's answer.
#[test]
fn typed_loops_write_nested_elements_push_and_scan_strings() {
    assert_eq!(
        eval(
            "function sub(s, box) { for (var r = 0; r < 2; r++) { for (var c = 0; c < 3; c++) s[r][c] = box[s[r][c]]; } return s.join(';'); }\
             sub([[0, 1, 2], [2, 1, 0]], [10, 20, 30]);"
        ),
        Ok(Value::String("10,20,30;30,20,10".into()))
    );
    assert_eq!(
        eval(
            "function fill(n) { var out = []; for (var i = 0; i < n; i++) { out.push(i * 2); out.push({ v: i }); } return out.length + ':' + out[2 * n - 2] + ':' + out[2 * n - 1].v; }\
             fill(50);"
        ),
        Ok(Value::String("100:98:49".into()))
    );
    assert_eq!(
        eval(
            "var frozen = Object.freeze([]); var custom = []; custom.push = function (x) { return 'custom' + x; };\
             var own = []; Object.setPrototypeOf(own, { push: function (x) { return 'own' + x; } });\
             function run(n) { var caught = 0, last;\
               for (var i = 0; i < n; i++) { try { frozen.push(i); } catch (error) { caught++; } last = custom.push(i) + own.push(i); }\
               return caught + ':' + last; }\
             run(5);"
        ),
        Ok(Value::String("5:custom4own4".into()))
    );
    assert_eq!(
        eval(
            "function hash(text) { var h = 0; for (var i = 0; i < text.length; i++) { h = (h * 31 + text.charCodeAt(i)) | 0; } return h; }\
             hash('hello world');"
        ),
        Ok(Value::Number(1_794_106_052.0))
    );
}

/// Property reads, `Math` intrinsics, and constants no register file can
/// hold, all inside admitted regions.
#[test]
fn typed_loops_read_properties_and_call_math() {
    // An own field, an inherited method's value, and `Math.sqrt`.
    assert_eq!(
        eval(
            "function Point(x) { this.x = x; }\
             Point.prototype.bias = 3;\
             function run(n) { var p = new Point(4), s = 0;\
               for (var i = 0; i < n; i++) { s += Math.sqrt(p.x) + p.bias; }\
               return s; }\
             run(10);"
        ),
        Ok(Value::Number(50.0))
    );
    // A detached numeric intrinsic is an ordinary `Call`, not a
    // receiver-preserving `CallResolved`. It must nevertheless use the
    // same guarded native operation once the frame-local callee is known.
    let unbound_source = "function run(n) { var floor = Math.floor, s = 0; for (var i = 0; i < n; i++) { s = s + floor(i / 2); } return s; }";
    assert_eq!(
        super::compile_all(&nested_function(unbound_source)).len(),
        1,
        "{unbound_source}"
    );
    assert_eq!(
        eval(&format!("{unbound_source} run(8);")),
        Ok(Value::Number(12.0))
    );
    // A generic Function local now compiles: the region records a helper
    // site and loop entry tries to flatten whatever the slot holds. This
    // callee writes a global, so nothing can be flattened and the program
    // declines *before* the loop runs -- one failed preparation per frame,
    // not a deoptimization per iteration. The counter is what proves the
    // interpreter still performed every call.
    let fallback_source = "var typedLoopCallCount = 0; function run(n, callbackValue) { var numeric = callbackValue, total = 0; for (var i = 0; i < n; i++) { total = total + numeric(i); } return total + ':' + typedLoopCallCount; } function callback(value) { typedLoopCallCount++; return value; }";
    assert_eq!(
        super::compile_all(&nested_function(fallback_source)).len(),
        1,
        "{fallback_source}"
    );
    let overwritten_alias_source = "function run(n, callbackValue) { var numeric = Math.floor; numeric = callbackValue; var total = 0; for (var i = 0; i < n; i++) { total = total + numeric(i); } return total; }";
    assert_eq!(
        super::compile_all(&nested_function(overwritten_alias_source)).len(),
        1,
        "{overwritten_alias_source}"
    );
    assert_eq!(
        eval(&format!("{fallback_source} run(4, callback);")),
        Ok(Value::String("6:4".to_owned().into()))
    );
    assert_eq!(
        eval(&format!(
            "{overwritten_alias_source} run(6, function (v) {{ return v + 1; }});"
        )),
        Ok(Value::Number(21.0))
    );
    // A string constant inside the region is held boxed and still compares
    // the way the interpreter does.
    assert_eq!(
        eval(
            "function run(n) { var s = 0;\
               for (var i = 0; i < n; i++) { s += (typeof i === 'number' ? 1 : 0); }\
               return s; }\
             run(12);"
        ),
        Ok(Value::Number(12.0))
    );
    // An element that is an object, read through a frame-slot array: the
    // receiver has to survive into the operand stack if the read leaves the
    // fast path, so it cannot live in the unboxed register file.
    assert_eq!(
        eval(
            "function run(n) { var rows = [{ x: 5 }, { x: 6 }], s = 0;\
               for (var i = 0; i < n; i++) { s += rows[i % 2].x; }\
               return s; }\
             run(9);"
        ),
        Ok(Value::Number(49.0))
    );
    // Writing a field the region also reads keeps the object's own value.
    assert_eq!(
        eval(
            "function run(n) { var box = { total: 0 };\
               for (var i = 0; i < n; i++) { box.total = box.total + i; }\
               return box.total; }\
             run(15);"
        ),
        Ok(Value::Number(105.0))
    );
}

/// A loop whose body calls a function stays in the register program when
/// the callee is one a closed-form leaf evaluator can answer. The call was
/// already frameless before this; what changes is that its receiver,
/// callee, arguments, and result no longer round-trip through the operand
/// stack, which is what lets the surrounding loop run natively at all.
#[test]
fn closed_form_leaf_calls_keep_their_loop_in_registers() {
    // Compiling is not enough to assert: a region whose call it cannot
    // execute still produces a program, and then deoptimizes at the call on
    // its first iteration. What has to hold is that the program contains
    // the operation that answers the call in registers.
    fn calls_in_registers(source: &str) -> bool {
        super::compile_all(&nested_function(source))
            .iter()
            .any(|program| {
                program
                    .ops
                    .iter()
                    .any(|op| matches!(op, super::TypedOp::CallClosedFormLeaf { .. }))
            })
    }

    assert!(
        calls_in_registers(
            "function run(n, pool) { var t = 0;\
               for (var i = 0; i < n; i++) { t += pool[i & 63].advance(i); }\
               return t; }"
        ),
        "a prototype-dispatched method call should run in registers"
    );
    assert!(
        calls_in_registers(
            "function run(n, cs) { var t = 0;\
               for (var i = 0; i < n; i++) { t += cs[i & 3](i); }\
               return t; }"
        ),
        "a computed-callee call should run in registers"
    );
    // A hoisted `Math` receiver keeps the unboxed intrinsic operation
    // instead, so its result feeds arithmetic without a boxing round trip.
    assert!(
        !calls_in_registers(
            "function run(n) { var t = 0;\
               for (var i = 0; i < n; i++) { t += Math.sqrt(i); }\
               return t; }"
        ),
        "a Math call should stay on the intrinsic operation"
    );
}

/// The call operation runs user code, so each way its assumptions can stop
/// holding has to hand the loop back with the same answer the interpreter
/// gives. Every expected value here was cross-checked against QuickJS-NG.
#[test]
fn closed_form_leaf_calls_match_interpreted_results() {
    // `this` is the element, not the pool the element came from.
    assert_eq!(
        eval(
            "function Stepper(s) { this.step = s; }\
             Stepper.prototype.advance = function (v) { return v + this.step; };\
             function run(n) { var pool = [new Stepper(1), new Stepper(10)], t = 0;\
               for (var i = 0; i < n; i++) { t += pool[i % 2].advance(i); }\
               return t; }\
             run(10);"
        ),
        Ok(Value::Number(100.0))
    );
    // A computed callee takes its container as the receiver.
    assert_eq!(
        eval(
            "function run(n) { var a = [function (v) { return v + (this.length || 0); }], t = 0;\
               for (var i = 0; i < n; i++) { t += a[0](i); }\
               return t; }\
             run(10);"
        ),
        Ok(Value::Number(55.0))
    );
    // The callee is replaced mid-loop by a body the evaluators decline.
    assert_eq!(
        eval(
            "function run(n) { var o = { f: function (v) { return v + 1; } }, t = 0;\
               for (var i = 0; i < n; i++) {\
                 if (i === 4) { o.f = function (v) { return v + this.k; }; o.k = 100; }\
                 t += o.f(i);\
               }\
               return t; }\
             run(10);"
        ),
        Ok(Value::Number(649.0))
    );
    // ... by a native, ...
    assert_eq!(
        eval(
            "function run(n) { var o = { f: function (v) { return v + 1; } }, t = 0;\
               for (var i = 0; i < n; i++) { if (i === 4) { o.f = Math.abs; } t += o.f(-i); }\
               return t; }\
             run(10);"
        ),
        Ok(Value::Number(37.0))
    );
    // ... and by something not callable at all.
    assert_eq!(
        eval(
            "function run(n) { var o = { f: function (v) { return v + 1; } }, t = 0;\
               for (var i = 0; i < n; i++) { if (i === 4) { o.f = null; }\
                 try { t += o.f(i); } catch (e) { t += 1000; } }\
               return t; }\
             run(10);"
        ),
        Ok(Value::Number(6010.0))
    );
    // A leaf body that throws must throw, not be answered in registers.
    assert_eq!(
        eval(
            "function run(n) { var o = { f: function (v) { return v.q; } }, t = 0;\
               for (var i = 0; i < n; i++) {\
                 try { t += o.f(i === 5 ? null : { q: 1 }); } catch (e) { t += 500; }\
               }\
               return t; }\
             run(10);"
        ),
        Ok(Value::Number(509.0))
    );
    // A class constructor called without `new` must throw rather than be
    // answered from registers.
    assert_eq!(
        eval(
            "class C { constructor(v) { this.v = v; } }\
             function run(n) { var t = 0;\
               for (var i = 0; i < n; i++) { try { t += C(i); } catch (e) { t += 7; } }\
               return t; }\
             run(6);"
        ),
        Ok(Value::Number(42.0))
    );
    // An accessor that answers with a closed-form function still works.
    assert_eq!(
        eval(
            "function run(n) { var o = {}, g = function (v) { return v + 5; };\
               Object.defineProperty(o, 'f', { get: function () { return g; } });\
               var t = 0; for (var i = 0; i < n; i++) { t += o.f(i); }\
               return t; }\
             run(10);"
        ),
        Ok(Value::Number(95.0))
    );
    // A hoisted `Math` receiver keeps the unboxed intrinsic operation.
    assert_eq!(
        eval(
            "function run(n) { var t = 0;\
               for (var i = 0; i < n; i++) { t += Math.sqrt(i) + Math.min(i, 3); }\
               return t; }\
             run(10).toFixed(6);"
        ),
        Ok(Value::String("43.306001".to_owned().into()))
    );
}

/// A region is admitted on what it can execute, not on how much of its work
/// is boxed. Each source here crossed the old "more than a third of the
/// operations are boxed" rejection and so declined on every backedge, which
/// left its arithmetic, induction variable, and branches on the generic
/// dispatcher too.
#[test]
fn property_heavy_regions_are_admitted() {
    // An object held in a local across the property reads. One element read
    // plus one move plus one named read is already past the old ratio.
    let via_local = nested_function(
        "function run(n, pool) { var total = 0;\
           for (var i = 0; i < n; i++) { var row = pool[i & 63]; total += row.step; }\
           return total; }",
    );
    assert!(
        !super::compile_all(&via_local).is_empty(),
        "a region that holds an element in a local should be admitted"
    );

    // Two element reads in one iteration, without any local.
    let two_reads = nested_function(
        "function run(n, pool) { var total = 0;\
           for (var i = 0; i < n; i++) { total += pool[i & 63].step + pool[i & 63].carry; }\
           return total; }",
    );
    assert!(
        !super::compile_all(&two_reads).is_empty(),
        "a region with two element reads should be admitted"
    );

    // The generic-path sentinel's shape: an element read into a local, then
    // three named reads off it.
    let three_fields = nested_function(
        "function run(n, pool) { var total = 0;\
           for (var i = 0; i < n; i++) {\
             var row = pool[i & 63];\
             total += i + row.step + row.carry + row.rest;\
           }\
           return total; }",
    );
    assert!(
        !super::compile_all(&three_fields).is_empty(),
        "a region reading three fields per iteration should be admitted"
    );
}

/// Admission is a performance decision, so each newly admitted shape has to
/// answer exactly what the interpreter answers -- including when the
/// assumptions its operations guard stop holding partway through the loop.
#[test]
fn admitted_property_heavy_regions_match_interpreted_results() {
    // Receivers deliberately built with three different storage layouts, so
    // no single cached slot covers the site.
    assert_eq!(
        eval(
            "function run(n) { var pool = [];\
               for (var k = 0; k < 3; k++) {\
                 if (k === 0) pool.push({ step: 1, carry: 2, rest: 3, left: k });\
                 else if (k === 1) pool.push({ left: k, carry: 2, step: 1, rest: 3 });\
                 else pool.push({ left: k, rest: 3, extra: k, carry: 2, step: 1 });\
               }\
               var total = 0;\
               for (var i = 0; i < n; i++) {\
                 var row = pool[i % 3];\
                 total += i + row.step + row.carry + row.rest;\
               }\
               return total; }\
             run(30);"
        ),
        Ok(Value::Number(615.0))
    );
    // A getter installed before the loop: the named read has to leave the
    // register program and resume at the exact instruction it stopped on.
    assert_eq!(
        eval(
            "function run(n) { var row = { carry: 10 };\
               Object.defineProperty(row, 'step', { get: function () { return 2; } });\
               var pool = [row], total = 0;\
               for (var i = 0; i < n; i++) { var r = pool[0]; total += r.step + r.carry; }\
               return total; }\
             run(7);"
        ),
        Ok(Value::Number(84.0))
    );
    // The receiver's shape changes partway through, so the operation's
    // remembered storage slot stops being valid mid-loop.
    assert_eq!(
        eval(
            "function run(n) { var pool = [{ step: 1, carry: 2 }, { step: 3, carry: 4 }];\
               var total = 0;\
               for (var i = 0; i < n; i++) {\
                 var row = pool[i % 2];\
                 total += row.step + row.carry;\
                 if (i === 3) { pool[0] = { carry: 20, step: 10 }; }\
               }\
               return total; }\
             run(8);"
        ),
        Ok(Value::Number(94.0))
    );
    // An element that is not an object at all: the read has to deoptimize
    // rather than answer from the boxed register file.
    assert_eq!(
        eval(
            "function run(n) { var pool = [{ step: 1, carry: 2 }, { step: 3, carry: 4 }];\
               var total = 0;\
               for (var i = 0; i < n; i++) {\
                 if (i === 4) { pool[0] = 'text'; }\
                 var row = pool[i % 2];\
                 total += row.step === undefined ? 100 : row.step + row.carry;\
               }\
               return total; }\
             run(8);"
        ),
        Ok(Value::Number(234.0))
    );
}

/// The search loop is the shape both `Leave` and `BoxedEquality` exist for:
/// it leaves through its result rather than its header test, and it compares
/// by identity. Before either operation the whole region declined.
const SEARCH: &str = "function run(list, target) {\
       for (var i = 0; i < list.length; i++) {\
         if (list[i].pos == target) { return i; }\
       }\
       return -1; }";

#[test]
fn a_search_loop_compiles_to_an_identity_test_and_an_early_leave() {
    let bytecode = nested_function(SEARCH);
    let programs = super::compile_all(&bytecode);
    let program = programs.first().expect("search loop should be admitted");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, super::TypedOp::BoxedEquality { .. })),
        "{:#?}",
        program.ops
    );
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, super::TypedOp::Leave { .. })),
        "{:#?}",
        program.ops
    );
}

#[test]
fn a_search_loop_returns_the_interpreted_answer() {
    let script = format!(
        "{SEARCH} var a = {{}}; var b = {{}}; \
         run([{{ pos: a }}, {{ pos: b }}, {{ pos: a }}], b);"
    );
    assert_eq!(eval(&script), Ok(Value::Number(1.0)));
    let missing = format!("{SEARCH} run([{{ pos: {{}} }}], {{}});");
    assert_eq!(eval(&missing), Ok(Value::Number(-1.0)));
}

#[test]
fn a_comparison_that_would_coerce_still_runs_its_hook() {
    // `number == object` is `ToPrimitive` on the object, which is user code.
    // The tier must hand the instruction back rather than answer it, so the
    // `valueOf` hook has to run once per iteration.
    // Every name the loop body touches is a local or a parameter, so the
    // region really is admitted and the assertion really does exercise the
    // guard; the only global write lives inside the hook.
    let script = "function run(list, probe) { var hits = 0;\
           for (var i = 0; i < list.length; i++) {\
             if (list[i].pos == probe) { hits = hits + 1; } }\
           return hits; }\
         var calls = 0;\
         var probe = { valueOf: function () { calls = calls + 1; return 2; } };\
         run([{ pos: 1 }, { pos: 2 }, { pos: 3 }, { pos: 2 }], probe) * 100 + calls;";
    assert_eq!(eval(script), Ok(Value::Number(204.0)));
}

/// The helper graph exists for this shape: a numeric loop whose body calls
/// ordinary functions, which used to abort the whole region. This is the
/// `imaging-darkroom` pixel loop reduced to its call structure -- three
/// levels of ordinary call, an intrinsic, a captured number and a captured
/// function.
const DARKROOM_HELPERS: &str = "function FastLog2(x) { return Math.log(x) / Math.LN2; }\
     var LOG2_HALF = FastLog2(0.5);\
     function FastBias(b, x) { return Math.pow(x, FastLog2(b) / LOG2_HALF); }\
     function FastGain(g, x) { return (x < 0.5)\
         ? FastBias(1.0 - g, 2.0 * x) * 0.5\
         : 1.0 - FastBias(1.0 - g, 2.0 - 2.0 * x) * 0.5; }\
     function Clamp(x) { return (x < 0.0) ? 0.0 : ((x > 1.0) ? 1.0 : x); }\
     function pixel(x, contrast) { return FastGain(contrast, Clamp(x)); }\
     function run(data, n, contrast) { var total = 0;\
       for (var i = 0; i < n; i++) { total = total + pixel(data[i], contrast); }\
       return total; }";

fn named_function(source: &str, name: &str) -> crate::bytecode::Bytecode {
    let script = qjs_parser::parse_script(source).expect("source should parse");
    let bytecode = crate::bytecode::compile_script(&script).expect("source should compile");
    bytecode
        .code
        .iter()
        .find_map(|op| match op {
            super::super::ir::Op::NewFunction {
                name: actual,
                bytecode,
                ..
            } if actual.as_deref() == Some(name) => Some(bytecode.as_ref().clone()),
            _ => None,
        })
        .expect("named function should be nested in the script")
}

#[test]
fn a_nested_helper_graph_answers_exactly_as_the_interpreter_does() {
    // The loop is admitted and runs its calls through the flattened graph;
    // the straight-line sum has no loop at all, so it can only be
    // interpreted. The two must agree bit for bit.
    assert_eq!(
        super::compile_all(&named_function(DARKROOM_HELPERS, "run")).len(),
        1
    );
    let data = "[-0.5, 0.25, 0.75, 1.5, 0.5]";
    let looped = format!("{DARKROOM_HELPERS} run({data}, 5, 0.4);");
    let straight = format!(
        "{DARKROOM_HELPERS} var d = {data};\
         pixel(d[0], 0.4) + pixel(d[1], 0.4) + pixel(d[2], 0.4)\
           + pixel(d[3], 0.4) + pixel(d[4], 0.4);"
    );
    let Ok(Value::Number(interpreted)) = eval(&straight) else {
        panic!("straight-line sum should evaluate to a number");
    };
    assert!(interpreted.is_finite(), "{interpreted}");
    assert_eq!(eval(&looped), Ok(Value::Number(interpreted)));
}

#[test]
fn a_flattened_helper_reproduces_its_own_branches() {
    let script = "function Clamp(x) { return (x < 0.0) ? 0.0 : ((x > 1.0) ? 1.0 : x); }\
         function run(data, n) { var total = 0;\
           for (var i = 0; i < n; i++) { total = total + Clamp(data[i]); }\
           return total; }\
         run([-1, 0.25, 2, 0.5], 4) * 100;";
    assert_eq!(eval(script), Ok(Value::Number(175.0)));
}

#[test]
fn a_helper_replaced_mid_loop_stops_using_the_prepared_body() {
    // The prepared body is `twice`; the identity check at every call is
    // what makes the switch to `thrice` produce the interpreted answer
    // instead of the stale one.
    let script = "function twice(x) { return x * 2; }\
         function thrice(x) { return x * 3; }\
         function run(a, b, n) { var f = a, total = 0;\
           for (var i = 0; i < n; i++) { total = total + f(i); if (i === 1) { f = b; } }\
           return total; }\
         run(twice, thrice, 4);";
    assert_eq!(eval(script), Ok(Value::Number(17.0)));
}

#[test]
fn filling_a_hole_creates_the_own_property_it_should() {
    // `new Array(n)` is n holes, so this loop stores into a hole every
    // iteration -- the shape `access-nsieve` opens with.
    assert_eq!(
        eval(
            "function run(n) { var a = new Array(n);\
               for (var i = 0; i < n; i++) { a[i] = i * 2; }\
               return a.join(','); } run(5);"
        ),
        Ok(Value::string_from_utf8("0,2,4,6,8"))
    );
    // A hole the loop skips stays absent, and the length is untouched.
    assert_eq!(
        eval(
            "function run(n) { var a = new Array(n);\
               for (var i = 0; i < n; i++) { if (i % 2 === 0) { a[i] = i; } }\
               return (0 in a) + ':' + (1 in a) + ':' + a.length; } run(4);"
        ),
        Ok(Value::string_from_utf8("true:false:4"))
    );
    // Storing past the end grows the array, which is an ordinary store.
    assert_eq!(
        eval(
            "function run(n) { var a = [];\
               for (var i = 0; i < n; i++) { a[i] = i; }\
               return a.length * 10 + a[3]; } run(5);"
        ),
        Ok(Value::Number(53.0))
    );
}

/// These pin the semantics the tier's hole store must not change. They are
/// *not* witnesses for its guards: the tier and the interpreter are
/// required to agree, so a guard failure is invisible to any behavioural
/// test -- it only shows up as a deoptimization. The evidence that the
/// store is reached at all is `access-nsieve`, whose dispatched
/// instructions fall from 12,235,041 to 452.
#[test]
fn a_hole_that_is_not_an_ordinary_store_is_left_to_the_interpreter() {
    // A sealed array is not extensible, so materializing a hole must fail
    // rather than become a dense write.
    assert_eq!(
        eval(
            "function run(n) { var a = new Array(n); Object.seal(a); var wrote = 0;\
               for (var i = 0; i < n; i++) { a[i] = i; if (i in a) { wrote = wrote + 1; } }\
               return wrote; } run(3);"
        ),
        Ok(Value::Number(0.0))
    );
    // An inherited index accessor has to run, and the receiver must not
    // gain an own property at that index.
    assert_eq!(
        eval(
            "var seen = 0;\
             Object.defineProperty(Array.prototype, '1',\
               { set: function (v) { seen = v; }, configurable: true });\
             function run(n) { var a = new Array(n);\
               for (var i = 0; i < n; i++) { a[i] = i + 10; }\
               return a.hasOwnProperty(1) + ':' + seen + ':' + a[0]; } run(3);"
        ),
        Ok(Value::string_from_utf8("false:11:10"))
    );
}

#[test]
fn a_break_leaves_the_region_instead_of_declining_it() {
    // A `break` is an unconditional jump past the backedge. It used to
    // decline the whole loop; `Leave` gives it the same one-instruction
    // hand-back that the loop's own exit test already used.
    let source = "function run(a, n, needle) { var found = -1;\
         for (var i = 0; i < n; i++) { if (a[i] === needle) { found = i; break; } }\
         return found; }";
    let bytecode = named_function(source, "run");
    let programs = super::compile_all(&bytecode);
    assert_eq!(programs.len(), 1, "{:#?}", bytecode.code);
    assert!(
        programs[0]
            .ops
            .iter()
            .any(|op| matches!(op, super::TypedOp::Leave { .. })),
        "{:#?}",
        programs[0].ops
    );
    // Taken early, taken late, and never taken.
    assert_eq!(
        eval(&format!("{source} run([3, 7, 9, 7], 4, 7);")),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(&format!("{source} run([3, 7, 9, 5], 4, 5);")),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(&format!("{source} run([3, 7, 9, 7], 4, 4);")),
        Ok(Value::Number(-1.0))
    );
}

#[test]
fn a_fused_constant_index_read_lowers_like_its_unfused_form() {
    // The compiler fuses `a[0]` into one `GetPropIndex` whose encoding
    // carries the receiver slot. Two regions of `access-fannkuch` declined
    // on nothing else.
    let source = "function run(a, b, n) { var total = 0;\
         for (var i = 0; i < n; i++) { total = total + a[0] + b[i]; }\
         return total; }";
    let bytecode = named_function(source, "run");
    let programs = super::compile_all(&bytecode);
    assert_eq!(programs.len(), 1, "{:#?}", bytecode.code);
    assert_eq!(
        programs[0]
            .ops
            .iter()
            .filter(|op| matches!(op, super::TypedOp::DenseRead { .. }))
            .count(),
        2,
        "{:#?}",
        programs[0].ops
    );
    assert_eq!(
        eval(&format!("{source} run([7], [1, 2, 3, 4], 4);")),
        Ok(Value::Number(38.0))
    );
}

#[test]
fn a_fused_constant_index_read_answers_out_of_bounds_and_holes() {
    let source = "function run(a, n) { var total = 0;\
         for (var i = 0; i < n; i++) { total = total + (a[0] === undefined ? 1 : a[0]); }\
         return total; }";
    // Empty array: every read is out of bounds.
    assert_eq!(
        eval(&format!("{source} run([], 4);")),
        Ok(Value::Number(4.0))
    );
    // A hole reads as `undefined`, exactly as the interpreter reports it.
    assert_eq!(
        eval(&format!("{source} run(new Array(3), 4);")),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval(&format!("{source} run([5], 4);")),
        Ok(Value::Number(20.0))
    );
}

#[test]
fn a_self_recursive_helper_flattens_and_agrees_with_the_interpreter() {
    // `fib` reaches itself twice per body and keeps a local, which is the
    // shape `recursive_call_tree` has. Its index is reserved before the
    // walk, so the self-calls resolve to the body being built.
    let source = "function fib(n) { var a = 0; if (n < 2) { return n; }\
         a = fib(n - 1) + fib(n - 2); return a; }";
    let looped = format!(
        "{source} function run(n) {{ var total = 0;\
           for (var i = 0; i < n; i++) {{ total = total + fib(i); }}\
           return total; }} run(16);"
    );
    // 0+1+1+2+3+5+8+13+21+34+55+89+144+233+377+610
    assert_eq!(eval(&looped), Ok(Value::Number(1596.0)));
}

#[test]
fn a_recursion_deeper_than_the_native_bound_still_answers() {
    // Past the bound the flattened body stops and the interpreter runs the
    // call instead. Because a flattened body is pure, stopping anywhere is
    // not observable -- only the answer is. This pins that answer across
    // the boundary; the bound itself is a stack-resource choice and has no
    // behavioural witness.
    let source = "function down(n) { if (n <= 0) { return 0; } return 1 + down(n - 1); }";
    let looped = format!(
        "{source} function run(n, depth) {{ var total = 0;\
           for (var i = 0; i < n; i++) {{ total = total + down(depth); }}\
           return total; }} run(3, 400);"
    );
    assert_eq!(eval(&looped), Ok(Value::Number(1200.0)));
}

#[test]
fn a_helper_local_is_a_register_of_its_own() {
    let source = "function blend(a, b) { var mid = (a + b) * 0.5;\
         var lift = mid * mid; return lift - mid; }";
    let looped = format!(
        "{source} function run(data, n) {{ var total = 0;\
           for (var i = 0; i < n; i++) {{ total = total + blend(data[i], i); }}\
           return total; }} run([2, 4, 6, 8], 4) * 10;"
    );
    // mid = 1, 2.5, 4, 5.5 -> lift-mid = 0, 3.75, 12, 24.75 -> 40.5
    assert_eq!(eval(&looped), Ok(Value::Number(405.0)));
}

#[test]
fn a_helper_that_is_not_pure_arithmetic_declines_without_losing_its_effect() {
    // The body writes a global, which no flattened graph can express, so
    // preparation fails and the loop stays interpreted. The counter proves
    // every call still ran.
    let script = "var calls = 0;\
         function step(x) { calls = calls + 1; return x + 1; }\
         function run(n) { var total = 0;\
           for (var i = 0; i < n; i++) { total = total + step(i); }\
           return total; }\
         run(5) * 100 + calls;";
    assert_eq!(eval(script), Ok(Value::Number(1505.0)));
}

#[test]
fn strict_equality_over_boxed_operands_matches_the_interpreter() {
    let script = "function run(list, needle) { var hits = 0;\
           for (var i = 0; i < list.length; i++) {\
             if (list[i] === needle) { hits = hits + 1; } }\
           return hits; }\
         run(['a', 'b', 'a', 'c'], 'a') * 10 + run([1, 2, 1], '1');";
    assert_eq!(eval(script), Ok(Value::Number(20.0)));
}
