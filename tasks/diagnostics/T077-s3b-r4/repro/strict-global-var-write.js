// Strict callee writes a declared global var: the bound start is elided and
// the store takes the realm var cell (own writable data property) fast path.
var G = 0;
function put(x) { "use strict"; G = x; return x; }
var s = 0;
for (var i = 0; i < 1000000; i++) s += put(i);
if (G !== 999999) throw new Error("bad");
