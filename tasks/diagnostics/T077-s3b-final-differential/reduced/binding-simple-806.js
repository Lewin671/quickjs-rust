function __S(v, d) { d = d || 0; try { if (typeof v === 'string') return JSON.stringify(v); if (typeof v === 'symbol') return 'Sym'; if (typeof v === 'function') return 'fn'; if (typeof v === 'bigint') return v + 'n'; if (v && typeof v === 'object') { if (d > 1) return '{..}'; if (Array.isArray(v)) return '[' + v.map(function (q) { return __S(q, d + 1); }).join(',') + ']'; return '{' + Object.keys(v).slice(0, 8).map(function (k) { return k + ':' + __S(v[k], d + 1); }).join(',') + '}'; } if (v === 0 && 1 / v < 0) return '-0'; return String(v); } catch (e) { return '<S>'; } }
function __o(l, f) { var r; try { r = __S(f()); } catch (e) { r = 'throws ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e)); } print(l + ': ' + r); }
var fs = [];
function f0(b, c) {
}
fs.push(() => u);
try { with (Object.create({u: 23, a: "w"})) {
fs.push(() => { y = b; return "w"; });
const y = (function () { try { return (0, eval)("b"); } catch (__q) { return "init-" + __q.constructor.name; } })();
} } catch (__x) { __e("E20", __x); }
for (let i = 0; i < 3; i++) {
for (let i = 0; i < 3; fs.push(() => i), i++) {
}
}
for (var __i = 0; __i < fs.length; __i++) { __o('fs' + __i, fs[__i]); }
