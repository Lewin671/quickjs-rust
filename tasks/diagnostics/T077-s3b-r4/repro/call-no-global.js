// Control: callee writes nothing global.
function put(x) { return x; }
var s = 0;
for (var i = 0; i < 1000000; i++) s += put(i);
if (s !== 499999500000) throw new Error("bad");
