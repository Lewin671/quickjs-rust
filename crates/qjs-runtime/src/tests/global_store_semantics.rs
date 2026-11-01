//! Every tier's global store is GlobalEnv.SetMutableBinding (ES2025
//! 9.1.1.4.5): the declarative record first, checked at store time, then the
//! object record's HasProperty over the global object's whole prototype chain
//! and `[[Set]]`, which runs inherited setters and honours inherited
//! read-only properties. Each case runs once and in a function hot enough for
//! the register tier; a bound start elided before its RHS must store exactly
//! like a reference selected there.
use crate::{Value, eval};

const HELPERS: &str = "var ES = __quickjsRustEvalScript; \
    function hot(f) { for (var i = 0; i <= 50; i++) f(i, i === 50); }";

fn with_helpers(body: &str, expected: &str) {
    let source = format!("{HELPERS} {body}");
    assert_eq!(
        eval(&source),
        Ok(Value::String(expected.into())),
        "{source}"
    );
}

#[test]
fn strict_stores_consult_global_lexicals_declared_after_compilation() {
    // The RHS declares the lexical: the bound reference writes it, not the
    // property the start found.
    with_helpers(
        r#"globalThis.sg = 5;
           (function () { "use strict"; sg = (ES('let sg = 13;'), 34); })();
           globalThis.sg + ':' + sg"#,
        "5:34",
    );
    with_helpers(
        r#"globalThis.sh = 5;
           hot(function (i, last) { "use strict"; sh = (last ? ES('let sh = 0;') : 0, i); });
           globalThis.sh + ':' + sh"#,
        "49:50",
    );
    // A later script's lexical redirects every later strict store of a body
    // that already wrote the property.
    with_helpers(
        r#"globalThis.gq = 5; function g(v) { "use strict"; gq = v; }
           hot(g); ES('let gq = 13;'); g(7); globalThis.gq + ':' + gq"#,
        "50:7",
    );
}

#[test]
fn eval_and_function_bodies_write_script_lexicals() {
    with_helpers(
        r#"ES('let z = 4; var f = (0, eval)(\'"use strict"; (() => { z = 7; return z; })\'); \
              var seen = f(); var zr = [z, seen]; \
              eval(\'"use strict"; (function () { z = 9; })()\'); zr.push(z);');
           zr.join(':')"#,
        "7:7:9",
    );
    with_helpers(
        r#"ES('let x = 0; var inc = eval(\'"use strict"; () => { x++; }\'); inc(); inc(); inc(); \
              (0, eval)(\'"use strict"; (function () { x += 10; })()\');');
           String(x)"#,
        "13",
    );
    with_helpers(
        r#"ES('let y = 4; new Function(\'"use strict"; y = 7;\')();'); String(y)"#,
        "7",
    );
    with_helpers(
        r#"ES('let w = 0; var hw = (0, eval)(\'"use strict"; (function (i) { w = i; })\'); hot(hw); \
              let v = 0; var hv = new Function("i", \'"use strict"; v += i;\'); hot(hv);');
           w + ':' + v"#,
        "50:1275",
    );
}

#[test]
fn compound_stores_reach_inherited_setters_and_read_only_properties() {
    with_helpers(
        r#"var log = [];
           Object.defineProperty(Object.prototype, "acc", { get() { return 10; }, set(v) { log.push("set:" + v); }, configurable: true });
           Object.defineProperty(Object.prototype, "nw", { value: 5, writable: false, configurable: true });
           Object.prototype.cnt = 0;
           function own(n) { return Object.prototype.hasOwnProperty.call(globalThis, n) ? "own=" + globalThis[n] : "none"; }
           function a() { acc += 1; return own("acc"); }
           function b() { acc++; return own("acc"); }
           function c() { nw += 1; return own("nw"); }
           function d() { "use strict"; acc ||= 1; acc += 1; return own("acc"); }
           function s1() { "use strict"; toString += ""; return typeof globalThis.toString; }
           function s2() { "use strict"; cnt++; return globalThis.cnt; }
           function s3() { "use strict"; nw += 1; }
           var r = [];
           for (var f of [a, b, c, d, s1, s2, s3]) { try { r.push(f.name + ":" + f()); } catch (e) { r.push(f.name + ":!" + e.name); } }
           try { acc += 2; r.push("top:" + own("acc")); } catch (e) { r.push("top:!" + e.name); }
           var n = log.length; hot(function () { acc += 1; acc++; }); r.push("hot:" + own("acc") + ":" + (log.length - n));
           var t = 0; hot(function () { "use strict"; try { nw += 1; } catch (e) { if (e instanceof TypeError) t++; } });
           r.push("hotnw:" + t + ":" + own("nw"));
           r.join(" ") + " | " + log.slice(0, 5).join(" ")"#,
        "a:none b:none c:none d:none s1:string s2:1 s3:!TypeError top:none hot:none:102 \
         hotnw:51:none | set:11 set:11 set:11 set:12 set:11",
    );
}

