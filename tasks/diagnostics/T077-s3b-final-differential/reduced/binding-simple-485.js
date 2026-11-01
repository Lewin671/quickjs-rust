function __S(v, d) { d = d || 0; try { if (typeof v === 'string') return JSON.stringify(v); if (typeof v === 'symbol') return 'Sym'; if (typeof v === 'function') return 'fn'; if (typeof v === 'bigint') return v + 'n'; if (v && typeof v === 'object') { if (d > 1) return '{..}'; if (Array.isArray(v)) return '[' + v.map(function (q) { return __S(q, d + 1); }).join(',') + ']'; return '{' + Object.keys(v).slice(0, 8).map(function (k) { return k + ':' + __S(v[k], d + 1); }).join(',') + '}'; } if (v === 0 && 1 / v < 0) return '-0'; return String(v); } catch (e) { return '<S>'; } }
function __o(l, f) { var r; try { r = __S(f()); } catch (e) { r = 'throws ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e)); } print(l + ': ' + r); }
function __e(l, e) { print(l + ': caught ' + (e && typeof e === 'object' && e.constructor ? e.constructor.name : __S(e))); }
var fs = [];
for (const k of [null, 75]) {
try {{
throw "s";
}} catch (b) {
try { eval("function b() { return u; }\nfunction* g4() {\nfunction f5(b) {\n__o(\"L7\", () => b);\nreturn [u, (() => (function () { return arguments[0]; })())()];\n}\n__o(\"C8\", () => f5(fs.length));\nyield u;\ntry { with (__px({b: 37, c: \"w\"})) {\n__o(\"L9\", () => x--);\n} } catch (__x) { __e(\"E10\", __x); }\nyield 63;\n}\nvar it10 = g4(null);\n__o(\"G11\", () => it10.next(null).value);\ntry { b ??= y; } catch (__x) { __e(\"E12\", __x); }\n__o(\"G13\", () => it10.next(0).value);\ntry { z = f5((fs.length + 54)); } catch (__x) { __e(\"E14\", __x); }\n__o(\"G15\", () => it10.next(5).value);\ntry { u = b; } catch (__x) { __e(\"E16\", __x); }\nfor (let i = 0; i < 3; fs.push(() => i), i++) {\n__o(\"L17\", () => x--);\n}"); } catch (__x) { __e("E18", __x); }
function* g18(b) {
}}
}
{
function* g27(c) {
for (let i = 0; i < 3; i++) {
}
}
try { with (__px({})) {
} } catch (__x) { __e("E37", __x); }
}
