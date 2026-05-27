// Derived from: test/language/expressions/delete/11.4.1-3-3.js
var o = {};
if (delete o.x !== true) { throw; }
