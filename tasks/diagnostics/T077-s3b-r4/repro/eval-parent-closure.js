// A closure nested (lexically) in a function that contains a sloppy direct eval.
function make() { var k = 1; eval(""); return function (x) { return Math.abs(x) + k; }; }
var f = make();
var s = 0;
for (var i = 0; i < 300000; i++) s += f(i);
if (s !== 44999850000 + 300000) throw new Error("bad " + s);
