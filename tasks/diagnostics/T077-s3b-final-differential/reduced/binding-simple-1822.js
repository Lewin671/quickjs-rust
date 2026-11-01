function __S(v, d) { d = d || 0; try { if (typeof v === 'string') return JSON.stringify(v); if (typeof v === 'symbol') return 'Sym'; if (typeof v === 'function') return 'fn'; if (typeof v === 'bigint') return v + 'n'; if (v && typeof v === 'object') { if (d > 1) return '{..}'; if (Array.isArray(v)) return '[' + v.map(function (q) { return __S(q, d + 1); }).join(',') + ']'; return '{' + Object.keys(v).slice(0, 8).map(function (k) { return k + ':' + __S(v[k], d + 1); }).join(',') + '}'; } if (v === 0 && 1 / v < 0) return '-0'; return String(v); } catch (e) { return '<S>'; } }
function __o(l, f) { var r; try { r = __S(f()); } catch (e) { r = 'throws ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e)); } print(l + ': ' + r); }
var fs = [];
function f0() {
try {{
}} catch (c) {
} finally {{
}}
return [a, 4];
}
function* g9() {
}
try {{
throw "w";
}} catch (a) {
{
{
try { eval("function a() { return f0(x); }\ntry { class K extends Object {  m() { return [true, typeof K]; } static s = eval(\"f0(eval(\\\"\\\\\\\"q\\\\\\\"\\\"))\"); f = () => this.#p; #p = (c = typeof y); gp() { return this.#p; } static { fs.push(() => [typeof K, typeof y]); } }\n__o(\"K21\", () => { var o = new K(); return [o.m(), K.s, typeof o.f === \"function\" ? o.f() : o.f, o.gp(), o.cv]; }); } catch (__x) { __e(\"E22\", __x); }\nfunction f22(b, c) {\n__o(\"L24\", () => u);\nreturn [(() => (y = eval(\"x\")))(), b];\n}\n__o(\"C25\", () => f22());"); } catch (__x) { __e("E26", __x); }
}
}
for (const k of [54, 41]) {
}}
function f33() {
(async function () {
})().then((v) => print("P37" + ": " + __S(v)), (e) => print("P37" + ": rej " + (e && e.constructor && e.constructor.name)));
}
if ((a ? fs.length : (c = c))) {
function* g41(b, a) {
}
if ((x + 9)) {
try { var c = typeof z; } catch (__x) { __e("E51", __x); }
function af51(a) {
}
}
}
var a = undefined;
