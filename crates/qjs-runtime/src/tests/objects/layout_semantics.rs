//! Ordinary property semantics across layout transitions: edges, attribute
//! edits, dictionaries, payload re-encodings, literal sites and realms.

use crate::{Value, eval};

#[test]
fn layout_semantics_survive_edges_dictionaries_and_reencoding() {
    let source = r#"
        var out = [];
        function keys(o) { return Object.keys(o).join(','); }
        // Delete and re-add moves the key to the end.
        var a = { x: 1, y: 2, z: 3 }; delete a.y; a.y = 4; out.push(keys(a));
        // Integer keys enumerate first, ascending.
        var b = { b: 1, 2: 1, a: 1, 1: 1 }; out.push(keys(b));
        // Equal keys with different attributes take different edges.
        var c1 = {}; c1.k = 1;
        var c2 = {}; Object.defineProperty(c2, 'k', { value: 1, enumerable: false, writable: true, configurable: true });
        var c3 = {}; c3.k = 2;
        out.push(keys(c1) + '|' + keys(c2) + '|' + keys(c3) + '|' + c2.k);
        // More than 32 edits and more than 128 properties.
        var d = {};
        for (var i = 0; i < 40; i++) { d['p' + i] = i; delete d['p' + i]; d['p' + i] = i * 2; }
        out.push(Object.keys(d).length + ':' + d.p39);
        var e = {};
        for (var i = 0; i < 150; i++) e['q' + i] = i;
        delete e.q0; e.q0 = 'last';
        var ek = Object.keys(e);
        out.push(ek.length + ':' + ek[0] + ':' + ek[149] + ':' + e.q75);
        // Freezing one literal instance leaves its siblings writable.
        function lit() { return { m: 1, n: 2 }; }
        var f1 = Object.freeze(lit()); var f2 = lit(); f2.m = 9; f1.m = 9;
        out.push(f1.m + ':' + f2.m + ':' + Object.isFrozen(f1) + ':' + Object.isFrozen(f2));
        // Class-field initializers that delete and observe the instance.
        class G { a = 1; b = (delete this.a, Object.keys(this).join('+') + '!'); c = 3; }
        var g = new G(); out.push(keys(g) + '=' + g.b);
        // A base constructor returning another object.
        class H { constructor() { return { other: true }; } }
        class K extends H { f = 1; }
        var k = new K(); out.push(keys(k));
        // Null-prototype objects and accessors.
        var n = Object.create(null); n.a = 1;
        Object.defineProperty(n, 'g', { get: function () { return this.a + 1; }, enumerable: true, configurable: true });
        out.push(keys(n) + ':' + n.g);
        // Lone-surrogate and halves-built keys name the same property.
        var s = {}; s['\uD800'] = 1; s['\uD83D' + '\uDE00'] = 2;
        out.push(s['\uD800'] + ':' + s['😀'] + ':' + Object.keys(s).length);
        // Literals across eval keep independent instances.
        var ev = eval('({ p: 1, q: 2 })'); ev.r = 3; out.push(keys(ev) + ':' + keys(eval('({ p: 1, q: 2 })')));
        out.join(' ; ');
    "#;
    let value = eval(source).expect("semantics script");
    let expected = "x,z,y ; 1,2,b,a ; k||k|1 ; 40:78 ; 150:q1:q0:75 ; 1:9:true:false ; \
                    b,c=! ; other,f ; a,g:2 ; 1:2:2 ; p,q,r:p,q";
    assert_eq!(value, Value::String(expected.into()));
}

#[test]
fn replayed_adds_respect_prototype_extensibility_and_layout_changes() {
    let source = r#"
        var out = [];
        function keys(o) { return Object.keys(o).join(','); }
        function P(v) { this.a = v; this.b = v; }
        var before = [new P(1), new P(2), new P(3)].map(keys).join('/');
        // A setter added to the prototype afterwards intercepts the add.
        var seen = [];
        Object.defineProperty(P.prototype, 'b', { set: function (v) { seen.push(v); }, configurable: true });
        var p = new P(4);
        out.push(before + '|' + keys(p) + '|' + seen.join(','));
        delete P.prototype.b;
        out.push(keys(new P(5)) + ':' + new P(6).b);
        // A receiver made non-extensible, or on another layout, misses.
        function Q(lock, extra) {
            if (lock) Object.preventExtensions(this);
            if (extra) this.z = 0;
            this.a = 1;
        }
        new Q(false, false); new Q(false, false);
        out.push(keys(new Q(true, false)) + '|' + keys(new Q(false, true)) + '|' + keys(new Q(false, false)));
        // Class fields: an initializer that adds a later field's key first,
        // and a prototype setter that a field definition must not call.
        class R { a = (this.b = 5, 1); b = 2; }
        var r = [new R(), new R()].map(function (o) { return keys(o) + '=' + o.b; }).join('/');
        class S { x = 1; y = 2; }
        new S();
        Object.defineProperty(S.prototype, 'x', { set: function () { throw new Error('called'); } });
        var s = new S();
        out.push(r + '|' + keys(s) + ':' + s.x);
        out.join(' ; ');
    "#;
    let value = eval(source).expect("replay semantics script");
    let expected = "a,b/a,b/a,b|a|4 ; a,b:6 ; |z,a|a ; b,a=2/b,a=2|x,y:1";
    assert_eq!(value, Value::String(expected.into()));
}
