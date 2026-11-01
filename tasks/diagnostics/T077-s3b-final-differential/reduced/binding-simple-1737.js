function __S(v, d) { d = d || 0; try { if (typeof v === 'string') return JSON.stringify(v); if (typeof v === 'symbol') return 'Sym'; if (typeof v === 'function') return 'fn'; if (typeof v === 'bigint') return v + 'n'; if (v && typeof v === 'object') { if (d > 1) return '{..}'; if (Array.isArray(v)) return '[' + v.map(function (q) { return __S(q, d + 1); }).join(',') + ']'; return '{' + Object.keys(v).slice(0, 8).map(function (k) { return k + ':' + __S(v[k], d + 1); }).join(',') + '}'; } if (v === 0 && 1 / v < 0) return '-0'; return String(v); } catch (e) { return '<S>'; } }
function __o(l, f) { var r; try { r = __S(f()); } catch (e) { r = 'throws ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e)); } print(l + ': ' + r); }
var fs = [];
for (let i = 0; i < 3; i++) {
}
(async function () {
try { with ({z: 52}) {
function* g2() {
switch (0) {
case 0: let z = 39;
fs.push(() => z);
}
}
var it6 = g2(null);
__o("G11", () => it6.next(true).value);
} } catch (__x) { __e("E13", __x); }
})().then((v) => print("P2" + ": " + __S(v)), (e) => print("P2" + ": rej " + (e && e.constructor && e.constructor.name)));
if (typeof x) {
for (var iv = 0; iv < 3; iv++) {
fs.push(function () { return ((true + (function () { return x; })()) + (14 + ["q", z])); });
}
for (let i = 0; i < 3; i++) {
try { with ({}) {
} } catch (__x) { __e("E20", __x); }
for (let k of ["w", 0]) {
function af20(b, c) {
}
}
}
}
for (const k of ["w", 9]) {
fs.push(() => k);
fs.push(function () { return 1; });
(() => {
fs.push(() => eval("undefined"));
try { fs.push(eval("(function () { return (x = (a = delete y)); })")); } catch (__x) { __e("E25", __x); }
})();
}
try { with ({}) {
} } catch (__x) { __e("E36", __x); }
try { fs.push(eval("(function () { return (a = (9 + [z, z])); })")); } catch (__x) { __e("E37", __x); }
fs.push(() => { u = typeof u; return "w"; });
try { with (Object.create({c: true})) {
fs.push(() => (0, eval)("u"));
function* g38() {
try { class Q {  m() { return [(() => (u = typeof z))(), typeof Q]; } static s = (function () { return typeof x; })(); f = eval("y"); #p = (typeof a + (function () { return u; })()); gp() { return this.#p; } static { fs.push(() => [typeof Q, (typeof c ? (c = c) : (function () { return x; })())]); } }
__o("K40", () => { var o = new Q(); return [o.m(), Q.s, typeof o.f === "function" ? o.f() : o.f, o.gp(), o.cv]; }); } catch (__x) { __e("E41", __x); }
try { with (Object.create({b: "w"})) {
for (let i = 0; i < 3; i++) {
try {{
}} catch ([cb]) {
} finally {{
}}
}
} } catch (__x) { __e("E45", __x); }
}
var it45 = g38(91);
__o("G46", () => it45.next(true).value);
} } catch (__x) { __e("E52", __x); }
for (var __i = 0; __i < fs.length; __i++) { __o('fs' + __i, fs[__i]); }
