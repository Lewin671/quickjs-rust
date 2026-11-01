// Counted loop calling a two-argument numeric leaf; the typed loop answers each
// call with the closed-form leaf evaluator in both builds.
function addTwo(v, inc) { return v + inc; }
function run(n) { var s = 0; for (var i = 0; i < n; i++) s += addTwo(i, 1); return s; }
if (run(2000000) !== 2000001000000) throw new Error("bad");
