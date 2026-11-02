// T077 S3c C2 attribution: per-call cost of an unexposed inline call on the wide tier.
// Usage: qjs call-unexposed.js N  (run N and 2N; (I(2N)-I(N))/N = instructions per call)
// base 8322fc98: 1003/call, C2 63b2347f: 1941/call (arm64, release fat LTO).
function mk() { var c = 0; return function () { return c; }; }
function run(n) { var w = mk(); var s = 0; for (var i = 0; i < n; i++) s += w(); return s; }
run(Number(scriptArgs[1] || 1000000));
