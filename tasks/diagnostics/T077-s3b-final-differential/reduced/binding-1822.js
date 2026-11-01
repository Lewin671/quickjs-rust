function __S(v, d) { d = d || 0; try { if (typeof v === 'string') return JSON.stringify(v); if (typeof v === 'symbol') return 'Sym'; if (typeof v === 'function') return 'fn'; if (typeof v === 'bigint') return v + 'n'; if (v && typeof v === 'object') { if (d > 1) return '{..}'; if (Array.isArray(v)) return '[' + v.map(function (q) { return __S(q, d + 1); }).join(',') + ']'; return '{' + Object.keys(v).slice(0, 8).map(function (k) { return k + ':' + __S(v[k], d + 1); }).join(',') + '}'; } if (v === 0 && 1 / v < 0) return '-0'; return String(v); } catch (e) { return '<S>'; } }
function __o(l, f) { var r; try { r = __S(f()); } catch (e) { r = 'throws ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e)); } print(l + ': ' + r); }
function __e(l, e) { print(l + ': caught ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e))); }
var fs = [];
function f0() {
try {{
}} catch (c) {
} finally {{
}}
try { class Q extends Object { constructor() { super(); this.cv = eval("typeof z"); } m() { return [b, typeof Q]; } static s = (typeof b + typeof y); f = u; #p = (fs.length + typeof c); gp() { return this.#p; } static { fs.push(() => [typeof Q, 70]); } }
__o("K5", () => { var o = new Q(); return [o.m(), Q.s, typeof o.f === "function" ? o.f() : o.f, o.gp(), o.cv]; }); } catch (__x) { __e("E6", __x); }
return [a, 4];
}
function* g9() {
}
try {{
try { fs.push(eval("(function () { return 57; })")); } catch (__x) { __e("E19", __x); }
throw "w";
}} catch (a) {
{
{
try { eval("function a() { return f0(x); }\ntry { class K extends Object {  m() { return [true, typeof K]; } static s = eval(\"f0(eval(\\\"\\\\\\\"q\\\\\\\"\\\"))\"); f = () => this.#p; #p = (c = typeof y); gp() { return this.#p; } static { fs.push(() => [typeof K, typeof y]); } }\n__o(\"K21\", () => { var o = new K(); return [o.m(), K.s, typeof o.f === \"function\" ? o.f() : o.f, o.gp(), o.cv]; }); } catch (__x) { __e(\"E22\", __x); }\nfunction f22(b, c) {\n__o(\"L24\", () => u);\nreturn [(() => (y = eval(\"x\")))(), b];\n}\n__o(\"C25\", () => f22());"); } catch (__x) { __e("E26", __x); }
}
}
for (const k of [54, 41]) {
fs.push(() => k);
try { class Q {  m() { return [[(f26(a) ? `${undefined}` : typeof y), typeof u], typeof Q]; } static s = (0, eval)("typeof a"); f = ((c = (() => "s")()) + typeof z); #p = typeof y; gp() { return this.#p; } static { fs.push(() => [typeof Q, b]); } }
__o("K31", () => { var o = new Q(); return [o.m(), Q.s, typeof o.f === "function" ? o.f() : o.f, o.gp(), o.cv]; }); } catch (__x) { __e("E32", __x); }
}}
function f33() {
}
for (var __i = 0; __i < fs.length; __i++) { __o('fs' + __i, fs[__i]); }
