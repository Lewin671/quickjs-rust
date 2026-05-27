// Derived from: test/built-ins/Array/prototype/lastIndexOf/15.4.4.15-8-1.js
var array = [false, "false", 0, "0", null, undefined, false];
if (array.lastIndexOf(false) !== 6) { throw; }
if (array.lastIndexOf("false") !== 1) { throw; }
if (array.lastIndexOf(undefined) !== 5) { throw; }
