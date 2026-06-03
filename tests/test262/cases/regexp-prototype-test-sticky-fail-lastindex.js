// Derived from: test/built-ins/RegExp/prototype/test/y-fail-lastindex.js
var r = /c/y;
r.lastIndex = 1;

if (r.test("abc") !== false) { throw; }
if (r.lastIndex !== 0) { throw; }
