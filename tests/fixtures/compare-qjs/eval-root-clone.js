(function () { function run(n) { var s = 0; for (var i = 0; i < n; i++) s += eval("var x = i; x + 1"); return s; } return run(100); })()
