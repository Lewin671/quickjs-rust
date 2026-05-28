// Derived from: test/built-ins/Object/freeze/15.2.3.9-2-d-2.js
var array = [1];
Object.freeze(array);
array[0] = 2;
if (Object.isFrozen(array) !== true) throw new Error("array should be frozen");
if (array[0] !== 1) throw new Error("frozen array index should not update");
