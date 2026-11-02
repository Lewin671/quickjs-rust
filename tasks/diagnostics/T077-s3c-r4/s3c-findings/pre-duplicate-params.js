// PRE-EXISTING (base and candidate identical): sloppy duplicate parameter with missing argument returns the first argument instead of undefined.
function f(a, a) { return a; }
function g(a, b, a) { return [a, b]; }
function h(x) { return f(x) + "|" + f(x, 9) + "|" + g(1, 2) + "|" + g(1, 2, 3); }
var r = []; for (var i = 0; i < 30; i++) r.push(h(i)); print("A: " + r.slice(-2).join(" "));
print("B: " + f(1) + " " + f(1, 2) + " " + g(1) + " " + [1, 2].map(f).join(","));
function k(a, a) { var t = a; return t === undefined ? "U" : t; }
function caller(n) { return k(n) + k(n, n + 1); }
var s = ""; for (var j = 0; j < 20; j++) s += caller(j) + ";"; print("C: " + s);
function m(a, b, a) { return this === undefined ? a : a + 1; }
var o = { m: m }; var z = []; for (var q = 0; q < 20; q++) z.push(o.m(q) + "/" + o.m(q, 1, 2)); print("D: " + z.slice(-2).join(" "));