#[test]
fn rhs_deletion_exposes_inherited_properties_to_a_bound_store() {
    with_helpers(
        r#"var log = [];
           Object.defineProperty(Object.prototype, "k", { set(v) { log.push("set:" + v); }, configurable: true });
           Object.defineProperty(Object.prototype, "ro", { value: "proto", writable: false, configurable: true });
           function mk(n) { Object.defineProperty(globalThis, n, { value: 1, writable: true, configurable: true }); }
           function own(n) { return Object.prototype.hasOwnProperty.call(globalThis, n) ? "own=" + globalThis[n] : "none"; }
           function sloppy(i) { mk("k"); k = (delete globalThis.k, i); return own("k"); }
           function strictRO(i) { "use strict"; mk("ro"); ro = (delete globalThis.ro, i); return own("ro"); }
           var r = [sloppy(2)];
           try { r.push(strictRO(3)); } catch (e) { r.push(e.name); }
           var errs = 0;
           hot(function (i) { sloppy(i); try { strictRO(i); } catch (e) { if (e instanceof TypeError) errs++; } });
           r.push(own("k"), own("ro"), errs, log.length, log[log.length - 1]);
           mk("k"); k = (delete globalThis.k, 4); r.push(own("k"), log[log.length - 1]);
           r.join(" ")"#,
        "none TypeError none none 51 52 set:50 none set:4",
    );
}

#[test]
fn proxy_in_the_global_prototype_chain_observes_has_and_set() {
    // SetMutableBinding always calls HasProperty, then [[Set]]; strict code
    // throws ReferenceError when the binding no longer exists.
    with_helpers(
        r#"var log = [];
           var proto = Object.getPrototypeOf(globalThis);
           Object.setPrototypeOf(globalThis, new Proxy(proto, {
             has(t, key) { if (key === "pk") log.push("has"); return Reflect.has(t, key); },
             set(t, key, v, rcv) { if (key === "pk") { log.push("set:" + v); return true; } return Reflect.set(t, key, v, rcv); },
           }));
           function mk() { Object.defineProperty(globalThis, "pk", { value: 1, writable: true, configurable: true }); }
           function own() { return Object.prototype.hasOwnProperty.call(globalThis, "pk"); }
           function f(i) { mk(); pk = (delete globalThis.pk, i); return own(); }
           function g(i) { "use strict"; mk(); try { pk = (delete globalThis.pk, i); return "nothrow"; } catch (e) { return e.name; } }
           var r = [f(2), g(3), log.join(",")];
           log.length = 0;
           hot(function (i, last) { if (last) log.length = 0; r[3] = f(i); r[4] = g(i); });
           r.push(log.join(","));
           r.join(" ")"#,
        "false ReferenceError has,set:2,has false ReferenceError has,set:50,has",
    );
}

#[test]
fn appending_to_a_global_lexical_leaves_a_shadowed_property_alone() {
    with_helpers(
        r#"globalThis.s = "p"; ES('let s = "L";');
           function f() { s += "x"; } f(); f();
           hot(function () { s += "y"; });
           globalThis.s + ":" + s.length + ":" + (0, eval)("s").slice(0, 4)"#,
        "p:54:Lxxy",
    );
}
