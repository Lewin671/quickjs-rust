// T077 S3c C2 attribution: captured_write shape (broad/captured_write == upw1).
// Usage: qjs call-captured-write.js N ; per-call delta C2-base is the same ~950 as
// call-unexposed.js; the captured write itself is unchanged (~314 -> ~320 per write).
function mk() { var c = 0; return function () { c += 1; return c; }; }
function run(n) { var w = mk(); var s = 0; for (var i = 0; i < n; i++) s += w(); return s; }
run(Number(scriptArgs[1] || 1000000));
