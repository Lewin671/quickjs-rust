// Derived from: test/built-ins/Array/prototype/includes/using-fromindex.js
var array = [1, 2, 3, 1];
if (!array.includes(1, 1)) { throw; }
if (array.includes(2, 2)) { throw; }
if (!array.includes(3, -2)) { throw; }
if (array.includes(1, 5)) { throw; }
