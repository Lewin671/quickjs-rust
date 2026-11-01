function __S(v, d) { d = d || 0; try { if (typeof v === 'string') return JSON.stringify(v); if (typeof v === 'symbol') return 'Sym'; if (typeof v === 'function') return 'fn'; if (typeof v === 'bigint') return v + 'n'; if (v && typeof v === 'object') { if (d > 1) return '{..}'; if (Array.isArray(v)) return '[' + v.map(function (q) { return __S(q, d + 1); }).join(',') + ']'; return '{' + Object.keys(v).slice(0, 8).map(function (k) { return k + ':' + __S(v[k], d + 1); }).join(',') + '}'; } if (v === 0 && 1 / v < 0) return '-0'; return String(v); } catch (e) { return '<S>'; } }
function __o(l, f) { var r; try { r = __S(f()); } catch (e) { r = 'throws ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e)); } print(l + ': ' + r); }
function __e(l, e) { print(l + ': caught ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e))); }
var fs = [];
{
try {{
for (var iv = 0; iv < 3; iv++) {
try { y = (0, eval)("typeof y"); } catch (__x) { __e("E3", __x); }
}
}} catch (c) {
for (const k of [67, "s"]) {
}}
}
function af11(c, a) {
}
for (var iv = 0; iv < 3; iv++) {
for (let k in {p: 1, q: 2}) {
for (let i = 0; i < 3; fs.push(() => i), i++) {
}
}
{
try { with ({}) {
} } catch (__x) { __e("E21", __x); }
}
}
try { with ({x: undefined, y: 4}) {
try {{
throw 3;
}} catch ([cb]) {
for (var iv = 0; iv < 3; iv++) {
}
(function () {
function af22(a, c) {
}
})();
} finally {{
__o("L28", () => [((z = x) + fs.length), eval("[4, 80]")]);
}}
} } catch (__x) { __e("E29", __x); }
function* g29() {
(function () {
try { with ({b: "w"}) {
if ((b = (x + b))) {
}
} } catch (__x) { __e("E36", __x); }
(function () {
})();
})();
}
try { x = (`${(y ? 5 : c)}` + z); } catch (__x) { __e("E42", __x); }
try { eval("function a() { return x; }\nfor (var iv = 0; iv < 3; iv++) {\ntry { z = c; } catch (__x) { __e(\"E46\", __x); }\nfor (var kv of [7, \"q\"]) {\nfs.push(() => kv);\nfs.push(() => eval(\"a\"));\n}\n}\n__o(\"D47\", () => [delete u, typeof x]);"); } catch (__x) { __e("E48", __x); }
function af49(c) {
}
