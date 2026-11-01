// Function-local loop on the general VM without eval or globals in the loop.
function f(n) { var s = 0; for (var i = 0; i < n; i++) { var o = { a: i }; s += o.a; } return s; }
if (f(1000000) !== 499999500000) throw new Error("bad");
