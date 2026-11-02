// PRE-EXISTING (base e60e7cfa and candidate identical; NG differs): outer finally skipped on continue.
// Expected (NG/spec) a0,b0,a1,b1 ; base & candidate print a0,b0,a1
var log = [];
for (var i = 0; i < 2; i++) { try { try { if (i) continue; } finally { log.push("a" + i); } } finally { log.push("b" + i); } }
print(log.join(","));
