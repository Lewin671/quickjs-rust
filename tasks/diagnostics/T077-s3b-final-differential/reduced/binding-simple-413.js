function __S(v, d) { d = d || 0; try { if (typeof v === 'string') return JSON.stringify(v); if (typeof v === 'symbol') return 'Sym'; if (typeof v === 'function') return 'fn'; if (typeof v === 'bigint') return v + 'n'; if (v && typeof v === 'object') { if (d > 1) return '{..}'; if (Array.isArray(v)) return '[' + v.map(function (q) { return __S(q, d + 1); }).join(',') + ']'; return '{' + Object.keys(v).slice(0, 8).map(function (k) { return k + ':' + __S(v[k], d + 1); }).join(',') + '}'; } if (v === 0 && 1 / v < 0) return '-0'; return String(v); } catch (e) { return '<S>'; } }
function __o(l, f) { var r; try { r = __S(f()); } catch (e) { r = 'throws ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e)); } print(l + ': ' + r); }
var fs = [];
try { with ({}) {
function f1(b) {
for (const k of [null, 7]) {
switch (2) {
}
}
return [(() => (() => y)())(), x];
}
function f6() {
fs.push(() => { y = `${(() => (x = null))()}`; return "w"; });
}
__o("C8", () => f6(typeof c));
} } catch (__x) { __e("E10", __x); }
{
function af10(b) {
}
}
try { eval("function c() { return u; }\n__o(\"F15\", () => Function(\"b\", \"return [typeof b, typeof a, arguments.length];\")(7));"); } catch (__x) { __e("E16", __x); }
const y = 4;
(async function () {
await 0;
return f1(`${c}`);
})().then((v) => print("P22" + ": " + __S(v)), (e) => print("P22" + ": rej " + (e && e.constructor && e.constructor.name)));
function af22(b, a) {
}
for (var __i = 0; __i < fs.length; __i++) { __o('fs' + __i, fs[__i]); }
