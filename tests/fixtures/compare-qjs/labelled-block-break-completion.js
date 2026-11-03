(function(){var r=[];
r.push(eval("for (var o of [1, 2]) { B: { 4209; try { break; } catch { } finally { } } }"));
r.push(eval("var w = 0; while (w++ < 2) { B: { try { 's'; break; } finally { } } }"));
r.push(eval("var w = 0; while (w++ < 2) { B: { try { 3492; } catch (e) { } try { } finally { break; } } }"));
r.push(eval("var w = 0; L: while (w++ < 2) { B: { 1; try { break L; } finally { } } }"));
r.push(eval("var w = 0; while (w++ < 2) { 1; B: { break; } }"));
r.push(eval("B: { 1; break B; }"));
r.push(eval("B: { 1; { 2; break B; } }"));
r.push(eval("A: B: { 1; break A; }"));
r.push(eval("var w = 0; O: while (w++ < 2) { I: for (;;) { 3; continue O; } }"));
r.push(eval("var w = 0; O: while (w++ < 2) { 4; I: for (;;) { if (w) continue O; } }"));
return r.map(String).join('|');})()
