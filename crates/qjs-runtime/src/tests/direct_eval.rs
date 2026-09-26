use crate::{Value, eval};

/// A cached direct eval that writes and deletes no binding skips the
/// write-back to its caller's frame. What it reads is still current, a
/// function it calls still updates the caller's captured variables and the
/// realm, and an eval that does assign still writes back.
#[test]
fn a_read_only_direct_eval_leaves_its_caller_current() {
    assert_eq!(
        eval(
            "var out = [];
             function f(k) {
               var n = 1, plain = 10;
               function g() { n++; return n; }
               function h() { zz = (typeof zz === 'number' ? zz : 0) + k; return zz; }
               function del() { return plain; }
               for (var i = 0; i < 3; i++) out.push(eval('g()'), n, eval('h()'), eval('plain + n'));
               eval('plain = 20'); out.push(plain);
               out.push(eval('del()'));
               return n;
             }
             out.push(f(1), f(2), zz);
             var gv = 1; function r() { return eval('gv'); }
             out.push(r()); gv = 7; out.push(r(), eval('gv'));
             out.join(',');"
        ),
        Ok(Value::String(
            "2,2,1,12,3,3,2,13,4,4,3,14,20,20,2,2,5,12,3,3,7,13,4,4,9,14,20,20,4,4,9,1,7,7"
                .to_owned()
                .into()
        ))
    );
}

/// A function literal in eval code that resolves nothing in the eval's
/// scope is created without it; one that does, or that could see a later
/// declaration, keeps it.
#[test]
fn eval_created_closures_resolve_the_same_names_with_or_without_the_eval_scope() {
    assert_eq!(
        eval(
            "var out = [];
             function outer(a) { var local = 'L'; eval('var g = function () { return local + a + typeof String; }'); return g(); }
             function own() { eval('var y = 3; var g = function () { return y; }; y = 4;'); return g(); }
             function receiver() { eval('var g = function () { return this === globalThis ? \"G\" : this.v; }'); return g() + g.call({ v: 'o' }); }
             function args() { eval('var g = function () { return arguments.length + \":\" + arguments[0]; }'); return g(7, 8); }
             function later() { eval('var g = function () { return typeof q; }; eval(\"var q = 1\");'); return g(); }
             function writes() { var x = 1; eval('var g = function () { x = 2; return x; }'); return g() + x; }
             function nested() { eval('var g = (function () { return function () { return String.name; }; })();'); return g(); }
             for (var i = 0; i < 3; i++) {
                 out.push(outer('A'), own(), receiver(), args(), later(), writes(), nested());
             }
             out.join();"
        ),
        Ok(Value::String(
            "LAfunction,4,Go,2:7,number,4,String,"
                .repeat(3)
                .trim_end_matches(',')
                .into()
        ))
    );
}

/// A closure created in a function that has a direct `eval` bypasses that
/// function's dynamic scope only until the scope binds a new name: a later
/// `eval('var String = 5')` must then be visible to it. A generator's frame
/// keeps the scope throughout, and a frame creating many closures stops
/// bypassing past its limit.
#[test]
fn closures_see_names_a_later_eval_adds_to_their_function() {
    assert_eq!(
        eval(
            "var out = [];
             function later() { var g = function () { return typeof String; }; var r = g(); eval('var String = 5'); return r + ' ' + g(); }
             function hoisted() { function g() { return typeof zz; } var r = g(); eval('var zz = 1'); return r + ' ' + g(); }
             function captured() { var self = 3; function d() { return self + 1; } return eval('d()') + eval('d()'); }
             function* suspended() { var g = function () { return typeof yy; }; yield g(); eval('var yy = 1'); yield g(); }
             function many() { var fns = []; for (var i = 0; i < 70; i++) fns.push(function () { return typeof Math; }); eval('var Math = 1'); return fns[0]() + ' ' + fns[69](); }
             function walkTree(filter) {
                 function walk(k, v) { var i; if (v && typeof v === 'object') { for (i in v) { var n = walk(i, v[i]); if (n !== undefined) v[i] = n; } } return filter(k, v); }
                 return JSON.stringify(walk('', eval('({a: [1, 2, {b: 3}], c: 4})')));
             }
             for (var i = 0; i < 2; i++) {
                 var it = suspended();
                 out.push(later(), hoisted(), captured(), it.next().value + ' ' + it.next().value, many(),
                          walkTree(function (k, v) { return typeof v === 'number' ? v * 10 : v; }));
             }
             out.join('|');"
        ),
        Ok(Value::String(
            "function number|undefined number|8|undefined number|number number|{\"a\":[10,20,{\"b\":30}],\"c\":40}|"
                .repeat(2)
                .trim_end_matches('|')
                .into()
        ))
    );
}

/// A bypass ends only for the closures that resolve the name a later `eval`
/// binds: an unrelated `var` leaves a closure on its path, and the name it
/// does read is still seen once declared.
#[test]
fn a_later_eval_revokes_only_the_closures_reading_its_name() {
    assert_eq!(
        eval(
            "function f() {
                 var reads = function () { return typeof String; };
                 var other = function () { return typeof Math; };
                 var out = [reads(), other()];
                 eval('var unrelated = 1');
                 out.push(reads(), other());
                 eval('var String = 5');
                 out.push(reads(), other(), unrelated);
                 return out.join(',');
             }
             f() + '|' + f();"
        ),
        Ok(Value::String(
            "function,object,function,object,number,object,1|function,object,function,object,number,object,1"
                .to_owned()
                .into()
        ))
    );
}
