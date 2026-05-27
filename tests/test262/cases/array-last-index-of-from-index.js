// Derived from: test/built-ins/Array/prototype/lastIndexOf/15.4.4.15-4-1.js
var array = [1, 2, 1, 3];
if (array.lastIndexOf(1, 1) !== 0) { throw; }
if (array.lastIndexOf(1, 3) !== 2) { throw; }
if (array.lastIndexOf(1, 10) !== 2) { throw; }
