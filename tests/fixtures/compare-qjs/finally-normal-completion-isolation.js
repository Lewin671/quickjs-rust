(function(){var r=[];
r.push(eval("var d = 0; do { try { try { continue; } catch (e) { } } finally { { 's'; } } } while (++d < 2);"));
r.push(eval("var d = 0; while (d++ < 2) { try { try { continue; } catch (e) { } } finally { with ({}) { 's'; } } }"));
r.push(eval("for (var k of [1]) { try { try { break; } finally { } } finally { { 's'; } } }"));
r.push(eval("var d = 0; do { try { continue; } finally { { 's'; } } } while (++d < 2);"));
r.push(eval("1; try { 2; } finally { 3; }"));
r.push(eval("1; try { } finally { 3; }"));
r.push(eval("L: { try { 2; } finally { 3; break L; } }"));
r.push(eval("L: { try { 2; } finally { break L; } }"));
r.push(eval("for (;;) { try { 2; break; } finally { 3; if (true) { 4; } } }"));
r.push(eval("var d = 0; while (d++ < 2) { try { 2; } finally { 3; continue; } }"));
return r.map(String).join('|');})()
