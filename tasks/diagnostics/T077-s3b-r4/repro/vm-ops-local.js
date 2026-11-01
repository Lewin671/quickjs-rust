// Function-local loop on the general VM (direct eval keeps it off the compact tiers);
// no globals are touched inside the loop.
function f(n) { eval(""); var s = 0; for (var i = 0; i < n; i++) { var o = { a: i }; s += o.a; } return s; }
if (f(1000000) !== 499999500000) throw new Error("bad");
