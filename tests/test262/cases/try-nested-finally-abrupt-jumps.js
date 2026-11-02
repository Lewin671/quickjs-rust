// Derived from: test/language/statements/try/S12.14_A10_T1.js
/*---
description: Continue and labeled break traverse every enclosing finally
---*/
var log = '';
for (var i = 0; i < 2; i++) {
  try { try { continue; } finally { log += 'a' + i; } }
  finally { log += 'b' + i; }
}
if (log !== 'a0b0a1b1') throw 'nested continue: ' + log;
log = '';
outer: while (true) {
  try { try { try { break outer; } finally { log += 'a'; } }
    finally { log += 'b'; } }
  finally { log += 'c'; }
}
if (log !== 'abc') throw 'nested labeled break: ' + log;
