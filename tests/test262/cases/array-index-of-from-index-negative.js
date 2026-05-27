// Derived from: test/built-ins/Array/prototype/indexOf/15.4.4.14-8-1.js
var array = [1, 2, 1, 3];
if (array.indexOf(1, -1) !== -1) { throw; }
if (array.indexOf(1, -2) !== 2) { throw; }
if (array.indexOf(1, -10) !== 0) { throw; }
