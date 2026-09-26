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
