// Derived from: test/built-ins/RegExp/prototype/test/y-set-lastindex.js
var r = /abc/y;

if (r.test("abc") !== true) { throw; }
if (r.lastIndex !== 3) { throw; }
