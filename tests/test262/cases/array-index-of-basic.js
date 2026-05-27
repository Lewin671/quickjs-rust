// Derived from: test/built-ins/Array/prototype/indexOf/15.4.4.14-9-1.js
var array = [false, "false", 0, "0", null, undefined];
if (array.indexOf(false) !== 0) { throw; }
if (array.indexOf("false") !== 1) { throw; }
if (array.indexOf(undefined) !== 5) { throw; }
