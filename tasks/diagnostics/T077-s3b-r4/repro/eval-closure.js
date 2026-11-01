// A closure created by direct eval inside a function (scope chain contains a
// sloppy-eval var holder) reads a global and a parameter on each call.
function make() { var k = 1; eval("var f = function (x) { return Math.abs(x) + k; }"); return f; }
var f = make();
var s = 0;
for (var i = 0; i < 300000; i++) s += f(i);
if (s !== 44999850000 + 300000) throw new Error("bad " + s);
