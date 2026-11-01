// Closure with 4 captured-variable writes called from a wide-tier loop: per-call
// ActivationContext construction (+~480 ins/call) plus checked BindingCell::assign.
function mk() { var c = 0; return function () {  c += 1; c += 1; c += 1; c += 1; return c; }; }
function run(n) { var w = mk(); var s = 0; for (var i = 0; i < n; i++) s += w(); return s; }
run(1000000);
