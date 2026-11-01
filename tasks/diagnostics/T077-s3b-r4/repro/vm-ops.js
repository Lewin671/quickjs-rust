// Top-level loop that no accelerator admits (object literal allocation);
// the general VM executes every op in both builds.
var s = 0;
for (var i = 0; i < 1000000; i++) { var o = { a: i }; s += o.a; }
if (s !== 499999500000) throw new Error("bad");
