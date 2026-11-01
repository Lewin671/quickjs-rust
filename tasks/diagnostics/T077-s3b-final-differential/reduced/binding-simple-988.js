function __S(v, d) { d = d || 0; try { if (typeof v === 'string') return JSON.stringify(v); if (typeof v === 'symbol') return 'Sym'; if (typeof v === 'function') return 'fn'; if (typeof v === 'bigint') return v + 'n'; if (v && typeof v === 'object') { if (d > 1) return '{..}'; if (Array.isArray(v)) return '[' + v.map(function (q) { return __S(q, d + 1); }).join(',') + ']'; return '{' + Object.keys(v).slice(0, 8).map(function (k) { return k + ':' + __S(v[k], d + 1); }).join(',') + '}'; } if (v === 0 && 1 / v < 0) return '-0'; return String(v); } catch (e) { return '<S>'; } }
function __o(l, f) { var r; try { r = __S(f()); } catch (e) { r = 'throws ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e)); } print(l + ': ' + r); }
function __e(l, e) { print(l + ': caught ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e))); }
var __log = [];
function __px(t) { return new Proxy(t, { has: function (o, k) { if (typeof k === 'string') __log.push('h' + k); return k in o; }, get: function (o, k, r) { if (typeof k === 'string') __log.push('g' + k); return Reflect.get(o, k, r); }, set: function (o, k, v, r) { if (typeof k === 'string') __log.push('s' + k); return Reflect.set(o, k, v, r); } }); }
try { with ({}) {
switch (1) {
}
} } catch (__x) { __e("E13", __x); }
function* g13(a) {
for (var kv of ["q", 3]) {
}
}
try {{
for (let k of ["w", undefined]) {
}
}
try { class Q {  m() { return [x, typeof Q]; } static s = [(`${2}` + b), fs.length]; f = () => this.#p; #p = fs.length; gp() { return this.#p; } static { fs.push(() => [typeof Q, f8(f8((function () { return null; })()))]); } }
__o("K42", () => { var o = new Q(); return [o.m(), Q.s, typeof o.f === "function" ? o.f() : o.f, o.gp(), o.cv]; }); } catch (__x) { __e("E43", __x); }
} finally {{
}}
try {{
if (fs.length) {
}
}} catch (b) {
function* g47(b, a) {
}
try { with (__px({c: 4, b: null})) {
try { eval("function a() { return (eval(\"y\") ? [(b = u), typeof b] : z); }\nif (a) {\ntry {{\n__o(\"L66\", () => typeof a);\nthrow undefined;\n}} catch ({a: ca}) {\n__o(\"L67\", () => y);\n} finally {{\n__o(\"L68\", () => \"w\");\n}}\n} else {\n__o(\"L69\", () => (fs.length + b));\n}\nfs.push(() => { b = (function () { return fs.length; })(); return \"w\"; });"); } catch (__x) { __e("E70", __x); }
} } catch (__x) { __e("E71", __x); }
} finally {{
}}
try { with ({x: true, y: 20}) {
function f76(c, b) {
}
} } catch (__x) { __e("E81", __x); }
if ((0, eval)("typeof y")) {
for (const k of [3, null]) {
(function () {
})();
for (let i = 0; i < 3; i++) {
}
}
}
