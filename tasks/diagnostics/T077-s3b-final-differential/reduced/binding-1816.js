function __S(v, d) { d = d || 0; try { if (typeof v === 'string') return JSON.stringify(v); if (typeof v === 'symbol') return 'Sym'; if (typeof v === 'function') return 'fn'; if (typeof v === 'bigint') return v + 'n'; if (v && typeof v === 'object') { if (d > 1) return '{..}'; if (Array.isArray(v)) return '[' + v.map(function (q) { return __S(q, d + 1); }).join(',') + ']'; return '{' + Object.keys(v).slice(0, 8).map(function (k) { return k + ':' + __S(v[k], d + 1); }).join(',') + '}'; } if (v === 0 && 1 / v < 0) return '-0'; return String(v); } catch (e) { return '<S>'; } }
function __o(l, f) { var r; try { r = __S(f()); } catch (e) { r = 'throws ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e)); } print(l + ': ' + r); }
var fs = [];
switch (1) {
}
function f7() {
for (let k of [6, 68]) {
for (const k in {p: 1, q: 2}) {
try { x = [(a = (6 + 61)), (`${5}` + u)]; } catch (__x) { __e("E17", __x); }
}
}
return [b--, arguments[0]];
}
try {{
function* g19() {
for (let i = 0; i < 3; i++) {
}
function* g24(c, ...b) {
try { with (Object.create({u: undefined})) {
function* g25(a, c) {
}
} } catch (__x) { __e("E35", __x); }
(async function () {
try { class K extends (class { m2() { return (z = (function () { return (c + x); })()); } }) {  m() { return [x, typeof K]; } static s = eval("(() => (function () { return b; })())()"); f = () => this.#p; #p = x--; gp() { return this.#p; } static { fs.push(() => [typeof K, eval("fs.length")]); } }
__o("K37", () => { var o = new K(); return [o.m(), K.s, typeof o.f === "function" ? o.f() : o.f, o.gp(), o.cv]; }); } catch (__x) { __e("E38", __x); }
})().then((v) => print("P36" + ": " + __S(v)), (e) => print("P36" + ": rej " + (e && e.constructor && e.constructor.name)));
}
}
throw 4;
}} catch (c) {
try {{
for (let i = 0; i < 3; i++) {
try { eval("function c() { return typeof b; }\nconst y = (function () { try { return \"w\"; } catch (__q) { return \"init-\" + __q.constructor.name; } })();"); } catch (__x) { __e("E53", __x); }
}
try { class K {  m() { return [[(0, eval)("x"), `${u++}`], typeof K]; } static s = (typeof a + eval("b")); f = x; #p = typeof x; gp() { return this.#p; } static { fs.push(() => [typeof K, typeof b]); } }
__o("K54", () => { var o = new K(); return [o.m(), K.s, typeof o.f === "function" ? o.f() : o.f, o.gp(), o.cv]; }); } catch (__x) { __e("E55", __x); }
}} catch (a) {
try { eval("function u() { return x; }\ntry { fs.push(eval(\"(function () { return f7(5); })\")); } catch (__x) { __e(\"E56\", __x); }"); } catch (__x) { __e("E57", __x); }
{
function f57(b, c) {
switch (2) {
}
return [fs.length, arguments[0]];
}
}
} finally {{
}}
try { eval("var b = undefined;\ntry { u ||= 5; } catch (__x) { __e(\"E63\", __x); }\n__o(\"L64\", () => eval(\"3\"));"); } catch (__x) { __e("E65", __x); }
} finally {{
}}
try { with ((function () { var o = {z: 25}; o[Symbol.unscopables] = {z: true}; return o; })()) {
try { class K extends Object {  m() { return [("s" ? `${typeof x}` : eval("[66, 33]")), typeof K]; } static s = f7(f57(u)); f = eval("c"); #p = (c + "q"); gp() { return this.#p; } static { fs.push(() => [typeof K, ((fs.length ? eval("b") : b--) + fs.length)]); } }
__o("K67", () => { var o = new K(); return [o.m(), K.s, typeof o.f === "function" ? o.f() : o.f, o.gp(), o.cv]; }); } catch (__x) { __e("E68", __x); }
try { with ({z: 0}) {
function* g71() {
function af72(c) {
}
}
} } catch (__x) { __e("E85", __x); }
}} catch ([cb]) {
} finally {{
}}
{
}
