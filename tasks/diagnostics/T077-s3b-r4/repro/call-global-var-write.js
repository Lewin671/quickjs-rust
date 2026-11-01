// Callee writes a declared (non-configurable) global var: the reference site is
// "stable", but stability is re-proved by name at every call entry.
var G = 0;
function put(x) { G = x; return x; }
var s = 0;
for (var i = 0; i < 1000000; i++) s += put(i);
if (G !== 999999) throw new Error("bad");
