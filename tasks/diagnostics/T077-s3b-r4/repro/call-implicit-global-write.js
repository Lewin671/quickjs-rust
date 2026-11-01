// Callee writes an implicit (configurable) global.
H = 0;
function put(x) { H = x; return x; }
var s = 0;
for (var i = 0; i < 1000000; i++) s += put(i);
if (H !== 999999) throw new Error("bad");
