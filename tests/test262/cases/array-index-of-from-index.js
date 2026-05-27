// Derived from: test/built-ins/Array/prototype/indexOf/15.4.4.14-4-1.js
var array = [1, 2, 1, 3];
if (array.indexOf(1, 1) !== 2) { throw; }
if (array.indexOf(1, 3) !== -1) { throw; }
if (array.indexOf(1, 10) !== -1) { throw; }
