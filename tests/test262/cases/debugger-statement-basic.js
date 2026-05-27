// Derived from: test/language/statements/debugger/statement.js
while (false) debugger;

debugger;
var value = 1;
if (value !== 1) { throw; }
